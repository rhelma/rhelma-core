#![forbid(unsafe_code)]

use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::from_fn,
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use tower::ServiceExt;

use api_gateway::error::X_MACH_ERROR_TYPE;
use api_gateway::middleware::error_envelope_middleware;

async fn rate_limited_handler() -> Response {
    let mut resp = (StatusCode::TOO_MANY_REQUESTS, "rate limit").into_response();
    resp.headers_mut().insert(
        X_MACH_ERROR_TYPE,
        axum::http::HeaderValue::from_static("rate_limited"),
    );
    resp
}

#[tokio::test]
async fn error_envelope_generates_request_ids_and_wraps_json() {
    let app = Router::new()
        .route("/fail", get(rate_limited_handler))
        .layer(from_fn(error_envelope_middleware));

    let resp = app
        .oneshot(Request::builder().uri("/fail").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);

    let rid = resp
        .headers()
        .get("x-rhelma-request-id")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    let cid = resp
        .headers()
        .get("x-rhelma-correlation-id")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    assert!(uuid::Uuid::parse_str(&rid).is_ok());
    assert!(uuid::Uuid::parse_str(&cid).is_ok());

    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();

    // v5.2 envelope shape: { "error": { ... } }
    let err = v.get("error").expect("must contain error object");
    assert!(err.get("error_code").is_some());

    assert_eq!(
        err.get("request_id").and_then(|x| x.as_str()),
        Some(rid.as_str())
    );

    assert_eq!(
        err.get("correlation_id").and_then(|x| x.as_str()),
        Some(cid.as_str())
    );
}
