#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConnectionId(Uuid);

impl ConnectionId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ConnectionId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ConnectionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // stable, log-friendly
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomEvent {
    /// Field `room`.
    pub room: String,
    /// Field `sender`.
    pub sender: String,
    /// Field `kind`.
    pub kind: String,
    /// Field `payload`.
    pub payload: serde_json::Value,
}

#[derive(Default)]
pub struct RoomManager {
    inner: Arc<RwLock<RoomsState>>,
}

#[derive(Default)]
struct RoomsState {
    rooms: HashMap<String, HashSet<ConnectionId>>,
}

impl RoomManager {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(RoomsState::default())),
        }
    }

    pub async fn join(&self, room: &str, conn_id: ConnectionId) {
        let mut inner = self.inner.write().await;
        inner
            .rooms
            .entry(room.to_string())
            .or_default()
            .insert(conn_id);
    }

    pub async fn leave(&self, room: &str, conn_id: ConnectionId) {
        let mut inner = self.inner.write().await;
        if let Some(set) = inner.rooms.get_mut(room) {
            set.remove(&conn_id);
            if set.is_empty() {
                inner.rooms.remove(room);
            }
        }
    }

    pub async fn members(&self, room: &str) -> Vec<ConnectionId> {
        let inner = self.inner.read().await;
        inner
            .rooms
            .get(room)
            .map(|set| set.iter().copied().collect())
            .unwrap_or_default()
    }
}
