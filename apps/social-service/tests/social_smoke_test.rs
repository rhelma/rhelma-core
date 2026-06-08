#![forbid(unsafe_code)]

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use secrecy::Secret;
use std::sync::Arc;
use tower::ServiceExt;

use rhelma_config::{CentralEnv, CoreConfig, FileBackend};
use social_service::{config::SocialConfig, routes, state::AppState};

async fn try_state() -> Option<Arc<AppState>> {
    // These tests are designed to compile everywhere, and run when Postgres+Redis are available
    // (e.g. via docker-compose.dev.yml).

    let database_url = std::env::var("RHELMA_TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@127.0.0.1:5432/rhelma".to_string());
    let redis_url = std::env::var("RHELMA_TEST_REDIS_URL")
        .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());

    // Quick dependency probes (skip if unavailable)
    if sqlx::PgPool::connect(&database_url).await.is_err() {
        eprintln!("skipping social-service tests: postgres not reachable ({database_url})");
        return None;
    }
    let client = match redis::Client::open(redis_url.clone()) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("skipping social-service tests: redis URL invalid: {e}");
            return None;
        }
    };
    let redis_ok = tokio::task::spawn_blocking(move || match client.get_connection() {
        Ok(mut conn) => redis::cmd("PING").query::<()>(&mut conn).is_ok(),
        Err(_) => false,
    })
    .await
    .unwrap_or(false);

    if !redis_ok {
        eprintln!("skipping social-service tests: redis not reachable ({redis_url})");
        return None;
    }

    let cfg = SocialConfig {
        central: CentralEnv {
            environment: "test".into(),
            region: "local".into(),
            service_version: "0.0.0-test".into(),
            tenant_id: None,
        },
        core: CoreConfig {
            db_url: Secret::new(database_url),
            db_read_replica_url: None,
            db_max_connections: None,
            db_min_connections: None,

            redis_url: Some(Secret::new(redis_url.clone())),
            redis_default_ttl_secs: None,

            file_backend: FileBackend::Local,
            file_local_root: None,
            file_s3_endpoint: None,
            file_s3_region: None,
            file_s3_bucket: None,

            obs_json_logs: false,
            obs_log_level: None,
            obs_otel_endpoint: None,
            obs_prometheus_port: None,
        },
        service_name: "social-service".into(),
        listen_addr: "127.0.0.1:0".into(),
        redis_url: Secret::new(redis_url),
        feed_default_limit: 20,
        feed_max_limit: 100,
    };

    let state = match AppState::initialize(cfg).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("skipping social-service tests: state init failed: {e}");
            return None;
        }
    };

    Some(Arc::new(state))
}

#[tokio::test]
async fn health_is_200() {
    let Some(state) = try_state().await else {
        return;
    };

    let app = routes::build_router(state);
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn create_post_without_bearer_token_is_401() {
    let Some(state) = try_state().await else {
        return;
    };

    let app = routes::build_router(state);

    let req = Request::builder()
        .method("POST")
        .uri("/posts")
        .header("content-type", "application/json")
        .header("x-tenant-id", "central")
        .header("x-region", "local")
        .body(Body::from(r#"{"kind":"post","body":"hi"}"#))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn latest_feed_missing_tenant_is_400() {
    let Some(state) = try_state().await else {
        return;
    };

    let app = routes::build_router(state);

    let req = Request::builder()
        .uri("/feed/latest")
        .header("x-region", "local")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}
