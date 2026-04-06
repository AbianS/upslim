use crate::{
    error::{Result, UpslimError},
    types::{
        AlertProviderConfig, CheckKind, HttpConfig, Monitor, MonitorAlertRef, SlackProviderConfig,
        TcpConfig, parse_duration,
    },
};
use serde::Deserialize;
use std::{collections::HashMap, path::Path, time::Duration};

// ---------------------------------------------------------------------------
// Final config — result of loading and processing everything
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct Config {
    pub monitors: Vec<Monitor>,
    pub alert_providers: Vec<AlertProviderConfig>,
    /// Directory where alert state is persisted
    pub state_dir: std::path::PathBuf,
    /// Maximum number of concurrent checks
    pub max_concurrent: usize,
}

// ---------------------------------------------------------------------------
// Raw structs — what serde parses directly from YAML
// ---------------------------------------------------------------------------

#[derive(Deserialize, Default)]
struct RawDefaults {
    interval: Option<String>,
    timeout: Option<String>,
    failure_threshold: Option<u32>,
    success_threshold: Option<u32>,
    send_on_resolved: Option<bool>,
}

#[derive(Deserialize)]
struct RawAlertProvider {
    name: String,
    #[serde(rename = "type")]
    kind: String,
    // Slack-specific
    token: Option<String>,
    channel: Option<String>,
    reminder_interval: Option<String>,
}

#[derive(Deserialize)]
struct RawMonitorAlertRef {
    name: String,
    failure_threshold: Option<u32>,
    success_threshold: Option<u32>,
    send_on_resolved: Option<bool>,
}

#[derive(Deserialize)]
struct RawMonitor {
    name: String,
    #[serde(rename = "type")]
    kind: String,
    // HTTP fields
    url: Option<String>,
    method: Option<String>,
    headers: Option<HashMap<String, String>>,
    body: Option<String>,
    // TCP fields
    host: Option<String>,
    port: Option<u16>,
    // Common overrides
    interval: Option<String>,
    timeout: Option<String>,
    failure_threshold: Option<u32>,
    success_threshold: Option<u32>,
    send_on_resolved: Option<bool>,
    #[serde(default)]
    conditions: Vec<String>,
    #[serde(default)]
    alerts: Vec<RawMonitorAlertRef>,
}

#[derive(Deserialize, Default)]
struct RawConfig {
    #[serde(default)]
    defaults: RawDefaults,
    #[serde(default)]
    alerting: Vec<RawAlertProvider>,
    #[serde(default)]
    monitors: Vec<RawMonitor>,
}

// ---------------------------------------------------------------------------
// Configuration loading
// ---------------------------------------------------------------------------

/// Loads config from a single file or directory (merges all *.yaml files).
pub fn load(path: &Path) -> Result<Config> {
    // state_dir and max_concurrent from env vars (with sensible defaults)
    let state_dir = std::env::var("UPSLIM_STATE_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("./state"));

    let max_concurrent: usize = std::env::var("UPSLIM_MAX_CONCURRENT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(10);

    let raw = load_raw(path)?;
    let config = process(raw, state_dir, max_concurrent)?;
    Ok(config)
}

/// Loads and merges YAML (single file or directory).
fn load_raw(path: &Path) -> Result<RawConfig> {
    if path.is_dir() {
        load_dir(path)
    } else {
        load_file(path)
    }
}

fn load_file(path: &Path) -> Result<RawConfig> {
    let content = std::fs::read_to_string(path)?;
    let expanded = shellexpand::env(&content).map_err(|e| {
        UpslimError::Config(format!(
            "Environment variable '{}' not set (referenced in {})",
            e.var_name,
            path.display()
        ))
    })?;
    let raw: RawConfig = serde_yaml::from_str(&expanded)?;
    Ok(raw)
}

fn load_dir(dir: &Path) -> Result<RawConfig> {
    let mut entries: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map(|ext| ext == "yaml" || ext == "yml")
                .unwrap_or(false)
        })
        .collect();

    // Deterministic lexicographic order
    entries.sort_by_key(|e| e.path());

    if entries.is_empty() {
        return Err(UpslimError::Config(format!(
            "No .yaml files found in directory {}",
            dir.display()
        )));
    }

    let mut merged = RawConfig::default();
    let mut defaults_set = false;

    for entry in entries {
        let raw = load_file(&entry.path())?;
        if !defaults_set && (raw.defaults.interval.is_some() || raw.defaults.timeout.is_some()) {
            merged.defaults = raw.defaults;
            defaults_set = true;
        }
        merged.alerting.extend(raw.alerting);
        merged.monitors.extend(raw.monitors);
    }

    Ok(merged)
}

// ---------------------------------------------------------------------------
// Processing — raw → typed with defaults applied
// ---------------------------------------------------------------------------

const DEFAULT_INTERVAL: Duration = Duration::from_secs(60);
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
const DEFAULT_FAILURE_THRESHOLD: u32 = 3;
const DEFAULT_SUCCESS_THRESHOLD: u32 = 2;

