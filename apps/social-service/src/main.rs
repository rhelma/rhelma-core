#![forbid(unsafe_code)]

use std::net::SocketAddr;
use tracing::info;

use social_service::{routes, AppState, SocialConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg = SocialConfig::from_env()?;
    cfg.validate()?;

    init_observability(&cfg.service_name).await?;

    let state = AppState::initialize(cfg.clone()).await?;
    // Router expects Arc<AppState>; std provides From<T> for Arc<T>.
    let app = routes::build_router(state.into());

    let addr: SocketAddr = cfg
        .listen_addr
        .parse()
        .expect("invalid listen_addr for social-service");

    info!(%addr, service=%cfg.service_name, "starting social-service");

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
    rhelma_observability_core::ObservabilityCore::init_from_unified(unified).await?;

    // Best-effort Prometheus recorder for `/metrics`.
    social_service::routes::metrics::init_prometheus_recorder();
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

    info!("shutting down social-service (graceful)");
}
