use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use std::time::Duration;
use tracing::debug;

use crate::{
    alert::AlertProvider,
    error::{Result, UpslimError},
    types::{AlertMessage, SlackProviderConfig},
};

const CHAT_POST_MESSAGE: &str = "https://slack.com/api/chat.postMessage";

pub struct SlackProvider {
    name: String,
    token: String,
    channel: String,
    client: reqwest::Client,
    reminder_interval: Option<Duration>,
}

impl SlackProvider {
    pub fn new(config: &SlackProviderConfig) -> Self {
        let client = reqwest::Client::builder()
            .use_rustls_tls()
            .build()
            .expect("Failed to build Slack HTTP client");

        Self {
            name: config.name.clone(),
            token: config.token.clone(),
            channel: config.channel.clone(),
            client,
            reminder_interval: config.reminder_interval,
        }
    }
}

// Slack always returns 200 OK — the error is in the `ok` field of the JSON.
#[derive(Deserialize)]
struct SlackResponse {
    ok: bool,
    error: Option<String>,
}

#[async_trait]
impl AlertProvider for SlackProvider {
    fn name(&self) -> &str {
        &self.name
    }

    fn validate(&self) -> Result<()> {
        if self.token.is_empty() {
            return Err(UpslimError::Config(format!(
                "Slack provider '{}': token is empty",
                self.name
            )));
        }
        if !self.token.starts_with("xoxb-") {
            return Err(UpslimError::Config(format!(
                "Slack provider '{}': token must start with 'xoxb-' (Bot token)",
                self.name
            )));
        }
        if self.channel.is_empty() {
            return Err(UpslimError::Config(format!(
                "Slack provider '{}': channel is empty",
                self.name
            )));
        }
        Ok(())
    }

    fn reminder_interval(&self) -> Option<Duration> {
        self.reminder_interval
    }

