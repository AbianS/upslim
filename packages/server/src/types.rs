use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

// ---------------------------------------------------------------------------
// Duration deserialization helpers
// ---------------------------------------------------------------------------

/// Parses duration strings: "30s", "5m", "1h", "200ms"
pub fn parse_duration(s: &str) -> std::result::Result<Duration, String> {
    let s = s.trim();
    if let Some(n) = s.strip_suffix("ms") {
        return n
            .trim()
            .parse::<u64>()
            .map(Duration::from_millis)
            .map_err(|e| e.to_string());
    }
    if let Some(n) = s.strip_suffix('h') {
        return n
            .trim()
            .parse::<u64>()
            .map(|n| Duration::from_secs(n * 3600))
            .map_err(|e| e.to_string());
    }
    if let Some(n) = s.strip_suffix('m') {
        return n
            .trim()
            .parse::<u64>()
            .map(|n| Duration::from_secs(n * 60))
            .map_err(|e| e.to_string());
    }
    if let Some(n) = s.strip_suffix('s') {
        return n
            .trim()
            .parse::<u64>()
            .map(Duration::from_secs)
            .map_err(|e| e.to_string());
    }
    s.parse::<u64>()
        .map(Duration::from_secs)
        .map_err(|_| format!("invalid duration '{s}': use '30s', '5m', '1h'"))
}

// ---------------------------------------------------------------------------
// Check kind
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckKind {
    Http,
    Tcp,
}

// ---------------------------------------------------------------------------
// Monitor — processed structure with defaults already applied
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Monitor {
    pub name: String,
    pub kind: CheckKind,
    /// HTTP-specific
    pub http: Option<HttpConfig>,
    /// TCP-specific
    pub tcp: Option<TcpConfig>,
    pub interval: Duration,
    pub timeout: Duration,
    pub failure_threshold: u32,
    pub success_threshold: u32,
    pub send_on_resolved: bool,
    pub conditions: Vec<String>,
    pub alerts: Vec<MonitorAlertRef>,
}

impl Monitor {
    /// Monitor URL to include in notifications (HTTP only)
    pub fn url(&self) -> Option<String> {
        self.http.as_ref().map(|h| h.url.clone())
    }
}

#[derive(Debug, Clone)]
pub struct HttpConfig {
    pub url: String,
    pub method: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TcpConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone)]
pub struct MonitorAlertRef {
    pub name: String,
    /// Optional per-monitor overrides
    pub failure_threshold: Option<u32>,
    pub success_threshold: Option<u32>,
    pub send_on_resolved: Option<bool>,
}

// ---------------------------------------------------------------------------
// CheckResult
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    pub monitor_name: String,
    pub timestamp: DateTime<Utc>,
    pub success: bool,
    pub response_time_ms: u64,
    pub status_code: Option<u16>,
    /// Detail of which condition failed (if any)
    pub failure_reason: Option<String>,
}

impl CheckResult {
    /// Constructor for transport-level errors (timeout, DNS, etc.)
    pub fn transport_error(monitor_name: &str, err: impl std::fmt::Display) -> Self {
        Self {
            monitor_name: monitor_name.to_owned(),
            timestamp: Utc::now(),
            success: false,
            response_time_ms: 0,
            status_code: None,
            failure_reason: Some(err.to_string()),
        }
    }
}

// ---------------------------------------------------------------------------
// AlertState — persisted per (monitor_name × alert_provider_name)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AlertState {
    pub consecutive_failures: u32,
    pub consecutive_successes: u32,
    /// The alert is currently "firing" (sent, not yet resolved)
    pub is_firing: bool,
    /// Last time a notification was sent (for reminder intervals)
    pub last_sent_at: Option<DateTime<Utc>>,
}

// ---------------------------------------------------------------------------
// AlertMessage — sent to the AlertProvider
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct AlertMessage {
    pub monitor_name: String,
    pub monitor_url: Option<String>,
    pub result: CheckResult,
    pub is_resolved: bool,
    pub is_reminder: bool,
}

// ---------------------------------------------------------------------------
// AlertAction — output of advance_state
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AlertAction {
    Fire,
    Resolve,
    Reminder,
}

// ---------------------------------------------------------------------------
// AlertProviderConfig — configuration for a provider
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct SlackProviderConfig {
    pub name: String,
    /// Bot token — starts with xoxb-
    pub token: String,
    /// Destination channel: ID ("C1234567890") or name ("#ops-alerts")
    pub channel: String,
    pub reminder_interval: Option<Duration>,
}

#[derive(Debug, Clone)]
pub enum AlertProviderConfig {
    Slack(SlackProviderConfig),
}

impl AlertProviderConfig {
    pub fn name(&self) -> &str {
        match self {
            AlertProviderConfig::Slack(c) => &c.name,
        }
    }
}
