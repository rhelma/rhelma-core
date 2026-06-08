#![forbid(unsafe_code)]

use axum::{
    extract::{
        ws::{CloseFrame, Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use futures_util::{SinkExt, StreamExt};
use rhelma_core::constants;
use rhelma_core::prelude::RequestContext;
use rhelma_http_observability::axum::ensure_contract_v60_headers;
use rhelma_http_observability::extract_minimal_headers;
use rhelma_tracing::context::scope_with_headers;
use serde::Deserialize;
use serde_json::{json, Value};
use std::{collections::HashSet, sync::Arc, time::Instant};
use tokio::sync::{mpsc, Mutex};
use tokio::time::interval;
use tracing::{debug, error, info, instrument};

use crate::auth::{anonymous, from_principal, WsAuthContext};
use crate::error::{unauthorized, v52_error, RhelmaErrorResponse, Severity};
use crate::metrics_layer;
use crate::presence::Presence;
use crate::rooms::ConnectionId;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/ws", get(ws_handler))
}

#[derive(Debug, Deserialize)]
pub struct WsQuery {
    /// Field `token`.
    pub token: Option<String>,
    /// Field `rooms`.
    pub rooms: Option<String>,
}

#[instrument(skip(ws, state, headers, query))]
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<WsQuery>,
) -> Result<Response, RhelmaErrorResponse> {
    let (ctx, canonical_headers) = request_context_from_headers_strict(&headers).map_err(|e| *e)?;

    let auth = authenticate(&state, &ctx, &headers, &query).await?;

    enforce_connection_limit(&state, &ctx, &auth).await?;

    let max_msg = state.config.ws_max_message_bytes;
    let trace_headers = extract_minimal_headers(&canonical_headers);

    let resp = ws
        .max_message_size(max_msg)
        .max_frame_size(max_msg)
        .on_upgrade(move |socket| {
            let trace_headers = trace_headers.clone();
            async move {
                scope_with_headers(&trace_headers, async move {
                    handle_socket(socket, state, ctx, auth, query).await
                })
                .await
            }
        })
        .into_response();

    // Echo canonical RequestContext v5.2 headers in the 101 handshake response so clients
    // can correlate end-to-end logs (especially useful when the gateway is bypassed).
    let mut resp = resp;
    if let Some(v) = canonical_headers
        .get(constants::HEADER_MACH_REQUEST_ID)
        .cloned()
    {
        resp.headers_mut()
            .insert(constants::HEADER_MACH_REQUEST_ID, v);
    }
    if let Some(v) = canonical_headers
        .get(constants::HEADER_MACH_CORRELATION_ID)
        .cloned()
    {
        resp.headers_mut()
            .insert(constants::HEADER_MACH_CORRELATION_ID, v);
    }
    if let Some(v) = canonical_headers.get(constants::HEADER_RESIDENCY).cloned() {
        resp.headers_mut().insert(constants::HEADER_RESIDENCY, v);
    }
    if let Some(v) = canonical_headers
        .get(constants::HEADER_TRACEPARENT)
        .cloned()
    {
        resp.headers_mut().insert(constants::HEADER_TRACEPARENT, v);
    }

    Ok(resp)
}

/// Build a RequestContext for WS requests with strict residency validation.
///
/// Notes:
/// - Contract headers are ensured (fail-open) by the router's `ContractV60Layer`.
/// - We still validate residency here because it's a governance/control plane input.
fn request_context_from_headers_strict(
    headers: &HeaderMap,
) -> Result<(RequestContext, HeaderMap), Box<RhelmaErrorResponse>> {
    // 1) Clone and ensure v5.2 headers exist (request/correlation/residency/traceparent).
    let mut canonical = headers.clone();
    ensure_contract_v60_headers(&mut canonical);

    // 2) Normalize residency and reject unknown values (fail-closed).
    let residency = canonical
        .get(constants::HEADER_RESIDENCY)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_ascii_uppercase())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "GLOBAL".to_string());

    let residency_norm = match residency.as_str() {
        "GLOBAL" => "GLOBAL",
        "REGIONAL_PREFERRED" => "REGIONAL_PREFERRED",
        // Back-compat normalization:
        "REGIONAL_ONLY" => "REGIONAL_PREFERRED",
        "REGIONAL_STRICT" | "REGION_STRICT" => "REGIONAL_STRICT",
        _ => {
            // Build a minimal context for a stable error envelope.
            let tmp_pairs: Vec<(&str, &str)> = canonical
                .iter()
                .filter_map(|(k, v)| Some((k.as_str(), v.to_str().ok()?)))
                .collect();
            let tmp_ctx =
                RequestContext::from_headers(tmp_pairs).unwrap_or_else(|_| RequestContext::empty());

            return Err(Box::new(v52_error(
                &tmp_ctx,
                StatusCode::BAD_REQUEST,
                "VALIDATION_ERROR",
                "invalid x-residency",
                false,
                Severity::Low,
                None,
                json!({ "x_residency": residency }),
            )));
        }
    };

    canonical.insert(constants::HEADER_RESIDENCY, residency_norm.parse().unwrap());

    // 3) Build RequestContext.
    let pairs: Vec<(&str, &str)> = canonical
        .iter()
        .filter_map(|(k, v)| Some((k.as_str(), v.to_str().ok()?)))
        .collect();

    let ctx = RequestContext::from_headers(pairs).map_err(|e| {
        let tmp_ctx = RequestContext::empty();
        Box::new(v52_error(
            &tmp_ctx,
            StatusCode::BAD_REQUEST,
            "VALIDATION_ERROR",
            format!("invalid request headers: {e}"),
            false,
            Severity::Low,
            None,
            json!({}),
        ))
    })?;

    Ok((ctx, canonical))
}

