use async_trait::async_trait;
use chrono::Utc;
use std::time::Duration;

use crate::{
    error::Result,
    types::{AlertAction, AlertMessage, AlertState, CheckResult},
};

pub mod slack;

// ---------------------------------------------------------------------------
// AlertProvider trait
// ---------------------------------------------------------------------------

#[async_trait]
pub trait AlertProvider: Send + Sync {
    fn name(&self) -> &str;

    /// Validate config at startup — fail fast before starting checks.
    fn validate(&self) -> Result<()>;

    /// Reminder interval (if the provider supports it).
    fn reminder_interval(&self) -> Option<Duration>;

    /// Send notification (failure, recovery, or reminder).
    async fn send(&self, msg: &AlertMessage) -> Result<()>;
}

// ---------------------------------------------------------------------------
// State machine — pura, sin I/O, 100% testable
// ---------------------------------------------------------------------------

/// Advances the alert state for a (monitor, provider) pair given a result.
/// Returns `Some(AlertAction)` if a notification should be sent, `None` otherwise.
pub fn advance_state(
    state: &mut AlertState,
    result: &CheckResult,
    failure_threshold: u32,
    success_threshold: u32,
    send_on_resolved: bool,
    reminder_interval: Option<Duration>,
) -> Option<AlertAction> {
    if result.success {
        // ---- Successful check ----
        state.consecutive_failures = 0;
        state.consecutive_successes += 1;

        if state.is_firing && state.consecutive_successes >= success_threshold {
            state.is_firing = false;
            state.last_sent_at = None;
            if send_on_resolved {
                return Some(AlertAction::Resolve);
            }
        }

        None
    } else {
        // ---- Failed check ----
        state.consecutive_successes = 0;
        state.consecutive_failures += 1;

        if !state.is_firing {
            if state.consecutive_failures >= failure_threshold {
                state.is_firing = true;
                state.last_sent_at = Some(Utc::now());
                return Some(AlertAction::Fire);
            }
        } else {
            // Already firing — check reminder
            if let Some(interval) = reminder_interval {
                if let Some(last_sent) = state.last_sent_at {
                    let elapsed = Utc::now()
                        .signed_duration_since(last_sent)
                        .to_std()
                        .unwrap_or(Duration::ZERO);

                    if elapsed >= interval {
                        state.last_sent_at = Some(Utc::now());
                        return Some(AlertAction::Reminder);
                    }
                }
            }
        }

        None
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::AlertState;
    use chrono::Utc;

    fn ok_result() -> CheckResult {
        CheckResult {
            monitor_name: "test".to_owned(),
            timestamp: Utc::now(),
            success: true,
            response_time_ms: 50,
            status_code: Some(200),
            failure_reason: None,
        }
    }

    fn fail_result() -> CheckResult {
        CheckResult {
            monitor_name: "test".to_owned(),
            timestamp: Utc::now(),
            success: false,
            response_time_ms: 5000,
            status_code: Some(503),
            failure_reason: Some("[STATUS] expected 200, got 503".to_owned()),
        }
    }

    #[test]
    fn no_action_below_threshold() {
        let mut state = AlertState::default();
        // 2 failures, threshold = 3
        for _ in 0..2 {
            let action = advance_state(&mut state, &fail_result(), 3, 2, true, None);
            assert!(action.is_none());
        }
        assert_eq!(state.consecutive_failures, 2);
        assert!(!state.is_firing);
    }

    #[test]
    fn fires_at_threshold() {
        let mut state = AlertState::default();
        // first and second failure — no action
        advance_state(&mut state, &fail_result(), 3, 2, true, None);
        advance_state(&mut state, &fail_result(), 3, 2, true, None);
        // third failure — fires
        let action = advance_state(&mut state, &fail_result(), 3, 2, true, None);
        assert_eq!(action, Some(AlertAction::Fire));
        assert!(state.is_firing);
    }

    #[test]
    fn no_action_while_firing_no_reminder() {
        let mut state = AlertState::default();
        // bring to firing state
        for _ in 0..3 {
            advance_state(&mut state, &fail_result(), 3, 2, true, None);
        }
        // more failures with no reminder_interval — no action
        let action = advance_state(&mut state, &fail_result(), 3, 2, true, None);
        assert!(action.is_none());
    }

    #[test]
    fn resolves_after_success_threshold() {
        let mut state = AlertState::default();
        for _ in 0..3 {
            advance_state(&mut state, &fail_result(), 3, 2, true, None);
        }
        assert!(state.is_firing);

        // first success — not yet resolved
        let a = advance_state(&mut state, &ok_result(), 3, 2, true, None);
        assert!(a.is_none());

        // second success — resolves
        let a = advance_state(&mut state, &ok_result(), 3, 2, true, None);
        assert_eq!(a, Some(AlertAction::Resolve));
        assert!(!state.is_firing);
    }

    #[test]
    fn resolve_suppressed_when_send_on_resolved_false() {
        let mut state = AlertState::default();
        for _ in 0..3 {
            advance_state(&mut state, &fail_result(), 3, 2, false, None);
        }
        advance_state(&mut state, &ok_result(), 3, 2, false, None);
        let a = advance_state(&mut state, &ok_result(), 3, 2, false, None);
        assert!(a.is_none()); // no Resolve sent
        assert!(!state.is_firing);
    }

    #[test]
    fn reminder_fires_after_interval() {
        let mut state = AlertState::default();
        let interval = Duration::from_secs(1);

        // fire alert
        for _ in 0..3 {
            advance_state(&mut state, &fail_result(), 3, 2, true, Some(interval));
        }
        assert!(state.is_firing);

        // Force last_sent_at to more than 1s ago
        state.last_sent_at = Some(Utc::now() - chrono::Duration::seconds(2));

        let a = advance_state(&mut state, &fail_result(), 3, 2, true, Some(interval));
        assert_eq!(a, Some(AlertAction::Reminder));
    }

    #[test]
    fn reminder_does_not_fire_too_soon() {
        let mut state = AlertState::default();
        let interval = Duration::from_secs(3600);

        for _ in 0..3 {
            advance_state(&mut state, &fail_result(), 3, 2, true, Some(interval));
        }

        // Additional failure — reminder interval has not elapsed
        let a = advance_state(&mut state, &fail_result(), 3, 2, true, Some(interval));
        assert!(a.is_none());
    }

    #[test]
    fn counters_reset_on_recovery() {
        let mut state = AlertState::default();
        advance_state(&mut state, &fail_result(), 3, 2, true, None);
        advance_state(&mut state, &fail_result(), 3, 2, true, None);
        assert_eq!(state.consecutive_failures, 2);

        // success — resets failures
        advance_state(&mut state, &ok_result(), 3, 2, true, None);
        assert_eq!(state.consecutive_failures, 0);
        assert_eq!(state.consecutive_successes, 1);
    }
}
