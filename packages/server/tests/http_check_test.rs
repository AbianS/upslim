mod common;

use upslim_server::checker::{Checker, http::HttpChecker};
use wiremock::{Mock, MockServer, ResponseTemplate, matchers};

#[tokio::test]
async fn http_200_passes_status_condition() {
    let server = MockServer::start().await;
    Mock::given(matchers::method("GET"))
        .and(matchers::path("/health"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let monitor = common::http_monitor(
        "test",
        &format!("{}/health", server.uri()),
        vec!["[STATUS] == 200"],
    );

    let checker = HttpChecker::new();
    let result = checker.check(&monitor).await.expect("check failed");

    assert!(result.success, "failure: {:?}", result.failure_reason);
    assert_eq!(result.status_code, Some(200));
}

#[tokio::test]
async fn http_503_fails_status_condition() {
    let server = MockServer::start().await;
    Mock::given(matchers::method("GET"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&server)
        .await;

    let monitor = common::http_monitor("test", &server.uri(), vec!["[STATUS] == 200"]);

    let checker = HttpChecker::new();
    let result = checker.check(&monitor).await.expect("check failed");

    assert!(!result.success);
    assert!(result.failure_reason.is_some());
}

#[tokio::test]
async fn http_response_time_condition() {
    let server = MockServer::start().await;
    Mock::given(matchers::method("GET"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    // Response time should be < 5000ms in any reasonable environment
    let monitor = common::http_monitor(
        "test",
        &server.uri(),
        vec!["[STATUS] == 200", "[RESPONSE_TIME] < 5000"],
    );

    let checker = HttpChecker::new();
    let result = checker.check(&monitor).await.unwrap();

    assert!(result.success, "failure: {:?}", result.failure_reason);
    assert!(result.response_time_ms < 5000);
}

#[tokio::test]
async fn http_body_json_condition() {
    let server = MockServer::start().await;
    Mock::given(matchers::method("GET"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"status": "healthy", "version": "1.0.0"})),
        )
        .mount(&server)
        .await;

    let monitor = common::http_monitor(
        "test",
        &server.uri(),
        vec!["[STATUS] == 200", "[BODY].status == healthy"],
    );

    let checker = HttpChecker::new();
    let result = checker.check(&monitor).await.unwrap();

    assert!(result.success, "failure: {:?}", result.failure_reason);
}

#[tokio::test]
async fn http_body_json_condition_fails_on_wrong_value() {
    let server = MockServer::start().await;
    Mock::given(matchers::method("GET"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({"status": "degraded"})),
        )
        .mount(&server)
        .await;

    let monitor = common::http_monitor(
        "test",
        &server.uri(),
        vec!["[STATUS] == 200", "[BODY].status == healthy"],
    );

    let checker = HttpChecker::new();
    let result = checker.check(&monitor).await.unwrap();

    assert!(!result.success);
    assert!(
        result
            .failure_reason
            .as_deref()
            .unwrap_or("")
            .contains("degraded")
    );
}

#[tokio::test]
async fn http_timeout_results_in_failure() {
    let server = MockServer::start().await;
    Mock::given(matchers::method("GET"))
        .respond_with(
            ResponseTemplate::new(200)
                // Delay longer than the monitor timeout
                .set_delay(std::time::Duration::from_secs(3)),
        )
        .mount(&server)
        .await;

    let mut monitor = common::http_monitor("test", &server.uri(), vec!["[STATUS] == 200"]);
    monitor.timeout = std::time::Duration::from_millis(200);

    let checker = HttpChecker::new();
    let result = checker.check(&monitor).await;

    // May be Ok(failure) or Err — either way the check should not pass
    match result {
        Ok(r) => assert!(!r.success, "expected timeout failure"),
        Err(_) => {} // Transport error is also valid
    }
}

#[tokio::test]
async fn http_post_with_body() {
    let server = MockServer::start().await;
    Mock::given(matchers::method("POST"))
        .and(matchers::path("/submit"))
        .respond_with(ResponseTemplate::new(201))
        .mount(&server)
        .await;

    let mut monitor = common::http_monitor(
        "test",
        &format!("{}/submit", server.uri()),
        vec!["[STATUS] == 201"],
    );
    monitor.http.as_mut().unwrap().method = "POST".to_owned();
    monitor.http.as_mut().unwrap().body = Some(r#"{"key": "value"}"#.to_owned());

    let checker = HttpChecker::new();
    let result = checker.check(&monitor).await.unwrap();

    assert!(result.success, "failure: {:?}", result.failure_reason);
}

#[tokio::test]
async fn http_custom_headers_sent() {
    let server = MockServer::start().await;
    Mock::given(matchers::method("GET"))
        .and(matchers::header("X-Api-Key", "secret"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let mut monitor = common::http_monitor("test", &server.uri(), vec!["[STATUS] == 200"]);
    monitor
        .http
        .as_mut()
        .unwrap()
        .headers
        .insert("X-Api-Key".to_owned(), "secret".to_owned());

    let checker = HttpChecker::new();
    let result = checker.check(&monitor).await.unwrap();

    assert!(result.success, "failure: {:?}", result.failure_reason);
}