async fn authenticate(
    state: &AppState,
    ctx: &RequestContext,
    headers: &HeaderMap,
    query: &WsQuery,
) -> Result<WsAuthContext, RhelmaErrorResponse> {
    let token = extract_bearer(headers).or_else(|| query.token.clone());

    match (token, state.auth.as_ref()) {
        (Some(tok), Some(auth_svc)) => match auth_svc.verify_access_token(&tok).await {
            Ok(p) => Ok(from_principal(p)),
            Err(e) => Err(v52_error(
                ctx,
                StatusCode::UNAUTHORIZED,
                "UNAUTHORIZED",
                format!("authentication failed: {e}"),
                false,
                Severity::High,
                None,
                json!({}),
            )),
        },

        (Some(_), None) => {
            if state.config.allow_anonymous {
                Ok(anonymous())
            } else {
                Err(unauthorized(ctx, "auth service not configured"))
            }
        }

        (None, _) => {
            if state.config.allow_anonymous {
                Ok(anonymous())
            } else {
                Err(unauthorized(ctx, "missing bearer token"))
            }
        }
    }
}

fn extract_bearer(headers: &HeaderMap) -> Option<String> {
    let h = headers.get(axum::http::header::AUTHORIZATION)?;
    let s = h.to_str().ok()?.trim();
    let s = s
        .strip_prefix("Bearer ")
        .or_else(|| s.strip_prefix("bearer "))?;
    let s = s.trim();
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

async fn enforce_connection_limit(
    state: &AppState,
    ctx: &RequestContext,
    auth: &WsAuthContext,
) -> Result<(), RhelmaErrorResponse> {
    let key = auth.user_id.to_string();
    let mut map = state.per_user_conn_count.write().await;
    let cur = map.get(&key).copied().unwrap_or(0);

    if cur >= state.config.max_connections_per_user {
        return Err(v52_error(
            ctx,
            StatusCode::TOO_MANY_REQUESTS,
            "RATE_LIMIT",
            "too many concurrent connections for user",
            true,
            Severity::Medium,
            Some(10_000),
            json!({
                "max_connections_per_user": state.config.max_connections_per_user,
                "current": cur
            }),
        ));
    }

    map.insert(key, cur + 1);
    Ok(())
}

async fn decrement_connection_count(state: &AppState, auth: &WsAuthContext) {
    let key = auth.user_id.to_string();
    let mut map = state.per_user_conn_count.write().await;
    if let Some(v) = map.get_mut(&key) {
        *v = v.saturating_sub(1);
        if *v == 0 {
            map.remove(&key);
        }
    }
}

struct TokenBucket {
    cap: u32,
    tokens: u32,
    refill_per_sec: u32,
    last: Instant,
}

impl TokenBucket {
    fn new(cap: u32, refill_per_sec: u32) -> Self {
        Self {
            cap,
            tokens: cap,
            refill_per_sec,
            last: Instant::now(),
        }
    }

    fn allow(&mut self) -> bool {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last);
        let secs = elapsed.as_secs_f64();

        if secs >= 0.001 {
            let add = (secs * self.refill_per_sec as f64).floor() as u32;
            if add > 0 {
                self.tokens = (self.tokens + add).min(self.cap);
                self.last = now;
            }
        }

        if self.tokens == 0 {
            return false;
        }
        self.tokens -= 1;
        true
    }
}

