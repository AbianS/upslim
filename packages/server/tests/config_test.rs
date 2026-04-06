mod common;

use std::io::Write;
use tempfile::NamedTempFile;
use upslim_server::{config, types::CheckKind};

fn write_config(content: &str) -> NamedTempFile {
    let mut f = NamedTempFile::with_suffix(".yaml").unwrap();
    f.write_all(content.as_bytes()).unwrap();
    f
}

#[test]
fn load_minimal_http_monitor() {
    let f = write_config(
        r#"
monitors:
  - name: api-health
    type: http
    url: "https://example.com"
    conditions:
      - "[STATUS] == 200"
"#,
    );
    let cfg = config::load(f.path()).expect("config load failed");
    assert_eq!(cfg.monitors.len(), 1);
    let m = &cfg.monitors[0];
    assert_eq!(m.name, "api-health");
    assert_eq!(m.kind, CheckKind::Http);
    assert_eq!(m.http.as_ref().unwrap().url, "https://example.com");
    assert_eq!(m.http.as_ref().unwrap().method, "GET");
    // Defaults aplicados
    assert_eq!(m.interval.as_secs(), 60);
    assert_eq!(m.timeout.as_secs(), 30);
    assert_eq!(m.failure_threshold, 3);
    assert_eq!(m.success_threshold, 2);
    assert!(m.send_on_resolved);
}

#[test]
fn load_tcp_monitor() {
    let f = write_config(
        r#"
monitors:
  - name: db-check
    type: tcp
    host: "db.internal"
    port: 5432
    conditions:
      - "[CONNECTED] == true"
"#,
    );
    let cfg = config::load(f.path()).expect("config load failed");
    let m = &cfg.monitors[0];
    assert_eq!(m.kind, CheckKind::Tcp);
    assert_eq!(m.tcp.as_ref().unwrap().host, "db.internal");
    assert_eq!(m.tcp.as_ref().unwrap().port, 5432);
}

#[test]
fn custom_defaults_applied() {
    let f = write_config(
        r#"
defaults:
  interval: 30s
  timeout: 5s
  failure_threshold: 2
  success_threshold: 1

monitors:
  - name: test
    type: http
    url: "https://example.com"
    conditions:
      - "[STATUS] == 200"
"#,
    );
    let cfg = config::load(f.path()).unwrap();
    let m = &cfg.monitors[0];
    assert_eq!(m.interval.as_secs(), 30);
    assert_eq!(m.timeout.as_secs(), 5);
    assert_eq!(m.failure_threshold, 2);
    assert_eq!(m.success_threshold, 1);
}

#[test]
fn per_monitor_overrides_default() {
    let f = write_config(
        r#"
defaults:
  interval: 60s
  failure_threshold: 5

monitors:
  - name: fast-check
    type: http
    url: "https://example.com"
    interval: 10s
    failure_threshold: 1
    conditions:
      - "[STATUS] == 200"
"#,
    );
    let cfg = config::load(f.path()).unwrap();
    let m = &cfg.monitors[0];
    assert_eq!(m.interval.as_secs(), 10);
    assert_eq!(m.failure_threshold, 1);
}

#[test]
fn error_on_missing_url_for_http() {
    let f = write_config(
        r#"
monitors:
  - name: broken
    type: http
    conditions:
      - "[STATUS] == 200"
"#,
    );
    assert!(config::load(f.path()).is_err());
}

#[test]
fn error_on_empty_conditions() {
    let f = write_config(
        r#"
monitors:
  - name: broken
    type: http
    url: "https://example.com"
    conditions: []
"#,
    );
    assert!(config::load(f.path()).is_err());
}

#[test]
fn error_on_no_monitors() {
    let f = write_config(
        r#"
monitors: []
"#,
    );
    assert!(config::load(f.path()).is_err());
}

#[test]
fn multiple_monitors() {
    let f = write_config(
        r#"
monitors:
  - name: api
    type: http
    url: "https://api.example.com"
    conditions:
      - "[STATUS] == 200"
  - name: db
    type: tcp
    host: "db.internal"
    port: 5432
    conditions:
      - "[CONNECTED] == true"
"#,
    );
    let cfg = config::load(f.path()).unwrap();
    assert_eq!(cfg.monitors.len(), 2);
    assert_eq!(cfg.monitors[0].name, "api");
    assert_eq!(cfg.monitors[1].name, "db");
}

#[test]
fn slack_provider_loaded() {
    let f = write_config(
        r##"
alerting:
  - name: slack-ops
    type: slack
    token: "xoxb-test-token"
    channel: "#ops-alerts"

monitors:
  - name: api
    type: http
    url: "https://example.com"
    conditions:
      - "[STATUS] == 200"
    alerts:
      - name: slack-ops
"##,
    );
    let cfg = config::load(f.path()).unwrap();
    assert_eq!(cfg.alert_providers.len(), 1);
    assert_eq!(cfg.monitors[0].alerts.len(), 1);
    assert_eq!(cfg.monitors[0].alerts[0].name, "slack-ops");
    // Verificar que token y channel se cargaron
    match &cfg.alert_providers[0] {
        upslim_server::types::AlertProviderConfig::Slack(s) => {
            assert_eq!(s.token, "xoxb-test-token");
            assert_eq!(s.channel, "#ops-alerts");
        }
    }
}

#[test]
fn load_from_directory() {
    let dir = tempfile::tempdir().unwrap();
    let f1 = dir.path().join("01-monitors.yaml");
    let f2 = dir.path().join("02-more.yaml");

    std::fs::write(
        &f1,
        r#"
monitors:
  - name: api1
    type: http
    url: "https://example.com"
    conditions:
      - "[STATUS] == 200"
"#,
    )
    .unwrap();

    std::fs::write(
        &f2,
        r#"
monitors:
  - name: api2
    type: http
    url: "https://example.org"
    conditions:
      - "[STATUS] == 200"
"#,
    )
    .unwrap();

    let cfg = config::load(dir.path()).unwrap();
    assert_eq!(cfg.monitors.len(), 2);
}

#[serial_test::serial]
#[test]
fn env_var_substitution() {
    // SAFETY: serial_test garantiza que no hay otros threads tocando env vars en paralelo
    unsafe { std::env::set_var("TEST_SLACK_TOKEN", "xoxb-from-env-1234") };
    let f = write_config(
        r##"
alerting:
  - name: slack-ops
    type: slack
    token: "${TEST_SLACK_TOKEN}"
    channel: "#ops-alerts"

monitors:
  - name: api
    type: http
    url: "https://example.com"
    conditions:
      - "[STATUS] == 200"
"##,
    );
    let cfg = config::load(f.path()).unwrap();
    match &cfg.alert_providers[0] {
        upslim_server::types::AlertProviderConfig::Slack(s) => {
            assert_eq!(s.token, "xoxb-from-env-1234");
        }
    }
    unsafe { std::env::remove_var("TEST_SLACK_TOKEN") };
}
