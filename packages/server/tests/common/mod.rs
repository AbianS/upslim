#![allow(dead_code)]
use chrono::Utc;
use std::collections::HashMap;
use upslim_server::types::{
    AlertState, CheckKind, CheckResult, HttpConfig, Monitor, MonitorAlertRef, TcpConfig,
};

/// Creates a minimal HTTP monitor for tests.
pub fn http_monitor(name: &str, url: &str, conditions: Vec<&str>) -> Monitor {
    Monitor {
        name: name.to_owned(),
        kind: CheckKind::Http,
        http: Some(HttpConfig {
            url: url.to_owned(),
            method: "GET".to_owned(),
            headers: HashMap::new(),
            body: None,
        }),
        tcp: None,
        interval: std::time::Duration::from_secs(60),
        timeout: std::time::Duration::from_secs(10),
        failure_threshold: 3,
        success_threshold: 2,
        send_on_resolved: true,
        conditions: conditions.into_iter().map(|s| s.to_owned()).collect(),
        alerts: vec![MonitorAlertRef {
            name: "slack-ops".to_owned(),
            failure_threshold: None,
            success_threshold: None,
            send_on_resolved: None,
        }],
    }
}

/// Creates a minimal TCP monitor for tests.
pub fn tcp_monitor(name: &str, host: &str, port: u16) -> Monitor {
    Monitor {
        name: name.to_owned(),
        kind: CheckKind::Tcp,
        http: None,
        tcp: Some(TcpConfig {
            host: host.to_owned(),
            port,
        }),
        interval: std::time::Duration::from_secs(60),
        timeout: std::time::Duration::from_secs(5),
        failure_threshold: 3,
        success_threshold: 2,
        send_on_resolved: true,
        conditions: vec!["[CONNECTED] == true".to_owned()],
        alerts: vec![],
    }
}

pub fn ok_result(monitor_name: &str) -> CheckResult {
    CheckResult {
        monitor_name: monitor_name.to_owned(),
        timestamp: Utc::now(),
        success: true,
        response_time_ms: 50,
        status_code: Some(200),
        failure_reason: None,
    }
}

pub fn fail_result(monitor_name: &str) -> CheckResult {
    CheckResult {
        monitor_name: monitor_name.to_owned(),
        timestamp: Utc::now(),
        success: false,
        response_time_ms: 5000,
        status_code: Some(503),
        failure_reason: Some("[STATUS] expected 200, got 503".to_owned()),
    }
}

/// AlertState with N consecutive failures and is_firing = (n >= threshold)
pub fn state_with_failures(n: u32, threshold: u32) -> AlertState {
    AlertState {
        consecutive_failures: n,
        consecutive_successes: 0,
        is_firing: n >= threshold,
        last_sent_at: if n >= threshold {
            Some(Utc::now())
        } else {
            None
        },
    }
}
