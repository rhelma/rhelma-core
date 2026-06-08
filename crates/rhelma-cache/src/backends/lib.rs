//! Rhelma Cache - High-performance caching layer for Rhelma platform

#![forbid(unsafe_code)]

pub mod backends;
pub mod config;
pub mod error;
pub mod prelude;
pub mod types;

mod macros;

pub use backends::*;
pub use config::*;
pub use error::*;

use std::time::Duration;

/// Main cache service
#[derive(Clone)]
pub struct CacheService {
    backend: Box<dyn backends::CacheBackend>,
    config: Arc<config::CacheConfig>,
}

impl CacheService {
    pub fn new(backend: Box<dyn backends::CacheBackend>, config: config::CacheConfig) -> Self {
        Self {
            backend,
            config: Arc::new(config),
        }
    }

    pub async fn get<T>(&self, key: &str) -> Result<Option<T>, CacheError>
    where
        T: serde::de::DeserializeOwned + Send + Sync,
    {
        self.backend.get(key).await
    }

    pub async fn set<T>(&self, key: &str, value: &T, ttl: Option<Duration>) -> Result<(), CacheError>
    where
        T: serde::Serialize + Send + Sync,
    {
        self.backend.set(key, value, ttl).await
    }

    pub async fn delete(&self, key: &str) -> Result<(), CacheError> {
        self.backend.delete(key).await
    }

    pub async fn exists(&self, key: &str) -> Result<bool, CacheError> {
        self.backend.exists(key).await
    }

    pub async fn increment(&self, key: &str, value: i64) -> Result<i64, CacheError> {
        self.backend.increment(key, value).await
    }

    pub async fn health_check(&self) -> Result<bool, CacheError> {
        self.backend.health_check().await
    }
}

/// Cache result type
pub type CacheResult<T> = std::result::Result<T, CacheError>;

/// Prelude for easy imports
pub mod prelude {
    pub use super::{
        CacheService, CacheResult, CacheError,
        backends::*, config::*,
	resilience::*,
    };
    pub use rhelma_core::prelude::*;
}