#[allow(clippy::too_many_arguments)]
fn ws_error_text(
    ctx: &RequestContext,
    status: StatusCode,
    code: &str,
    message: impl Into<String>,
    retryable: bool,
    severity: Severity,
    retry_after_ms: Option<u64>,
    context: Value,
) -> String {
    let resp = v52_error(
        ctx,
        status,
        code,
        message,
        retryable,
        severity,
        retry_after_ms,
        context,
    );

    serde_json::to_string(&json!({
        "type": "error",
        "error": resp.body.error
    }))
    .unwrap_or_else(|_| r#"{"type":"error","error":{"error_code":"SERIALIZE_ERROR","http_status":500,"message":"failed to serialize error","retryable":false,"severity":"HIGH","context":{},"request_id":"","correlation_id":"","timestamp":""}}"#.to_string())
}

async fn handle_socket(
    stream: WebSocket,
    state: AppState,
    ctx: RequestContext,
    auth: WsAuthContext,
    query: WsQuery,
) {
    let opened_at = Instant::now();

    metrics_layer::incr_connections_opened();

    let conn_id = ConnectionId::new();
    let (mut sender, mut receiver) = stream.split();

    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();

    {
        let mut conns = state.connections.write().await;
        conns.insert(conn_id, tx.clone());
        metrics_layer::set_active_connections(conns.len() as i64);
    }

    // Track close reason/code for metrics.
    #[derive(Debug, Clone)]
    struct CloseInfo {
        reason: String,
        code: Option<u16>,
    }
    let close_info: Arc<Mutex<CloseInfo>> = Arc::new(Mutex::new(CloseInfo {
        reason: "unknown".to_string(),
        code: None,
    }));

    let joined_rooms: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));

    if let Some(room_str) = query.rooms.as_deref() {
        for r in room_str
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
        {
            let mut jr = joined_rooms.lock().await;
            if (jr.len() as u32) >= state.config.max_rooms_per_connection {
                break;
            }
            if jr.insert(r.to_string()) {
                drop(jr);
                state.rooms.join(r, conn_id).await;
            }
        }
    }

    {
        let jr = joined_rooms.lock().await;
        if !jr.is_empty() {
            let presence = Presence {
                user_id: auth.user_id,
                tenant_id: auth.tenant_id.clone(),
                rooms: jr.iter().cloned().collect(),
            };
            drop(jr);
            state.presence.update_presence(presence).await;
        }
    }

    let welcome = json!({
        "type": "welcome",
        "user_id": auth.user_id.to_string(),
        "tenant_id": auth.tenant_id.as_ref().map(|t| t.to_string()),
        "session_id": auth.session_id,
        "request_id": ctx.request_id().to_string(),
        "correlation_id": ctx.correlation_id().map(|s| s.to_string()).unwrap_or_else(|| ctx.request_id().to_string()),
    });
    let _ = tx.send(Message::Text(welcome.to_string().into()));

    let last_pong = Arc::new(Mutex::new(Instant::now()));

    let ping_interval = state.config.ws_ping_interval;
    let pong_timeout = state.config.ws_pong_timeout;
    let last_pong_w = last_pong.clone();

    let close_info_s = close_info.clone();

    let mut sender_task = tokio::spawn(async move {
        let mut tick = interval(ping_interval);

        loop {
            tokio::select! {
                _ = tick.tick() => {
                    let lp = *last_pong_w.lock().await;
                    if lp.elapsed() > pong_timeout {
                        debug!("pong timeout; closing socket");
                        let mut ci = close_info_s.lock().await;
                        if ci.reason == "unknown" {
                            ci.reason = "pong_timeout".to_string();
                        }
                        break;
                    }
                    if sender.send(Message::Ping(vec![].into())).await.is_err() {
                        let mut ci = close_info_s.lock().await;
                        if ci.reason == "unknown" {
                            ci.reason = "send_error".to_string();
                        }
                        break;
                    }
                }
                maybe = rx.recv() => {
                    let Some(msg) = maybe else { break; };
                    if sender.send(msg).await.is_err() {
                        let mut ci = close_info_s.lock().await;
                        if ci.reason == "unknown" {
                            ci.reason = "send_error".to_string();
                        }
                        break;
                    }
                    metrics_layer::incr_messages_out();
                }
            }
        }
    });

    let state_clone = state.clone();
    let auth_clone = auth.clone();
    let ctx_clone = ctx.clone();
    let last_pong_r = last_pong.clone();
    let joined_rooms_r = joined_rooms.clone();

    let mut bucket = TokenBucket::new(
        state_clone.config.ws_msg_burst,
        state_clone.config.ws_msgs_per_sec,
    );

    let close_info_r = close_info.clone();
    let mut receiver_task = tokio::spawn(async move {
        while let Some(res) = receiver.next().await {
            match res {
                Ok(Message::Pong(_)) => {
                    *last_pong_r.lock().await = Instant::now();
                }
                Ok(Message::Text(text)) => {
                    metrics_layer::incr_messages_in();

                    if text.len() > state_clone.config.ws_max_message_bytes {
                        let wire = ws_error_text(
                            &ctx_clone,
                            StatusCode::BAD_REQUEST,
                            "VALIDATION_ERROR",
                            "message too large",
                            false,
                            Severity::Low,
                            None,
                            json!({ "max_bytes": state_clone.config.ws_max_message_bytes }),
                        );
                        let _ = tx.send(Message::Text(wire.into()));
                        metrics_layer::incr_messages_rejected();
                        break;
                    }

                    if !bucket.allow() {
                        metrics_layer::incr_rate_limit_hit();
                        let wire = ws_error_text(
                            &ctx_clone,
                            StatusCode::TOO_MANY_REQUESTS,
                            "RATE_LIMIT",
                            "rate limit exceeded",
                            true,
                            Severity::Medium,
                            Some(1000),
                            json!({ "retry_after_ms": 1000 }),
                        );
                        let _ = tx.send(Message::Text(wire.into()));
                        continue;
                    }

                    if let Err(e) = crate::ws::message::handle_message(
                        &state_clone,
                        &ctx_clone,
                        conn_id,
                        &auth_clone,
                        &joined_rooms_r,
                        &tx,
                        text.to_string(),
                    )
                    .await
                    {
                        error!(error = %e, "failed to handle websocket message");
                        let wire = ws_error_text(
                            &ctx_clone,
                            StatusCode::BAD_REQUEST,
                            "VALIDATION_ERROR",
                            "invalid message",
                            false,
                            Severity::Low,
                            None,
                            json!({ "detail": e.to_string() }),
                        );
                        let _ = tx.send(Message::Text(wire.into()));
                        metrics_layer::incr_messages_rejected();
                    }
                }
                Ok(Message::Close(frame)) => {
                    let mut ci = close_info_r.lock().await;
                    ci.reason = "client_close".to_string();
                    ci.code = frame.as_ref().map(|f: &CloseFrame| f.code);
                    break;
                }
                Ok(Message::Ping(_)) => {}
                Ok(Message::Binary(_)) => {}
                Err(e) => {
                    error!(error = %e, "websocket receive error");
                    let mut ci = close_info_r.lock().await;
                    ci.reason = "recv_error".to_string();
                    break;
                }
            }
        }
    });

    let _ = tokio::join!(&mut sender_task, &mut receiver_task);

    let rooms_to_leave: Vec<String> = {
        let jr = joined_rooms.lock().await;
        jr.iter().cloned().collect()
    };
    for r in rooms_to_leave.iter() {
        state.rooms.leave(r, conn_id).await;
    }

    state.presence.clear_presence(&auth.user_id).await;

    {
        let mut conns = state.connections.write().await;
        conns.remove(&conn_id);
        metrics_layer::set_active_connections(conns.len() as i64);
    }

    decrement_connection_count(&state, &auth).await;
    metrics_layer::incr_connections_closed();

    // Close reason/code + duration (best-effort).
    let ci = close_info.lock().await.clone();
    metrics_layer::incr_close(&ci.reason, ci.code);
    metrics_layer::record_connection_duration(opened_at.elapsed().as_secs_f64());

    info!(conn_id = %conn_id, "websocket connection closed");
}

