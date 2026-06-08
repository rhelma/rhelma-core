#![forbid(unsafe_code)]

use axum::{body::Body, http::Request, routing::get, Router};
use tower::ServiceExt;

/// Phase 94: `/metrics` must be scrapeable.
///
/// We keep this test independent from heavy backends (qdrant/meili) by
/// mounting the handler on a minimal router.
#[tokio::test]
async fn metrics_endpoint_returns_200() {
    crate::metrics_endpoint::init_prometheus_recorder();

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
