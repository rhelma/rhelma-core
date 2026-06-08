#![allow(dead_code)]
#![forbid(unsafe_code)]
use std::sync::Arc;
use tracing::{info, warn};

mod config;
mod docs;
mod error;
mod eventing;
mod middleware;
mod region_routing;
mod routes;
mod services;
mod state;

#[cfg(feature = "openapi")]
mod openapi;

use crate::config::GatewayConfig;
use crate::state::AppState;
mod metrics_endpoint;

#[tokio::main]
async fn main() -> Result<(), crate::error::ApiError> {
    dotenvy::dotenv().ok();

    // 1) Load config first (CentralEnv comes from here)
    let cfg = GatewayConfig::load()?;

    // 2) Init observability exactly once, using the same CentralEnv
    let unified = rhelma_config::UnifiedObservabilityConfig::from_central_env(
        &cfg.central,
        &cfg.service_name,
    );

    rhelma_config::validation::validate_all(&unified, &cfg.central)
        .map_err(|e| crate::error::ApiError::internal(format!("config validation: {e}")))?;

    rhelma_observability_core::ObservabilityCore::init_from_unified(unified)
        .await
        .map_err(|e| crate::error::ApiError::internal(format!("observability init: {e}")))?;

    // 3) Governance bootstrap checks (fail-open unless explicitly required)
    rhelma_core::governance::bootstrap::ensure_governance_ready(&cfg.service_name)
        .map_err(|e| crate::error::ApiError::internal(format!("governance: {e}")))?;

    let addr = cfg.bind_addr()?;

    let state = Arc::new(AppState::new(cfg).await?);
    let app = routes::build_router(state);

    info!("🚀 api-gateway listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| crate::error::ApiError::internal(format!("bind error: {e}")))?;

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|e| crate::error::ApiError::internal(format!("server error: {e}")))?;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{signal, SignalKind};
        let mut sigterm = signal(SignalKind::terminate()).expect("sigterm handler");
        sigterm.recv().await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => warn!("shutdown: ctrl+c"),
        _ = terminate => warn!("shutdown: sigterm"),
    }
}
