#![forbid(unsafe_code)]
use std::net::SocketAddr;

use tracing::info;

use realtime_service::{routes, AppState, RealtimeConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg = RealtimeConfig::from_env()?;
    cfg.validate()?;

    init_observability(&cfg.service_name).await?;

    let state = AppState::initialize(cfg.clone()).await?;
    let app = routes::build_router(state);

    let addr: SocketAddr = cfg
        .listen_addr
        .parse()
        .expect("invalid listen_addr for realtime-service");

    info!(%addr, service=%cfg.service_name, "starting realtime-service");

    let listener = tokio::net::TcpListener::bind(addr).await?;

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn init_observability(service_name: &str) -> anyhow::Result<()> {
    let central = rhelma_config::CentralEnv::from_env_strict()?;
    let unified =
        rhelma_config::UnifiedObservabilityConfig::from_central_env(&central, service_name);
    rhelma_config::validation::validate_all(&unified, &central)?;
    rhelma_observability_core::ObservabilityCore::init_from_unified(unified)
        .await
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;

    // Best-effort Prometheus recorder for `/metrics`.
    realtime_service::metrics_endpoint::init_prometheus_recorder();
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{signal, SignalKind};
        if let Ok(mut sigterm) = signal(SignalKind::terminate()) {
            sigterm.recv().await;
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("shutting down realtime-service (graceful)");
}
