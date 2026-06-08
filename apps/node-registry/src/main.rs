#![forbid(unsafe_code)]
use std::sync::Arc;

use tracing::info;

mod admission;
mod attestation_verify;
mod config;
mod crypto;
mod docs;
mod error;
mod middleware;
mod models;

// Optional OpenAPI generation + Swagger UI.
#[cfg(feature = "openapi")]
mod openapi;
mod routes;
mod state;
mod store;

use crate::config::NodeRegistryConfig;
use crate::state::AppState;
mod metrics_endpoint;

#[tokio::main]
async fn main() -> Result<(), error::RegistryError> {
    dotenvy::dotenv().ok();

    // 1) Load config first (CentralEnv comes from here)
    let cfg = NodeRegistryConfig::load()?;

    // Touch core config and environment helper so `-D warnings` doesn't fail on dead-code.
    let _ = &cfg.core;
    info!(prod = cfg.is_prod(), service = %cfg.service_name, "node-registry config loaded");

    // 2) Init observability exactly once, using the same CentralEnv
    let unified = rhelma_config::UnifiedObservabilityConfig::from_central_env(
        &cfg.central,
        &cfg.service_name,
    );

    rhelma_config::validation::validate_all(&unified, &cfg.central)
        .map_err(|e| error::RegistryError::config(format!("config validation: {e}")))?;

    rhelma_observability_core::ObservabilityCore::init_from_unified(unified)
        .await
        .map_err(|e| error::RegistryError::config(format!("observability init: {e}")))?;
    crate::metrics_endpoint::init_prometheus_recorder();

    // Governance bootstrap checks (fail-open unless explicitly required)
    rhelma_core::governance::bootstrap::ensure_governance_ready(&cfg.service_name)
        .map_err(|e| error::RegistryError::config(format!("governance: {e}")))?;

    let public_addr = cfg.bind_addr()?;
    let internal_addr = cfg.internal_bind_addr();

    // Stage 4: compatibility switch. When an internal listener is configured,
    // also expose internal routes on the public listener (default: enabled).
    let expose_internal_on_public =
        std::env::var("RHELMA_NODE_REGISTRY__EXPOSE_INTERNAL_ON_PUBLIC")
            .ok()
            .map(|v| {
                let v = v.trim();
                !(v.is_empty() || v == "0" || v.eq_ignore_ascii_case("false"))
            })
            .unwrap_or(true);

    let state = Arc::new(AppState::new(cfg).await?);
    routes::spawn_background_tasks(state.clone());

    // Build public router (no `/v1/internal/*` routes when internal listener is configured).
    let public_app = routes::build_public_router(state.clone());

    // If internal listener is configured, host internal routes there; otherwise keep
    // backwards-compatible single listener with all routes.
    if let (Some(internal_addr), Some(_token)) = (internal_addr, state.cfg.admin_token.clone()) {
        let internal_router = routes::build_internal_router(state.clone());

        let internal_for_public = internal_router.clone().layer(
            rhelma_http_observability::security::ip_allowlist_layer_from_env(
                "RHELMA_NODE_REGISTRY__INTERNAL_PUBLIC_ALLOWLIST",
                &["/v1/internal"],
            ),
        );

        let public_app = if expose_internal_on_public {
            public_app.merge(internal_for_public)
        } else {
            public_app
        };

        let internal_app = internal_router;

        info!("🚀 node-registry public listening on {}", public_addr);
        info!("🔒 node-registry internal listening on {}", internal_addr);

        let public_listener = tokio::net::TcpListener::bind(public_addr)
            .await
            .map_err(|e| error::RegistryError::config(format!("bind {public_addr}: {e}")))?;
        let internal_listener = tokio::net::TcpListener::bind(internal_addr)
            .await
            .map_err(|e| error::RegistryError::config(format!("bind {internal_addr}: {e}")))?;

        let public = axum::serve(
            public_listener,
            public_app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
        );
        let internal = axum::serve(
            internal_listener,
            internal_app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
        );

        tokio::select! {
            res = public => {
                res.map_err(|e| error::RegistryError::internal(format!("public server error: {e}")))?;
            }
            res = internal => {
                res.map_err(|e| error::RegistryError::internal(format!("internal server error: {e}")))?;
            }
        }
    } else {
        // Legacy single listener mode.
        // Stage 4: when internal routes are reachable on the public listener, optionally
        // enforce an IP allow-list for /v1/internal/* endpoints.
        let internal_public = routes::build_internal_router(state.clone()).layer(
            rhelma_http_observability::security::ip_allowlist_layer_from_env(
                "RHELMA_NODE_REGISTRY__INTERNAL_PUBLIC_ALLOWLIST",
                &["/v1/internal"],
            ),
        );

        let app = routes::build_public_router(state.clone()).merge(internal_public);

        info!("🚀 node-registry listening on {}", public_addr);

        let listener = tokio::net::TcpListener::bind(public_addr)
            .await
            .map_err(|e| error::RegistryError::config(format!("bind {public_addr}: {e}")))?;

        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
        )
        .await
        .map_err(|e| error::RegistryError::internal(format!("server error: {e}")))?;
    }

    Ok(())
}
