#![forbid(unsafe_code)]

use axum::Router;
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio_tungstenite::{connect_async, tungstenite::Message};

/// Smoke test: start realtime-service router and ensure WS handshake works and we receive `welcome`.
///
/// Notes:
/// - Runs with allow_anonymous=true so it doesn't depend on AuthService/Redis.
/// - Uses `connect_async` with a &str/String request (Url does NOT implement IntoClientRequest in our dep set).
#[tokio::test]
async fn websocket_welcome_smoke_test() {
    // Build minimal app state via realtime-service library exports.
    // If your crate exposes different constructors, adjust accordingly.
    let mut cfg = realtime_service::config::RealtimeConfig::for_tests();
    cfg.allow_anonymous = true;

    let state = realtime_service::state::AppState::initialize(cfg)
        .await
        .expect("init state");

    let app: Router = realtime_service::routes::build_router(state);

    // Bind ephemeral port
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");

    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve");
    });

    // Connect WS (MUST be &str / String, not Url)
    // If your realtime-service uses a different path, change "/ws" here.
    let url = format!("ws://{addr}/ws");
    let (mut ws, _resp) = connect_async(url.as_str()).await.expect("connect ws");

    // Expect welcome message (first text frame)
    let msg = ws.next().await.expect("first frame").expect("frame ok");

    let text = match msg {
        Message::Text(s) => s,
        Message::Binary(b) => String::from_utf8(b).expect("welcome binary must be utf8"),
        other => panic!("expected text/binary welcome frame, got: {other:?}"),
    };

    assert!(
        text.contains(r#""type":"welcome""#)
            || text.contains(r#""type": "welcome""#)
            || text.contains(r#""type":"welcome_v1""#),
        "expected welcome, got: {text}"
    );

    // Close (best-effort)
    let _ = ws.send(Message::Close(None)).await;

    // Stop server task
    server.abort();
    // prevent "JoinError: cancelled" from causing flaky confusion in some setups
    let _ = server.await;
}
