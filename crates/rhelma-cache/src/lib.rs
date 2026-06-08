//! Rhelma Cache - High-performance caching layer for Rhelma platform.
//!
//! This crate provides memory, redis, and layered cache backends along with a
//! high-level `CacheService` facade plus small utilities for metrics and tracing.

#![forbid(unsafe_code)]

/// Cache metrics helpers.
pub mod metrics;
/// Tracing helpers for cache operations.
pub mod tracing_ext;

/// Cache backend implementations (memory, Redis, layered).
pub mod backends;
/// Cache configuration types.
pub mod config;
/// Cache error types.
pub mod error;
/// Prelude re-exports for convenient use.
pub mod prelude;
/// Core cache types.
pub mod types;

mod macros;

pub use config::*;
pub use error::*;
pub use types::*;

use std::sync::Arc;
use std::time::Duration;

// Import types from backends module
use backends::CacheBackend;
use backends::{LayeredCache, MemoryCache, RedisCache};

/// Enum for different cache backends.
#[derive(Clone)]
pub enum CacheBackendImpl {
    /// In-memory backend.
    Memory(MemoryCache),
    /// Redis backend.
    Redis(RedisCache),
    /// Layered backend (memory + redis).
    Layered(LayeredCache),
}

impl CacheBackendImpl {
    async fn get<T>(&self, key: &str) -> CacheResult<Option<T>>
    where
        T: serde::de::DeserializeOwned + serde::Serialize + Send + Sync + Clone + 'static,
    {
        match self {
            Self::Memory(cache) => cache.get(key).await,
            Self::Redis(cache) => cache.get(key).await,
            Self::Layered(cache) => cache.get(key).await,
        }
    }

    async fn set<T>(&self, key: &str, value: &T, ttl: Option<Duration>) -> CacheResult<()>
    where
        T: serde::Serialize + Send + Sync,
    {
        match self {
            Self::Memory(cache) => cache.set(key, value, ttl).await,
            Self::Redis(cache) => cache.set(key, value, ttl).await,
            Self::Layered(cache) => cache.set(key, value, ttl).await,
        }
    }

    async fn delete(&self, key: &str) -> CacheResult<()> {
        match self {
            Self::Memory(cache) => cache.delete(key).await,
            Self::Redis(cache) => cache.delete(key).await,
            Self::Layered(cache) => cache.delete(key).await,
        }
    }

    async fn exists(&self, key: &str) -> CacheResult<bool> {
        match self {
            Self::Memory(cache) => cache.exists(key).await,
            Self::Redis(cache) => cache.exists(key).await,
            Self::Layered(cache) => cache.exists(key).await,
        }
    }

    async fn increment(&self, key: &str, value: i64) -> CacheResult<i64> {
        match self {
            Self::Memory(cache) => cache.increment(key, value).await,
            Self::Redis(cache) => cache.increment(key, value).await,
            Self::Layered(cache) => cache.increment(key, value).await,
        }
    }

    async fn clear(&self) -> CacheResult<()> {
        match self {
            Self::Memory(cache) => cache.clear().await,
            Self::Redis(cache) => cache.clear().await,
            Self::Layered(cache) => cache.clear().await,
        }
    }

    async fn health_check(&self) -> CacheResult<bool> {
        match self {
            Self::Memory(cache) => cache.health_check().await,
            Self::Redis(cache) => cache.health_check().await,
            Self::Layered(cache) => cache.health_check().await,
        }
    }
}

/// Main cache service.
#[derive(Clone)]
pub struct CacheService {
    backend: CacheBackendImpl,
    config: Arc<CacheConfig>,
}

impl CacheService {
    /// Create a new cache service from a backend and config.
    #[must_use]
    pub fn new(backend: CacheBackendImpl, config: CacheConfig) -> Self {
        Self {
            backend,
            config: Arc::new(config),
        }
    }

    /// Return the backend kind for metrics/tracing.
    #[must_use]
    pub fn backend_kind(&self) -> crate::metrics::CacheBackendKind {
        match &self.backend {
            CacheBackendImpl::Memory(_) => crate::metrics::CacheBackendKind::Memory,
            CacheBackendImpl::Redis(_) => crate::metrics::CacheBackendKind::Redis,
            CacheBackendImpl::Layered(_) => crate::metrics::CacheBackendKind::Layered,
        }
    }

    /// Get a value from the cache.
    ///
    /// # Errors
    /// Returns [`CacheError`] if the backend fails to read or the value cannot be deserialized.
    pub async fn get<T>(&self, key: &str) -> CacheResult<Option<T>>
    where
        T: serde::de::DeserializeOwned + serde::Serialize + Send + Sync + Clone + 'static,
    {
        self.backend.get(key).await
    }

    /// Set a value in the cache.
    ///
    /// # Errors
    /// Returns [`CacheError`] if the backend fails to write or the value cannot be serialized.
    pub async fn set<T>(&self, key: &str, value: &T, ttl: Option<Duration>) -> CacheResult<()>
    where
        T: serde::Serialize + Send + Sync,
    {
        // If ttl is not provided, use default ttl
        let ttl = ttl.or(Some(self.config.default_ttl));
        self.backend.set(key, value, ttl).await
    }