    async fn send(&self, msg: &AlertMessage) -> Result<()> {
        let payload = build_payload(&self.channel, msg);

        debug!(
            provider = %self.name,
            monitor = %msg.monitor_name,
            channel = %self.channel,
            resolved = msg.is_resolved,
            reminder = msg.is_reminder,
            "sending Slack notification"
        );

        let resp = self
            .client
            .post(CHAT_POST_MESSAGE)
            .header(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", self.token),
            )
            .json(&payload)
            .send()
            .await
            .map_err(|e| UpslimError::Alert(format!("Slack request failed: {e}")))?;

        // Slack returns 200 even on errors — must read the JSON
        let slack_resp: SlackResponse = resp
            .json()
            .await
            .map_err(|e| UpslimError::Alert(format!("Failed to parse Slack response: {e}")))?;

        if !slack_resp.ok {
            let error = slack_resp.error.as_deref().unwrap_or("unknown_error");
            return Err(UpslimError::Alert(format!(
                "Slack API error: {error} (channel: {})",
                self.channel
            )));
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Block Kit payload builder
// ---------------------------------------------------------------------------

fn build_payload(channel: &str, msg: &AlertMessage) -> Value {
    let (emoji, header_text, color) = if msg.is_resolved {
        (
            "🟢",
            format!("{}  is RECOVERED", msg.monitor_name),
            "#36A64F",
        )
    } else if msg.is_reminder {
        (
            "⏰",
            format!("{}  is STILL DOWN", msg.monitor_name),
            "#FF8C00",
        )
    } else {
        ("🔴", format!("{}  is DOWN", msg.monitor_name), "#DD0000")
    };

    let header = format!("{emoji}  {header_text}");

    let mut fields = vec![
        json!({
            "type": "mrkdwn",
            "text": format!("*Monitor:*\n{}", msg.monitor_name)
        }),
        json!({
            "type": "mrkdwn",
            "text": format!("*Time:*\n{}", msg.result.timestamp.format("%Y-%m-%dT%H:%M:%SZ"))
        }),
        json!({
            "type": "mrkdwn",
            "text": format!("*Response Time:*\n{}ms", msg.result.response_time_ms)
        }),
    ];

    if let Some(url) = &msg.monitor_url {
        fields.push(json!({
            "type": "mrkdwn",
            "text": format!("*URL:*\n{url}")
        }));
    }

    if let Some(reason) = &msg.result.failure_reason {
        if !msg.is_resolved {
            fields.push(json!({
                "type": "mrkdwn",
                "text": format!("*Reason:*\n{reason}")
            }));
        }
    }

    json!({
        "channel": channel,
        "attachments": [{
            "color": color,
            // fallback: plain text for push notifications / clients without block support
            "fallback": header_text,
            "blocks": [
                {
                    "type": "header",
                    "text": {
                        "type": "plain_text",
                        "text": header
                    }
                },
                {
                    "type": "section",
                    "fields": fields
                }
            ]
        }]
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AlertMessage, CheckResult};
    use chrono::Utc;

    fn make_provider(token: &str, channel: &str) -> SlackProvider {
        SlackProvider::new(&SlackProviderConfig {
            name: "test".to_owned(),
            token: token.to_owned(),
            channel: channel.to_owned(),
            reminder_interval: None,
        })
    }

    fn make_msg(is_resolved: bool, is_reminder: bool, success: bool) -> AlertMessage {
        AlertMessage {
            monitor_name: "api-health".to_owned(),
            monitor_url: Some("https://api.example.com/health".to_owned()),
            result: CheckResult {
                monitor_name: "api-health".to_owned(),
                timestamp: Utc::now(),
                success,
                response_time_ms: 123,
                status_code: if success { Some(200) } else { Some(503) },
                failure_reason: if success {
                    None
                } else {
                    Some("[STATUS] expected 200, got 503".to_owned())
                },
            },
            is_resolved,
            is_reminder,
        }
    }

    #[test]
    fn payload_includes_channel() {
        let msg = make_msg(false, false, false);
        let payload = build_payload("#ops-alerts", &msg);
        assert_eq!(payload["channel"], "#ops-alerts");
    }

    #[test]
    fn payload_has_fallback_text() {
        let msg = make_msg(false, false, false);
        let payload = build_payload("#ops-alerts", &msg);
        // fallback inside the attachment — no top-level text
        assert!(payload["text"].is_null());
        assert!(
            !payload["attachments"][0]["fallback"]
                .as_str()
                .unwrap_or("")
                .is_empty()
        );
    }

    #[test]
    fn fire_payload_has_red_color() {
        let msg = make_msg(false, false, false);
        let payload = build_payload("C123", &msg);
        assert_eq!(payload["attachments"][0]["color"], "#DD0000");
    }

    #[test]
    fn resolve_payload_has_green_color() {
        let msg = make_msg(true, false, true);
        let payload = build_payload("C123", &msg);
        assert_eq!(payload["attachments"][0]["color"], "#36A64F");
    }

    #[test]
    fn reminder_payload_has_orange_color() {
        let msg = make_msg(false, true, false);
        let payload = build_payload("C123", &msg);
        assert_eq!(payload["attachments"][0]["color"], "#FF8C00");
    }

    #[test]
    fn fire_header_contains_down() {
        let msg = make_msg(false, false, false);
        let payload = build_payload("C123", &msg);
        let header = payload["attachments"][0]["blocks"][0]["text"]["text"]
            .as_str()
            .unwrap();
        assert!(header.contains("DOWN"), "header: {header}");
    }

    #[test]
    fn resolve_header_contains_recovered() {
        let msg = make_msg(true, false, true);
        let payload = build_payload("C123", &msg);
        let header = payload["attachments"][0]["blocks"][0]["text"]["text"]
            .as_str()
            .unwrap();
        assert!(header.contains("RECOVERED"), "header: {header}");
    }

    #[test]
    fn validate_passes_with_valid_config() {
        let p = make_provider("xoxb-1234-5678-abcd", "#ops-alerts");
        assert!(p.validate().is_ok());
    }

    #[test]
    fn validate_fails_with_empty_token() {
        let p = make_provider("", "#ops-alerts");
        assert!(p.validate().is_err());
    }

    #[test]
    fn validate_fails_with_non_bot_token() {
        let p = make_provider("xoxp-1234-5678-abcd", "#ops-alerts");
        let err = p.validate().unwrap_err().to_string();
        assert!(err.contains("xoxb-"), "err: {err}");
    }

    #[test]
    fn validate_fails_with_empty_channel() {
        let p = make_provider("xoxb-1234-5678-abcd", "");
        assert!(p.validate().is_err());
    }
}