// --- message handler lives in a submodule to keep ws/mod.rs small ---
pub mod message {
    use super::*;
    use anyhow::Result;

    #[derive(Debug, Deserialize)]
    #[serde(tag = "type", rename_all = "snake_case")]
    enum ClientMessage {
        Join { room: String },
        JoinRoom { room_id: String },

        Leave { room: String },
        LeaveRoom { room_id: String },

        Send { room: String, payload: Value },
        SendMessage { room_id: String, content: String },
    }

    pub async fn handle_message(
        state: &AppState,
        ctx: &RequestContext,
        conn_id: ConnectionId,
        auth: &WsAuthContext,
        joined_rooms: &Arc<Mutex<HashSet<String>>>,
        tx: &mpsc::UnboundedSender<Message>,
        text: String,
    ) -> Result<()> {
        let msg: ClientMessage = serde_json::from_str(&text)?;

        match msg {
            ClientMessage::Join { room } => {
                join_room(state, ctx, conn_id, auth, joined_rooms, tx, room).await?
            }
            ClientMessage::JoinRoom { room_id } => {
                join_room(state, ctx, conn_id, auth, joined_rooms, tx, room_id).await?
            }

            ClientMessage::Leave { room } => {
                leave_room(state, ctx, conn_id, auth, joined_rooms, tx, room).await?
            }
            ClientMessage::LeaveRoom { room_id } => {
                leave_room(state, ctx, conn_id, auth, joined_rooms, tx, room_id).await?
            }

            ClientMessage::Send { room, payload } => {
                send_to_room(state, ctx, auth, joined_rooms, tx, &room, payload).await?
            }
            ClientMessage::SendMessage { room_id, content } => {
                send_to_room(
                    state,
                    ctx,
                    auth,
                    joined_rooms,
                    tx,
                    &room_id,
                    json!({ "content": content }),
                )
                .await?
            }
        }

        Ok(())
    }