    /// Delete a key from the cache.
    ///
    /// # Errors
    /// Returns [`CacheError`] if the backend fails to delete the key.
    pub async fn delete(&self, key: &str) -> CacheResult<()> {
        self.backend.delete(key).await
    }

    /// Check if a key exists in the cache.
    ///
    /// # Errors
    /// Returns [`CacheError`] if the backend fails to check key existence.
    pub async fn exists(&self, key: &str) -> CacheResult<bool> {
        self.backend.exists(key).await
    }

    /// Increment a numeric value in the cache.
    ///
    /// # Errors
    /// Returns [`CacheError`] if the backend fails to increment the value.
    pub async fn increment(&self, key: &str, value: i64) -> CacheResult<i64> {
        self.backend.increment(key, value).await
    }

    /// Clear all cache entries.
    ///
    /// # Errors
    /// Returns [`CacheError`] if the backend fails to clear entries.
    pub async fn clear(&self) -> CacheResult<()> {
        self.backend.clear().await
    }

    /// Health check the cache backend.
    ///
    /// # Errors
    /// Returns [`CacheError`] if the backend health probe fails.
    pub async fn health_check(&self) -> CacheResult<bool> {
        self.backend.health_check().await
    }

    /// Get multiple values from the cache.
    ///
    /// # Errors
    /// Returns [`CacheError`] if any underlying `get` operation fails.
    pub async fn mget<T>(&self, keys: &[&str]) -> CacheResult<Vec<Option<T>>>
    where
        T: serde::de::DeserializeOwned + serde::Serialize + Send + Sync + Clone + 'static,
    {
        let mut result = Vec::with_capacity(keys.len());
        for key in keys {
            result.push(self.get::<T>(key).await?);
        }
        Ok(result)
    }

    /// Set multiple values in the cache.
    ///
    /// # Errors
    /// Returns [`CacheError`] if any underlying `set` operation fails.
    pub async fn mset<T>(&self, entries: &[(&str, &T)], ttl: Option<Duration>) -> CacheResult<()>
    where
        T: serde::Serialize + Send + Sync,
    {
        for (key, value) in entries {
            self.set(key, *value, ttl).await?;
        }
        Ok(())
    }

    /// Create memory cache service.
    #[must_use]
    pub fn memory(config: CacheConfig) -> Self {
        let memory_cache = MemoryCache::new(config.memory.max_capacity);
        Self::new(CacheBackendImpl::Memory(memory_cache), config)
    }

    /// Create redis cache service (legacy constructor).
    ///
    /// # Errors
    /// Returns [`CacheError`] if the Redis backend cannot be initialized.
    pub fn redis(config: CacheConfig, redis_url: &str, key_prefix: String) -> CacheResult<Self> {
        let redis_cache = RedisCache::new(redis_url)?.with_prefix(key_prefix);
        Ok(Self::new(CacheBackendImpl::Redis(redis_cache), config))
    }

    /// Create redis cache service from `CacheConfig.redis`.
    ///
    /// # Errors
    /// Returns [`CacheError`] if the Redis config is missing or the Redis backend cannot be initialized.
    pub fn redis_from_config(config: CacheConfig) -> CacheResult<Self> {
        let redis_cfg = config
            .redis
            .clone()
            .ok_or_else(|| CacheError::Config("redis config is missing".to_string()))?;
        let redis_cache = RedisCache::new(&redis_cfg.url)?.with_prefix(redis_cfg.key_prefix);
        Ok(Self::new(CacheBackendImpl::Redis(redis_cache), config))
    }

    /// Create layered cache service (legacy constructor).
    ///
    /// # Errors
    /// Returns [`CacheError`] if the Redis backend cannot be initialized.
    pub fn layered(config: CacheConfig, redis_url: &str, key_prefix: String) -> CacheResult<Self> {
        let memory_cache = MemoryCache::new(config.memory.max_capacity);
        let redis_cache = RedisCache::new(redis_url)?.with_prefix(key_prefix);
        let layered_cache = LayeredCache::new(memory_cache, redis_cache, config.layered.clone());
        Ok(Self::new(CacheBackendImpl::Layered(layered_cache), config))
    }

    /// Create layered cache service from `CacheConfig.redis`.
    ///
    /// # Errors
    /// Returns [`CacheError`] if the Redis config is missing or backends cannot be initialized.
    pub fn layered_from_config(config: CacheConfig) -> CacheResult<Self> {
        let redis_cfg = config
            .redis
            .clone()
            .ok_or_else(|| CacheError::Config("redis config is missing".to_string()))?;
        let memory_cache = MemoryCache::new(config.memory.max_capacity);
        let redis_cache = RedisCache::new(&redis_cfg.url)?.with_prefix(redis_cfg.key_prefix);
        let layered_cache = LayeredCache::new(memory_cache, redis_cache, config.layered.clone());
        Ok(Self::new(CacheBackendImpl::Layered(layered_cache), config))
    }
}

/// Cache result type.
pub type CacheResult<T> = std::result::Result<T, CacheError>;
