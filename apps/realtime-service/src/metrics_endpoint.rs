#![forbid(unsafe_code)]

//! Prometheus `/metrics` endpoint wiring for realtime-service.
//!
//! The recorder installation is best-effort to avoid conflicts with other
//! global recorders. The handler always returns 200 for scrapeability.

use std::time::Duration;

use axum::{
    http::{header, HeaderValue, StatusCode},
    response::IntoResponse,
};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use once_cell::sync::OnceCell;
use tracing::{info, warn};

static PROM: OnceCell<PrometheusHandle> = OnceCell::new();

pub fn init_prometheus_recorder() {
    if PROM.get().is_some() {
        return;
    }

    let builder = PrometheusBuilder::new().upkeep_timeout(Duration::from_secs(5));

    match builder.install_recorder() {
        Ok(handle) => {
            let _ = PROM.set(handle.clone());
            spawn_prometheus_upkeep(handle);
            info!("prometheus recorder installed (realtime-service)");
        }
        Err(e) => {
            warn!(error = %e, "failed to install prometheus recorder (metrics may be unavailable)");
        }
    }
}

fn spawn_prometheus_upkeep(handle: PrometheusHandle) {
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(5));
        loop {
            tick.tick().await;
            handle.run_upkeep();
        }
    });
}

pub async fn metrics_handler() -> impl IntoResponse {
    let body = PROM
        .get()
        .map(|h| h.render())
        .unwrap_or_else(|| "# metrics_unavailable 1\n".to_string());

    (
        StatusCode::OK,
        [(
            header::CONTENT_TYPE,
            HeaderValue::from_static("text/plain; version=0.0.4"),
        )],
        body,
    )
}
