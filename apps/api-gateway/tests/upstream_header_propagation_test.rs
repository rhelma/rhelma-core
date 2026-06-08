#![forbid(unsafe_code)]

use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use axum::http::HeaderMap;
use axum::{extract::State, routing::post, Json, Router};
use tokio::net::TcpListener;

use api_gateway::{
    config::{CorsConfig, GatewayConfig, ServiceEndpoints, TimeoutsConfig},
    eventing::{build_event_bus, GatewayEventPublisher},
    routes::search::SearchResponse,
    services::SearchService,
};
use rhelma_config::{CentralEnv, CoreConfig, FileBackend};
use rhelma_core::constants;
use rhelma_core::RequestContext;
use secrecy::Secret;

#[derive(Clone, Default)]
struct Capture(Arc<Mutex<Option<HeaderMap>>>);

async fn capture_handler(
    State(cap): State<Capture>,
    headers: HeaderMap,
    Json(_req): Json<serde_json::Value>,
) -> Json<SearchResponse> {
    *cap.0.lock().expect("lock") = Some(headers);
    // `SearchResponse` in api-gateway routes uses JSON values for hits.
    Json(SearchResponse {
        query: "hello".into(),
        limit: 1,
        hits: vec![serde_json::json!({
            "doc_id": "doc-1",
            "score": 1.0,
            "snippet": "ok"
        })],
    })
}

fn test_config(search_url: String) -> GatewayConfig {
    GatewayConfig {
        central: CentralEnv {
            environment: "test".into(),
            region: "local".into(),
            service_version: "0.0.0-test".into(),
            tenant_id: None,
        },
        core: CoreConfig {
            db_url: Secret::new("postgres://postgres:postgres@127.0.0.1:5432/rhelma".into()),
            db_read_replica_url: None,
            db_max_connections: None,
            db_min_connections: None,

            redis_url: Some(Secret::new("redis://127.0.0.1:6379".into())),
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
        service_name: "api-gateway".into(),
        bind_host: "127.0.0.1".into(),
        bind_port: 0,
        cors: CorsConfig {
            allow_origins: vec!["*".into()],
            allow_credentials: false,
        },
        timeouts: TimeoutsConfig {
            global: std::time::Duration::from_secs(2),
            upstream: std::time::Duration::from_secs(2),
        },
        services: ServiceEndpoints {
            auth_service_url: "http://127.0.0.1:7000".into(),
            search_service_url: search_url,
            social_service_url: "http://127.0.0.1:7300".into(),
            user_service_url: "http://127.0.0.1:7100".into(),
            ai_service_url: "http://127.0.0.1:7200".into(),
            control_service_url: None,
        },
        discovery_cache_ttl: std::time::Duration::from_secs(30),
        region_routing: None,
        kafka_brokers: "noop".into(),
        kafka_topic_prefix: "rhelma".into(),
        publish_region_events: false,
        redis_url: Secret::new("redis://127.0.0.1:6379".into()),
        rate_limit_requests_per_minute: 60,
        rate_limit_burst: 20,
    }
}

#[tokio::test]
async fn propagates_rhelma_contract_headers_to_upstream() {
    let cap = Capture::default();

    // Mock upstream server (captures incoming headers).
    let app = Router::new()
        .route("/search", post(capture_handler))
        .with_state(cap.clone());

    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr: SocketAddr = listener.local_addr().expect("addr");
    let server = axum::serve(listener, app);
    tokio::spawn(async move {
        // `Serve` implements `IntoFuture` (not `Future`) in axum.
        let _ = server.await;
    });

    let cfg = Arc::new(test_config(format!("http://{addr}")));
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .expect("reqwest client");

    // Event publishing is disabled in this test; we pass a noop bus.
    let bus = build_event_bus(
        &cfg.service_name,
        &cfg.kafka_brokers,
        &cfg.kafka_topic_prefix,
    );
    let publisher = Arc::new(GatewayEventPublisher::new(
        false,
        cfg.service_name.clone(),
        cfg.central.region.clone(),
        bus,
    ));

    let svc = SearchService::new(cfg, client, None, publisher);

    let request_id = "0193c730-2f4a-7a3b-9c7a-000000000001";
    let correlation_id = "0193c730-2f4a-7a3b-9c7a-000000000002";
    let residency = "local";
    let traceparent = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01";

    let mut h = HashMap::new();
    h.insert(
        constants::HEADER_MACH_REQUEST_ID.to_string(),
        request_id.to_string(),
    );
    h.insert(
        constants::HEADER_MACH_CORRELATION_ID.to_string(),
        correlation_id.to_string(),
    );
    h.insert(
        constants::HEADER_RESIDENCY.to_string(),
        residency.to_string(),
    );
    h.insert(
        constants::HEADER_TRACEPARENT.to_string(),
        traceparent.to_string(),
    );

    rhelma_tracing::context::scope_with_headers(&h, async {
        let ctx = RequestContext::empty();
        let _ = svc.search(&ctx, "hello", 1).await.expect("search call");
    })
    .await;

    let captured = cap
        .0
        .lock()
        .expect("lock")
        .take()
        .expect("captured headers");

    assert_eq!(
        captured
            .get(constants::HEADER_MACH_REQUEST_ID)
            .expect("x-rhelma-request-id")
            .to_str()
            .expect("utf8"),
        request_id
    );
    assert_eq!(
        captured
            .get(constants::HEADER_MACH_CORRELATION_ID)
            .expect("x-rhelma-correlation-id")
            .to_str()
            .expect("utf8"),
        correlation_id
    );
    assert_eq!(
        captured
            .get(constants::HEADER_RESIDENCY)
            .expect("x-residency")
            .to_str()
            .expect("utf8"),
        residency
    );
    assert_eq!(
        captured
            .get(constants::HEADER_TRACEPARENT)
            .expect("traceparent")
            .to_str()
            .expect("utf8"),
        traceparent
    );
}
