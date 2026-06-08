#![forbid(unsafe_code)]

use axum::{body::Body, http::Request, routing::get, Router};
use tower::ServiceExt;

#[tokio::test]
async fn metrics_endpoint_returns_200() {
    crate::metrics_endpoint::init_prometheus_recorder();

    // Build a minimal router without requiring DB/config.
    let app = Router::new().route("/metrics", get(crate::metrics_endpoint::metrics_handler));

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/metrics")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("router response");

    assert_eq!(resp.status(), axum::http::StatusCode::OK);
}
