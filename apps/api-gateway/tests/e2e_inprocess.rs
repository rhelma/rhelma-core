#![forbid(unsafe_code)]

use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::from_fn,
    response::IntoResponse,
    routing::get,
    Extension, Router,
};
use tower::ServiceExt;
use uuid::Uuid;

use api_gateway::middleware::{request_guard_middleware, OptionalAuthUserExtractor};
use rhelma_core::RequestContext;

async fn ok_ctx(Extension(_ctx): Extension<RequestContext>) -> impl IntoResponse {
    "ok"
}

async fn ok_optional(
    OptionalAuthUserExtractor(_p): OptionalAuthUserExtractor,
) -> impl IntoResponse {
    "ok"
}

fn mk_req(path: &str) -> Request<Body> {
    Request::builder()
        .uri(path)
        .header("x-rhelma-request-id", Uuid::new_v4().to_string())
        .header("x-rhelma-correlation-id", Uuid::new_v4().to_string())
        .header("x-tenant-id", "tenant-001")
        .header("x-region", "local")
        .header("x-residency", "GLOBAL")
        .header(
            "traceparent",
            "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01",
        )
        .body(Body::empty())
        .unwrap()
}

#[tokio::test]
async fn request_guard_accepts_missing_headers_and_generates_minimum_set() {
    let app = Router::new()
        .route("/", get(ok_ctx))
        .layer(from_fn(request_guard_middleware));

    let req = Request::builder().uri("/").body(Body::empty()).unwrap();
    let resp = app.oneshot(req).await.unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    assert!(resp.headers().get("x-rhelma-request-id").is_some());
    assert!(resp.headers().get("x-rhelma-correlation-id").is_some());
}

#[tokio::test]
async fn request_guard_allows_with_headers_and_injects_ctx() {
    let app = Router::new()
        .route("/", get(ok_ctx))
        .layer(from_fn(request_guard_middleware));

    let resp = app.oneshot(mk_req("/")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn optional_auth_without_auth_service_is_anonymous_not_401() {
    // No Extension<Arc<AuthService>> on purpose.
    let app = Router::new()
        .route("/", get(ok_optional))
        .layer(from_fn(request_guard_middleware));

    let resp = app.oneshot(mk_req("/")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}
