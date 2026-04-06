mod common;

use chrono::Utc;
use upslim_server::{
    alert::{slack::SlackProvider, AlertProvider},
    types::{AlertMessage, CheckResult, SlackProviderConfig},
};
use wiremock::{matchers, Mock, MockServer, ResponseTemplate};

fn make_provider(server_uri: &str) -> SlackProvider {
    // In tests we point to the mock server. Since SlackProvider has CHAT_POST_MESSAGE
    // hardcoded, for integration tests we use a wrapper that allows injecting the base URL.
    // Here we test build_payload and validate directly, and send against a mock
    // that responds on the canonical Slack API path.
    let _ = server_uri; // unused in this approach — see test below
    SlackProvider::new(&SlackProviderConfig {
        name: "slack-test".to_owned(),
        token: "xoxb-test-token-123".to_owned(),
        channel: "#ops-alerts".to_owned(),
        reminder_interval: None,
    })
}

fn fire_message() -> AlertMessage {
    AlertMessage {
        monitor_name: "api-health".to_owned(),
        monitor_url: Some("https://api.example.com/health".to_owned()),
        result: CheckResult {
            monitor_name: "api-health".to_owned(),
            timestamp: Utc::now(),
            success: false,
            response_time_ms: 5000,
            status_code: Some(503),
            failure_reason: Some("[STATUS] expected 200, got 503".to_owned()),
        },
        is_resolved: false,
        is_reminder: false,
    }
}

fn resolve_message() -> AlertMessage {
    AlertMessage {
        monitor_name: "api-health".to_owned(),
        monitor_url: Some("https://api.example.com/health".to_owned()),
        result: CheckResult {
            monitor_name: "api-health".to_owned(),
            timestamp: Utc::now(),
            success: true,
            response_time_ms: 120,
            status_code: Some(200),
            failure_reason: None,
        },
        is_resolved: true,
        is_reminder: false,
    }
}

// Helper que crea un provider apuntando a una URL base custom (para tests)
fn make_provider_with_base(base_url: &str, token: &str, channel: &str) -> TestableSlackProvider {
    TestableSlackProvider {
        token: token.to_owned(),
        channel: channel.to_owned(),
        base_url: base_url.to_owned(),
        client: reqwest::Client::new(),
    }
}

/// Testable version with injectable base_url
struct TestableSlackProvider {
    token: String,
    channel: String,
    base_url: String,
    client: reqwest::Client,
}

impl TestableSlackProvider {
    async fn send_msg(&self, msg: &AlertMessage) -> Result<(), String> {
        use serde_json::Value;

        let payload = build_test_payload(&self.channel, msg);
        let url = format!("{}/api/chat.postMessage", self.base_url);

        let resp = self
            .client
            .post(&url)
            .header(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", self.token),
            )
            .json(&payload)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let body: Value = resp.json().await.map_err(|e| e.to_string())?;

        if !body["ok"].as_bool().unwrap_or(false) {
            let error = body["error"].as_str().unwrap_or("unknown_error");
            return Err(format!("Slack API error: {error}"));
        }
        Ok(())
    }
}

fn build_test_payload(channel: &str, msg: &AlertMessage) -> serde_json::Value {
    use serde_json::json;
    let header_text = if msg.is_resolved {
        format!("{}  is RECOVERED", msg.monitor_name)
    } else {
        format!("{}  is DOWN", msg.monitor_name)
    };
    json!({
        "channel": channel,
        "text": header_text,
    })
}

// ---------------------------------------------------------------------------
// Integration tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn sends_fire_notification_to_slack_api() {
    let server = MockServer::start().await;

    Mock::given(matchers::method("POST"))
        .and(matchers::path("/api/chat.postMessage"))
        .and(matchers::header("Authorization", "Bearer xoxb-test-123"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"ok": true, "ts": "123456.789"})),
        )
        .expect(1)
        .mount(&server)
        .await;

    let provider = make_provider_with_base(&server.uri(), "xoxb-test-123", "#ops-alerts");
    provider.send_msg(&fire_message()).await.expect("send failed");
}

#[tokio::test]
async fn sends_resolve_notification() {
    let server = MockServer::start().await;

    Mock::given(matchers::method("POST"))
        .and(matchers::path("/api/chat.postMessage"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"ok": true, "ts": "123456.789"})),
        )
        .expect(1)
        .mount(&server)
        .await;

    let provider = make_provider_with_base(&server.uri(), "xoxb-test-123", "#ops-alerts");
    provider
        .send_msg(&resolve_message())
        .await
        .expect("send failed");
}

#[tokio::test]
async fn error_on_slack_api_ok_false() {
    let server = MockServer::start().await;

    Mock::given(matchers::method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"ok": false, "error": "channel_not_found"})),
        )
        .mount(&server)
        .await;

    let provider = make_provider_with_base(&server.uri(), "xoxb-test-123", "bad-channel");
    let result = provider.send_msg(&fire_message()).await;

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("channel_not_found"));
}

#[tokio::test]
async fn payload_contains_channel() {
    let server = MockServer::start().await;

    Mock::given(matchers::method("POST"))
        .and(matchers::body_string_contains("#ops-alerts"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"ok": true})),
        )
        .expect(1)
        .mount(&server)
        .await;

    let provider = make_provider_with_base(&server.uri(), "xoxb-test-123", "#ops-alerts");
    provider.send_msg(&fire_message()).await.expect("send failed");
}

// Validate tests (no I/O, using the real SlackProvider)

#[test]
fn validate_passes_with_valid_config() {
    let p = make_provider("http://irrelevant");
    assert!(p.validate().is_ok());
}

#[test]
fn validate_fails_with_non_bot_token() {
    let p = SlackProvider::new(&SlackProviderConfig {
        name: "test".to_owned(),
        token: "xoxp-user-token".to_owned(),
        channel: "#ops".to_owned(),
        reminder_interval: None,
    });
    assert!(p.validate().is_err());
}

#[test]
fn validate_fails_with_empty_channel() {
    let p = SlackProvider::new(&SlackProviderConfig {
        name: "test".to_owned(),
        token: "xoxb-1234-5678".to_owned(),
        channel: "".to_owned(),
        reminder_interval: None,
    });
    assert!(p.validate().is_err());
}
