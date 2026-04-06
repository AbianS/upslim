use async_trait::async_trait;
use std::time::Instant;
use tracing::debug;

use crate::{
    checker::Checker,
    condition::{evaluate, EvalContext},
    error::{Result, UpslimError},
    types::{CheckKind, CheckResult, Monitor},
};

pub struct HttpChecker {
    client: reqwest::Client,
}

impl HttpChecker {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .tcp_nodelay(true)
            .redirect(reqwest::redirect::Policy::limited(5))
            .use_rustls_tls()
            .build()
            .expect("Failed to build HTTP client");
        Self { client }
    }
}

impl Default for HttpChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Checker for HttpChecker {
    async fn check(&self, monitor: &Monitor) -> Result<CheckResult> {
        debug_assert_eq!(monitor.kind, CheckKind::Http);

        let http = match &monitor.http {
            Some(h) => h,
            None => {
                return Ok(CheckResult::transport_error(
                    &monitor.name,
                    "HTTP monitor missing url configuration",
                ));
            }
        };

        let method =
            reqwest::Method::from_bytes(http.method.as_bytes()).unwrap_or(reqwest::Method::GET);

        let mut req = self.client.request(method, &http.url);

        for (key, value) in &http.headers {
            req = req.header(key, value);
        }

        if let Some(body) = &http.body {
            req = req.body(body.clone());
        }

        let start = Instant::now();

        let response = tokio::time::timeout(monitor.timeout, req.send())
            .await
            .map_err(|_| {
                UpslimError::Check(format!("Timeout after {}ms", monitor.timeout.as_millis()))
            })?
            .map_err(|e| UpslimError::Check(e.to_string()))?;

        let response_time_ms = start.elapsed().as_millis() as u64;
        let status_code = response.status().as_u16();

        // Read body only if any condition references it
        let body = if monitor.conditions.iter().any(|c| c.contains("[BODY]")) {
            match response.text().await {
                Ok(b) => Some(b),
                Err(e) => {
                    debug!(monitor = %monitor.name, error = %e, "failed to read body");
                    None
                }
            }
        } else {
            None
        };

        let ctx = EvalContext {
            status: Some(status_code),
            response_time_ms,
            body,
            connected: None,
        };

        let (success, failure_reason) = evaluate(&monitor.conditions, &ctx);

        Ok(CheckResult {
            monitor_name: monitor.name.clone(),
            timestamp: chrono::Utc::now(),
            success,
            response_time_ms,
            status_code: Some(status_code),
            failure_reason,
        })
    }
}