    async fn join_room(
        state: &AppState,
        ctx: &RequestContext,
        conn_id: ConnectionId,
        auth: &WsAuthContext,
        joined_rooms: &Arc<Mutex<HashSet<String>>>,
        tx: &mpsc::UnboundedSender<Message>,
        room: String,
    ) -> Result<()> {
        let rooms_snapshot: Vec<String>;
        {
            let mut jr = joined_rooms.lock().await;

            if jr.contains(&room) {
                return Ok(());
            }
            if (jr.len() as u32) >= state.config.max_rooms_per_connection {
                let wire = ws_error_text(
                    ctx,
                    StatusCode::BAD_REQUEST,
                    "VALIDATION_ERROR",
                    "max rooms per connection exceeded",
                    false,
                    Severity::Low,
                    None,
                    json!({ "max_rooms_per_connection": state.config.max_rooms_per_connection }),
                );
                let _ = tx.send(Message::Text(wire.into()));
                return Ok(());
            }

            jr.insert(room.clone());
            rooms_snapshot = jr.iter().cloned().collect();
        }

        state.rooms.join(&room, conn_id).await;

        let presence = Presence {
            user_id: auth.user_id,
            tenant_id: auth.tenant_id.clone(),
            rooms: rooms_snapshot,
        };
        state.presence.update_presence(presence).await;

        let ack = json!({ "type": "joined", "room": room });
        let _ = tx.send(Message::Text(ack.to_string().into()));

        let _ = state
            .events
            .publish_ws_audit(
                ctx,
                Some(auth.user_id.to_string()),
                auth.tenant_id.as_ref().map(|t| t.to_string()),
                "realtime.join_room",
                &room,
                json!({}),
            )
            .await;

        Ok(())
    }

