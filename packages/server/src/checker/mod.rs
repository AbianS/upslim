use crate::{
    error::Result,
    types::{CheckResult, Monitor},
};
use async_trait::async_trait;

pub mod http;
pub mod tcp;

#[async_trait]
pub trait Checker: Send + Sync {
    /// Runs a single check.
    /// Implementations MUST respect `monitor.timeout`.
    async fn check(&self, monitor: &Monitor) -> Result<CheckResult>;
}
