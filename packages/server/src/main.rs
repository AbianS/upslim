use std::{collections::HashMap, path::PathBuf, sync::Arc};
use tokio_util::sync::CancellationToken;
use tracing::info;
use upslim_server::{
    alert::{AlertProvider, slack::SlackProvider},
    checker::{Checker, http::HttpChecker, tcp::TcpChecker},
    config, error, scheduler,
    state::StateStore,
    types::AlertProviderConfig,
};

#[tokio::main]
async fn main() {
    // Tracing init — nivel configurable con RUST_LOG
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(false)
        .compact()
        .init();

    if let Err(e) = run().await {
        eprintln!("Fatal error: {e}");
        std::process::exit(1);
    }
}

async fn run() -> error::Result<()> {
    let config_path = parse_config_arg();

    info!(path = %config_path.display(), "Loading configuration");

    let cfg = config::load(&config_path)?;

    info!(
        monitors = cfg.monitors.len(),
        providers = cfg.alert_providers.len(),
        "Configuration loaded"
    );

    // Construir alert providers
    let mut providers: HashMap<String, Arc<dyn AlertProvider>> = HashMap::new();
    for provider_config in &cfg.alert_providers {
        let provider: Arc<dyn AlertProvider> = match provider_config {
            AlertProviderConfig::Slack(c) => Arc::new(SlackProvider::new(c)),
        };
        providers.insert(provider_config.name().to_owned(), provider);
    }

    // Inicializar state store
    let state_store = StateStore::load(&cfg.state_dir)?;

    // Construir checkers (compartidos via Arc)
    let checker_http: Arc<dyn Checker> = Arc::new(HttpChecker::new());
    let checker_tcp: Arc<dyn Checker> = Arc::new(TcpChecker);

    // Shutdown token
    let shutdown = CancellationToken::new();
    let shutdown_trigger = shutdown.clone();
    tokio::spawn(async move {
        wait_for_shutdown_signal().await;
        info!("Shutdown signal received, stopping monitors...");
        shutdown_trigger.cancel();
    });

    info!("Starting {} monitors", cfg.monitors.len());

    scheduler::run(
        cfg.monitors,
        checker_http,
        checker_tcp,
        providers,
        state_store,
        cfg.max_concurrent,
        shutdown,
    )
    .await;

    info!("upslim-server stopped");
    Ok(())
}

async fn wait_for_shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        let mut sigterm = signal(SignalKind::terminate()).expect("Failed to register SIGTERM");
        tokio::select! {
            _ = sigterm.recv() => {}
            _ = tokio::signal::ctrl_c() => {}
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}

fn parse_config_arg() -> PathBuf {
    let args: Vec<String> = std::env::args().collect();
    let mut i = 1;
    while i < args.len() {
        if (args[i] == "--config" || args[i] == "-c") && i + 1 < args.len() {
            return PathBuf::from(&args[i + 1]);
        }
        i += 1;
    }
    PathBuf::from("./config")
}
