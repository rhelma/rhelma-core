#![forbid(unsafe_code)]

use crate::auth::build_auth_service;
use crate::config::RealtimeConfig;
use crate::eventing::EventSink;
use crate::presence::{InMemoryPresenceBackend, PresenceBackend};
use crate::rooms::{ConnectionId, RoomManager};
use rhelma_auth::AuthService;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

use axum::extract::ws::Message;

#[derive(Clone)]
pub struct AppState {
    /// Field `config`.
    pub config: RealtimeConfig,

    /// Field `presence`.
    pub presence: Arc<dyn PresenceBackend>,
    /// Field `rooms`.
    pub rooms: Arc<RoomManager>,
    /// Field `events`.
    pub events: Arc<EventSink>,

    /// Field `auth`.
    pub auth: Option<Arc<AuthService>>,

    /// ConnectionId -> sender channel
    pub connections:
        Arc<RwLock<HashMap<ConnectionId, tokio::sync::mpsc::UnboundedSender<Message>>>>,

    /// user_id string -> active connection count
    pub per_user_conn_count: Arc<RwLock<HashMap<String, u32>>>,
}

#[derive(Debug, Error)]
pub enum InitError {
    #[error("event sink init failed: {0}")]
    /// Variant `EventSink`.
    EventSink(String),

    #[error("auth init failed: {0}")]
    /// Variant `Auth`.
    Auth(String),
}

impl AppState {
    pub async fn initialize(config: RealtimeConfig) -> Result<Self, InitError> {
        let presence: Arc<dyn PresenceBackend> = Arc::new(InMemoryPresenceBackend::default());
        let rooms = Arc::new(RoomManager::new());

        let events = Arc::new(
            EventSink::new(config.service_name.clone(), config.region.clone())
                .await
                .map_err(|e| InitError::EventSink(e.to_string()))?,
        );

        // Zero-trust default:
        // - allow_anonymous=false => auth MUST be available
        // - allow_anonymous=true  => best-effort auth, continue if misconfigured
        let auth = if config.allow_anonymous {
            match build_auth_service(
                &config.service_name,
                &config.environment,
                config.auth_redis_url_override.clone(),
            )
            .await
            {
                Ok(svc) => Some(Arc::new(svc)),
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        "auth init failed; continuing because allow_anonymous=true"
                    );
                    None
                }
            }
        } else {
            let svc = build_auth_service(
                &config.service_name,
                &config.environment,
                config.auth_redis_url_override.clone(),
            )
            .await
            .map_err(|e| InitError::Auth(e.to_string()))?;
            Some(Arc::new(svc))
        };

        Ok(Self {
            config,
            auth,
            presence,
            rooms,
            events,
            connections: Arc::new(RwLock::new(HashMap::new())),
            per_user_conn_count: Arc::new(RwLock::new(HashMap::new())),
        })
    }
}