    async fn leave_room(
        state: &AppState,
        ctx: &RequestContext,
        conn_id: ConnectionId,
        auth: &WsAuthContext,
        joined_rooms: &Arc<Mutex<HashSet<String>>>,
        tx: &mpsc::UnboundedSender<Message>,
        room: String,
    ) -> Result<()> {
        let rooms_snapshot: Vec<String>;
        {
            let mut jr = joined_rooms.lock().await;
            if !jr.remove(&room) {
                return Ok(());
            }
            rooms_snapshot = jr.iter().cloned().collect();
        }

        state.rooms.leave(&room, conn_id).await;

        let presence = Presence {
            user_id: auth.user_id,
            tenant_id: auth.tenant_id.clone(),
            rooms: rooms_snapshot,
        };
        state.presence.update_presence(presence).await;

        let ack = json!({ "type": "left", "room": room });
        let _ = tx.send(Message::Text(ack.to_string().into()));

        let _ = state
            .events
            .publish_ws_audit(
                ctx,
                Some(auth.user_id.to_string()),
                auth.tenant_id.as_ref().map(|t| t.to_string()),
                "realtime.leave_room",
                &room,
                json!({}),
            )
            .await;

        Ok(())
    }

    async fn send_to_room(
        state: &AppState,
        ctx: &RequestContext,
        auth: &WsAuthContext,
        joined_rooms: &Arc<Mutex<HashSet<String>>>,
        tx: &mpsc::UnboundedSender<Message>,
        room: &str,
        payload: Value,
    ) -> Result<()> {
        let joined = {
            let jr = joined_rooms.lock().await;
            jr.contains(room)
        };

        if !joined {
            let wire = ws_error_text(
                ctx,
                StatusCode::BAD_REQUEST,
                "VALIDATION_ERROR",
                "cannot send to a room you have not joined",
                false,
                Severity::Low,
                None,
                json!({ "room": room }),
            );
            let _ = tx.send(Message::Text(wire.into()));
            return Ok(());
        }

        // Secure defaults:
        if auth.is_anonymous {
            let wire = ws_error_text(
                ctx,
                StatusCode::FORBIDDEN,
                "FORBIDDEN",
                "anonymous users cannot send messages",
                false,
                Severity::High,
                None,
                json!({}),
            );
            let _ = tx.send(Message::Text(wire.into()));
            return Ok(());
        }

        let members = state.rooms.members(room).await;

        let msg = json!({
            "type": "message",
            "room": room,
            "from": auth.user_id.to_string(),
            "tenant_id": auth.tenant_id.as_ref().map(|t| t.to_string()),
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "payload": payload,
        });
        let wire = Message::Text(msg.to_string().into());

        let conns = state.connections.read().await;

        for cid in members {
            if let Some(ch) = conns.get(&cid) {
                let _ = ch.send(wire.clone());
                metrics_layer::incr_messages_out();
            }
        }

        Ok(())
    }
}
