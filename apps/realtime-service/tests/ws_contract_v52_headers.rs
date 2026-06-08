#![forbid(unsafe_code)]

use axum::Router;
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{client::IntoClientRequest, Message},
};

/// Contract (v5.2): realtime-service WS handshake must be resilient to malformed/missing
/// Rhelma headers because it can be used internally behind api-gateway.
///
/// This test ensures:
/// - Malformed `x-rhelma-request-id` does NOT break the handshake.
/// - Malformed `traceparent` does NOT break the handshake.
#[tokio::test]
async fn websocket_accepts_invalid_request_id_and_traceparent_headers() {
    let mut cfg = realtime_service::config::RealtimeConfig::for_tests();
    cfg.allow_anonymous = true;

    let state = realtime_service::state::AppState::initialize(cfg)
        .await
        .expect("init state");

    let app: Router = realtime_service::routes::build_router(state);

    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");

    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve");
    });

    let url_str = format!("ws://{}/ws", addr);
    let mut req = url_str
        .as_str()
        .into_client_request()
        .expect("into request");

    // Intentionally malformed headers.
    req.headers_mut()
        .insert("x-rhelma-request-id", "not-a-uuid".parse().unwrap());
    req.headers_mut()
        .insert("traceparent", "nope".parse().unwrap());

    let (mut ws, _resp) = connect_async(req).await.expect("connect ws");

    let msg = ws.next().await.expect("first frame").expect("frame ok");
    let text = match msg {
        Message::Text(s) => s,
        other => panic!("expected text welcome frame, got: {:?}", other),
    };

    assert!(
        text.contains(r#""type":"welcome""#) || text.contains(r#""type": "welcome""#),
        "expected welcome, got: {text}"
    );

    let _ = ws.send(Message::Close(None)).await;
    server.abort();
}
