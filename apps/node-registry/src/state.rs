#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use tokio::sync::Mutex;

use crate::admission::{
    rate_limit::TtlCounter, redis_store::RedisAdmissionStore, AdmissionChallengeRecord,
};
use crate::config::NodeRegistryConfig;
use crate::error::RegistryError;
use crate::store::InMemoryNodeStore;

pub enum AdmissionBackend {
    /// In-memory challenges and rate-limit counters (default).
    Memory(Mutex<AdmissionState>),
    /// Redis-backed challenges and rate-limit counters.
    Redis(RedisAdmissionStore),
}

pub struct AdmissionState {
    pub register_counter: TtlCounter,
    pub challenges: HashMap<String, AdmissionChallengeRecord>,
}

impl AdmissionState {
    pub fn new() -> Self {
        Self {
            register_counter: TtlCounter::default(),
            challenges: HashMap::new(),
        }
    }

    pub fn prune_expired(&mut self, now_unix: i64) {
        self.register_counter.prune();
        self.challenges.retain(|_, rec| !rec.is_expired(now_unix));
    }
}

pub struct AppState {
    pub cfg: NodeRegistryConfig,
    pub store: InMemoryNodeStore,
    pub admission: AdmissionBackend,
    #[allow(dead_code)]
    pub start_time: Instant,
}

impl AppState {
    pub async fn new(cfg: NodeRegistryConfig) -> Result<Self, RegistryError> {
        let store = InMemoryNodeStore::new(cfg.tuning.max_nodes);

        let admission = match cfg
            .admission
            .redis_url
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            Some(redis_url) => {
                let prefix = cfg.admission.redis_prefix.clone();
                let store = RedisAdmissionStore::connect(
                    redis_url,
                    prefix,
                    cfg.admission.pow_challenge_ttl,
                    cfg.admission.register_rate_limit_ttl,
                    cfg.admission.register_rate_limit_max,
                )
                .await
                .map_err(RegistryError::config)?;
                AdmissionBackend::Redis(store)
            }
            None => AdmissionBackend::Memory(Mutex::new(AdmissionState::new())),
        };

        Ok(Self {
            cfg,
            store,
            admission,
            start_time: Instant::now(),
        })
    }
}

/// Shared app state used by route handlers.
pub type SharedState = Arc<AppState>;
