#![forbid(unsafe_code)]

use axum::{
    body::Body,
    http::{HeaderValue, Request, StatusCode},
    middleware::from_fn,
    response::IntoResponse,
    routing::get,
    Extension, Router,
};
use rhelma_core::RequestContext;
use tower::ServiceExt;

use api_gateway::middleware::request_guard_middleware;

async fn ok_handler(Extension(ctx): Extension<RequestContext>) -> impl IntoResponse {
    // Prove middleware inserted RequestContext + trace is always present.
    assert!(!ctx.request_id().is_nil());
    assert!(ctx.trace().current_trace_id().is_some());

    let mut resp = StatusCode::NO_CONTENT.into_response();

    // Echo the trace-id for assertions.
    if let Some(tid) = ctx.trace().current_trace_id() {
        resp.headers_mut()
            .insert("x-test-trace-id", HeaderValue::from_str(tid).unwrap());
    }

    resp
}

fn app() -> Router {
    Router::new()
        .route("/ok", get(ok_handler))
        .layer(from_fn(request_guard_middleware))
}

fn get_header(resp: &axum::response::Response, name: &str) -> Option<String> {
    resp.headers()
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

fn is_valid_uuid(s: &str) -> bool {
    uuid::Uuid::parse_str(s).is_ok()
}

#[tokio::test]
async fn missing_x_rhelma_request_id_is_accepted_and_generated() {
    let resp = app()
        .oneshot(Request::builder().uri("/ok").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let rid = get_header(&resp, "x-rhelma-request-id")
        .expect("response must include x-rhelma-request-id");
    assert!(is_valid_uuid(&rid));
}

#[tokio::test]
async fn valid_x_rhelma_request_id_is_accepted_and_echoed() {
    let rid = uuid::Uuid::now_v7().to_string();

    let resp = app()
        .oneshot(
            Request::builder()
                .uri("/ok")
                .header("x-rhelma-request-id", &rid)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let echoed =
        get_header(&resp, "x-rhelma-request-id").expect("response must echo x-rhelma-request-id");
    assert_eq!(echoed, rid);

    // Gateway must not leak legacy headers on responses.
    assert!(resp.headers().get("x-request-id").is_none());
}

#[tokio::test]
async fn legacy_x_request_id_without_x_rhelma_request_id_is_ignored() {
    let legacy = uuid::Uuid::new_v4().to_string();

    let resp = app()
        .oneshot(
            Request::builder()
                .uri("/ok")
                .header("x-request-id", &legacy)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let rid = get_header(&resp, "x-rhelma-request-id")
        .expect("response must include x-rhelma-request-id");
    assert!(is_valid_uuid(&rid));
    assert_ne!(rid, legacy);
}

#[tokio::test]
async fn invalid_x_rhelma_request_id_is_rejected() {
    let resp = app()
        .oneshot(
            Request::builder()
                .uri("/ok")
                .header("x-rhelma-request-id", "not-a-uuid")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn traceparent_is_preserved_and_parsed_into_context() {
    let rid = uuid::Uuid::now_v7().to_string();
    let tp = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01";

    let resp = app()
        .oneshot(
            Request::builder()
                .uri("/ok")
                .header("x-rhelma-request-id", &rid)
                .header("traceparent", tp)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let trace_id = get_header(&resp, "x-test-trace-id").expect("handler must expose trace id");
    assert_eq!(trace_id, "4bf92f3577b34da6a3ce929d0e0e4736");
}
