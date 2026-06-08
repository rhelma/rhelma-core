#![forbid(unsafe_code)]

use std::{collections::HashMap, sync::Arc, time::Duration};

use axum::{body::Body, http::Request};
use realtime_service::{
    eventing::EventSink, presence::InMemoryPresenceBackend, rooms::RoomManager, routes,
    state::AppState, RealtimeConfig,
};
use tokio::sync::RwLock;
use tower::ServiceExt;

#[tokio::test]
async fn metrics_endpoint_returns_200() {
    realtime_service::metrics_endpoint::init_prometheus_recorder();

    // Build a minimal in-memory AppState without external dependencies.
    let cfg = RealtimeConfig {
        service_name: "realtime-service".to_string(),
        environment: "test".to_string(),
        region: "test".to_string(),
        listen_addr: "127.0.0.1:0".to_string(),
        allow_anonymous: true,
        ws_max_message_bytes: 256 * 1024,
        ws_ping_interval: Duration::from_secs(30),
        ws_pong_timeout: Duration::from_secs(10),
        ws_msgs_per_sec: 10,
        ws_msg_burst: 10,
        max_connections_per_user: 5,
        max_rooms_per_connection: 5,
        auth_redis_url_override: None,
    };

    let state = AppState {
        config: cfg.clone(),
        presence: Arc::new(InMemoryPresenceBackend::default()),
        rooms: Arc::new(RoomManager::new()),
        events: Arc::new(
            EventSink::new(cfg.service_name.clone(), cfg.region.clone())
                .await
                .unwrap(),
        ),
        auth: None,
        connections: Arc::new(RwLock::new(HashMap::new())),
        per_user_conn_count: Arc::new(RwLock::new(HashMap::new())),
    };

    let app = routes::build_router(state);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/metrics")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), axum::http::StatusCode::OK);
}
