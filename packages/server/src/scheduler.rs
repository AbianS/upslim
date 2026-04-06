use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::{sync::Semaphore, time::MissedTickBehavior};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};

use crate::{
    alert::{AlertProvider, advance_state},
    checker::Checker,
    state::{StateStore, state_key},
    types::{AlertAction, AlertMessage, CheckResult, Monitor},
};

/// Arranca un task Tokio por monitor con stagger de 222ms entre ellos.
/// Retorna cuando el `shutdown` token es cancelado y todos los tasks terminan.
pub async fn run(
    monitors: Vec<Monitor>,
    checker_http: Arc<dyn Checker>,
    checker_tcp: Arc<dyn Checker>,
    providers: HashMap<String, Arc<dyn AlertProvider>>,
    state_store: StateStore,
    max_concurrent: usize,
    shutdown: CancellationToken,
) {
    // Validar todos los providers al inicio
    for (name, provider) in &providers {
        if let Err(e) = provider.validate() {
            error!(provider = %name, error = %e, "Alert provider validation failed — check your config");
        }
    }

    let semaphore = Arc::new(Semaphore::new(max_concurrent));
    let mut handles = Vec::with_capacity(monitors.len());

    for (i, monitor) in monitors.into_iter().enumerate() {
        let stagger = Duration::from_millis(i as u64 * 222);
        let checker = match monitor.kind {
            crate::types::CheckKind::Http => checker_http.clone(),
            crate::types::CheckKind::Tcp => checker_tcp.clone(),
        };

        let handle = tokio::spawn(monitor_loop(
            monitor,
            stagger,
            semaphore.clone(),
            checker,
            providers.clone(),
            state_store.clone(),
            shutdown.clone(),
        ));
        handles.push(handle);
    }

    // Esperar a que todos los tasks terminen
    for handle in handles {
        if let Err(e) = handle.await {
            error!(error = ?e, "Monitor task panicked");
        }
    }

    info!("All monitor tasks stopped");
}

async fn monitor_loop(
    monitor: Monitor,
    stagger: Duration,
    semaphore: Arc<Semaphore>,
    checker: Arc<dyn Checker>,
    providers: HashMap<String, Arc<dyn AlertProvider>>,
    state_store: StateStore,
    shutdown: CancellationToken,
) {
    // Stagger de arranque para evitar thundering herd
    if !stagger.is_zero() {
        tokio::select! {
            _ = tokio::time::sleep(stagger) => {}
            _ = shutdown.cancelled() => {
                debug!(monitor = %monitor.name, "Shutdown before stagger elapsed");
                return;
            }
        }
    }

    let mut interval = tokio::time::interval(monitor.interval);
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

    info!(
        monitor = %monitor.name,
        interval_secs = monitor.interval.as_secs(),
        "Monitor started"
    );

    loop {
        tokio::select! {
            _ = interval.tick() => {
                run_check(&monitor, &semaphore, &checker, &providers, &state_store).await;
            }
            _ = shutdown.cancelled() => {
                info!(monitor = %monitor.name, "Monitor shutting down");
                break;
            }
        }
    }
}

async fn run_check(
    monitor: &Monitor,
    semaphore: &Semaphore,
    checker: &Arc<dyn Checker>,
    providers: &HashMap<String, Arc<dyn AlertProvider>>,
    state_store: &StateStore,
) {
    // Adquirir slot de concurrencia
    let _permit = match semaphore.acquire().await {
        Ok(p) => p,
        Err(_) => {
            warn!(monitor = %monitor.name, "Semaphore closed, skipping check");
            return;
        }
    };

    let result = match checker.check(monitor).await {
        Ok(r) => r,
        Err(e) => {
            error!(monitor = %monitor.name, error = %e, "Checker returned error");
            CheckResult::transport_error(&monitor.name, e)
        }
    };

    if result.success {
        debug!(
            monitor = %monitor.name,
            response_time_ms = result.response_time_ms,
            "Check OK"
        );
    } else {
        warn!(
            monitor = %monitor.name,
            response_time_ms = result.response_time_ms,
            reason = ?result.failure_reason,
            "Check FAILED"
        );
    }

    // Procesar alertas para cada provider configurado en este monitor
    for alert_ref in &monitor.alerts {
        let key = state_key(&monitor.name, &alert_ref.name);
        let mut state = state_store.get(&key);

        let ft = alert_ref
            .failure_threshold
            .unwrap_or(monitor.failure_threshold);
        let st = alert_ref
            .success_threshold
            .unwrap_or(monitor.success_threshold);
        let sor = alert_ref
            .send_on_resolved
            .unwrap_or(monitor.send_on_resolved);

        let provider = providers.get(&alert_ref.name);
        let reminder = provider.and_then(|p| p.reminder_interval());

        let action = advance_state(&mut state, &result, ft, st, sor, reminder);

        state_store.set(&key, state);

        if let Some(action) = action {
            let Some(provider) = provider else {
                warn!(
                    monitor = %monitor.name,
                    provider = %alert_ref.name,
                    "Alert provider not found in config"
                );
                continue;
            };

            let msg = AlertMessage {
                monitor_name: monitor.name.clone(),
                monitor_url: monitor.url(),
                result: result.clone(),
                is_resolved: action == AlertAction::Resolve,
                is_reminder: action == AlertAction::Reminder,
            };

            if let Err(e) = provider.send(&msg).await {
                error!(
                    monitor = %monitor.name,
                    provider = %alert_ref.name,
                    error = %e,
                    "Failed to send alert"
                );
            } else {
                info!(
                    monitor = %monitor.name,
                    provider = %alert_ref.name,
                    action = ?action,
                    "Alert sent"
                );
            }
        }
    }
}
