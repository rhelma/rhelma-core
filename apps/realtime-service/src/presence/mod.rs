#![forbid(unsafe_code)]

use async_trait::async_trait;
use rhelma_core::prelude::{TenantId, UserId};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Presence {
    /// Field `user_id`.
    pub user_id: UserId,
    #[serde(default)]
    /// Field `tenant_id`.
    pub tenant_id: Option<TenantId>,
    /// Field `rooms`.
    pub rooms: Vec<String>,
}

#[async_trait]
pub trait PresenceBackend: Send + Sync {
    async fn update_presence(&self, presence: Presence);
    async fn clear_presence(&self, user_id: &UserId);
    async fn get_presence(&self, user_id: &UserId) -> Option<Presence>;
    async fn users_in_room(&self, room: &str) -> Vec<Presence>;
}

#[derive(Default)]
pub struct InMemoryPresenceBackend {
    inner: Arc<RwLock<PresenceState>>,
}

#[derive(Default)]
struct PresenceState {
    by_user: HashMap<UserId, Presence>,
    by_room: HashMap<String, HashSet<UserId>>,
}

#[async_trait]
impl PresenceBackend for InMemoryPresenceBackend {
    async fn update_presence(&self, presence: Presence) {
        let mut inner = self.inner.write().await;

        if let Some(old_rooms) = inner
            .by_user
            .get(&presence.user_id)
            .map(|p| p.rooms.clone())
        {
            for r in &old_rooms {
                if let Some(set) = inner.by_room.get_mut(r) {
                    set.remove(&presence.user_id);
                }
            }
        }

        for r in &presence.rooms {
            inner
                .by_room
                .entry(r.clone())
                .or_default()
                .insert(presence.user_id);
        }

        inner.by_user.insert(presence.user_id, presence);
    }

    async fn clear_presence(&self, user_id: &UserId) {
        let mut inner = self.inner.write().await;
        if let Some(old) = inner.by_user.remove(user_id) {
            for r in &old.rooms {
                if let Some(set) = inner.by_room.get_mut(r) {
                    set.remove(user_id);
                }
            }
        }
    }

    async fn get_presence(&self, user_id: &UserId) -> Option<Presence> {
        let inner = self.inner.read().await;
        inner.by_user.get(user_id).cloned()
    }

    async fn users_in_room(&self, room: &str) -> Vec<Presence> {
        let inner = self.inner.read().await;
        let Some(users) = inner.by_room.get(room) else {
            return vec![];
        };

        users
            .iter()
            .filter_map(|u| inner.by_user.get(u).cloned())
            .collect()
    }
}
