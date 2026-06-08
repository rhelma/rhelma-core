#![forbid(unsafe_code)]

#[path = "../src/metrics_endpoint.rs"]
mod metrics_endpoint;

use axum::http::StatusCode;
use axum::response::IntoResponse;

#[tokio::test]
async fn metrics_endpoint_returns_ok() {
    // Best-effort: the recorder may already be installed by observability core.
    // The endpoint must still respond with 200 so Prometheus scrape checks stay stable.
    metrics_endpoint::init_prometheus_recorder();

    let resp = metrics_endpoint::metrics_handler().await.into_response();
    assert_eq!(resp.status(), StatusCode::OK);
}
