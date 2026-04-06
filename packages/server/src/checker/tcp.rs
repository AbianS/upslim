use async_trait::async_trait;
use std::time::Instant;

use crate::{
    checker::Checker,
    condition::{EvalContext, evaluate},
    error::Result,
    types::{CheckKind, CheckResult, Monitor},
};

pub struct TcpChecker;

#[async_trait]
impl Checker for TcpChecker {
    async fn check(&self, monitor: &Monitor) -> Result<CheckResult> {
        debug_assert_eq!(monitor.kind, CheckKind::Tcp);

        let tcp = match &monitor.tcp {
            Some(t) => t,
            None => {
                return Ok(CheckResult::transport_error(
                    &monitor.name,
                    "TCP monitor missing host/port configuration",
                ));
            }
        };

        let addr = format!("{}:{}", tcp.host, tcp.port);
        let start = Instant::now();

        let connected =
            tokio::time::timeout(monitor.timeout, tokio::net::TcpStream::connect(&addr))
                .await
                .map(|r| r.is_ok())
                .unwrap_or(false);

        let response_time_ms = start.elapsed().as_millis() as u64;

        let ctx = EvalContext {
            connected: Some(connected),
            response_time_ms,
            ..Default::default()
        };

        let (success, failure_reason) = evaluate(&monitor.conditions, &ctx);

        Ok(CheckResult {
            monitor_name: monitor.name.clone(),
            timestamp: chrono::Utc::now(),
            success,
            response_time_ms,
            status_code: None,
            failure_reason,
        })
    }
}