fn process(raw: RawConfig, state_dir: std::path::PathBuf, max_concurrent: usize) -> Result<Config> {
    // Parse global defaults
    let default_interval = raw
        .defaults
        .interval
        .as_deref()
        .map(parse_duration)
        .transpose()
        .map_err(|e| UpslimError::Config(format!("defaults.interval: {e}")))?
        .unwrap_or(DEFAULT_INTERVAL);

    let default_timeout = raw
        .defaults
        .timeout
        .as_deref()
        .map(parse_duration)
        .transpose()
        .map_err(|e| UpslimError::Config(format!("defaults.timeout: {e}")))?
        .unwrap_or(DEFAULT_TIMEOUT);

    let default_failure_threshold = raw
        .defaults
        .failure_threshold
        .unwrap_or(DEFAULT_FAILURE_THRESHOLD);

    let default_success_threshold = raw
        .defaults
        .success_threshold
        .unwrap_or(DEFAULT_SUCCESS_THRESHOLD);

    let default_send_on_resolved = raw.defaults.send_on_resolved.unwrap_or(true);

    // Process alert providers
    let mut alert_providers = Vec::new();
    for rp in raw.alerting {
        let provider = match rp.kind.as_str() {
            "slack" => {
                let token = rp.token.ok_or_else(|| {
                    UpslimError::Config(format!(
                        "Alert provider '{}' (slack) missing token",
                        rp.name
                    ))
                })?;
                let channel = rp.channel.ok_or_else(|| {
                    UpslimError::Config(format!(
                        "Alert provider '{}' (slack) missing channel",
                        rp.name
                    ))
                })?;
                let reminder_interval = rp
                    .reminder_interval
                    .as_deref()
                    .map(parse_duration)
                    .transpose()
                    .map_err(|e| {
                        UpslimError::Config(format!("alerting[{}].reminder_interval: {e}", rp.name))
                    })?;
                AlertProviderConfig::Slack(SlackProviderConfig {
                    name: rp.name,
                    token,
                    channel,
                    reminder_interval,
                })
            }
            other => {
                return Err(UpslimError::Config(format!(
                    "Unknown alert provider type '{other}'"
                )));
            }
        };
        alert_providers.push(provider);
    }

    // Process monitors
    let mut monitors = Vec::new();
    for rm in raw.monitors {
        let interval = rm
            .interval
            .as_deref()
            .map(parse_duration)
            .transpose()
            .map_err(|e| UpslimError::Config(format!("monitor '{}' interval: {e}", rm.name)))?
            .unwrap_or(default_interval);

        let timeout = rm
            .timeout
            .as_deref()
            .map(parse_duration)
            .transpose()
            .map_err(|e| UpslimError::Config(format!("monitor '{}' timeout: {e}", rm.name)))?
            .unwrap_or(default_timeout);

        let kind = match rm.kind.as_str() {
            "http" => CheckKind::Http,
            "tcp" => CheckKind::Tcp,
            other => {
                return Err(UpslimError::Config(format!(
                    "Monitor '{}': unknown type '{other}'",
                    rm.name
                )));
            }
        };

        let http = if kind == CheckKind::Http {
            let url = rm.url.ok_or_else(|| {
                UpslimError::Config(format!("HTTP monitor '{}' missing url", rm.name))
            })?;
            Some(HttpConfig {
                url,
                method: rm.method.unwrap_or_else(|| "GET".to_owned()),
                headers: rm.headers.unwrap_or_default(),
                body: rm.body,
            })
        } else {
            None
        };

        let tcp = if kind == CheckKind::Tcp {
            let host = rm.host.ok_or_else(|| {
                UpslimError::Config(format!("TCP monitor '{}' missing host", rm.name))
            })?;
            let port = rm.port.ok_or_else(|| {
                UpslimError::Config(format!("TCP monitor '{}' missing port", rm.name))
            })?;
            Some(TcpConfig { host, port })
        } else {
            None
        };

        if rm.conditions.is_empty() {
            return Err(UpslimError::Config(format!(
                "Monitor '{}' has no conditions",
                rm.name
            )));
        }

        let alerts = rm
            .alerts
            .into_iter()
            .map(|ra| MonitorAlertRef {
                name: ra.name,
                failure_threshold: ra.failure_threshold,
                success_threshold: ra.success_threshold,
                send_on_resolved: ra.send_on_resolved,
            })
            .collect();

        monitors.push(Monitor {
            name: rm.name,
            kind,
            http,
            tcp,
            interval,
            timeout,
            failure_threshold: rm.failure_threshold.unwrap_or(default_failure_threshold),
            success_threshold: rm.success_threshold.unwrap_or(default_success_threshold),
            send_on_resolved: rm.send_on_resolved.unwrap_or(default_send_on_resolved),
            conditions: rm.conditions,
            alerts,
        });
    }

    if monitors.is_empty() {
        return Err(UpslimError::Config("No monitors configured".to_owned()));
    }

    Ok(Config {
        monitors,
        alert_providers,
        state_dir,
        max_concurrent,
    })
}
