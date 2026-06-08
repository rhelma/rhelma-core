#![forbid(unsafe_code)]

//! In-memory LRU cache backend.
//!
//! This backend is designed as a fast L1 cache:
//! - Values are stored as JSON strings (serde) in an LRU map
//! - Optional TTL is enforced on `get`/`exists`
//! - Expired entries are eagerly removed
//!
//! Production hardening:
//! - TTL is enforced consistently across get/exists
//! - Expired entries are eagerly removed to avoid unbounded growth
//! - LRU recency is updated on get (uses `LruCache::get`)

use crate::CacheError;
use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// An in-process LRU cache suitable for hot-path reads.
#[derive(Clone)]
pub struct MemoryCache {
    inner: Arc<RwLock<LruCache<String, CacheEntry>>>,
}

struct CacheEntry {
    value: String,
    expires_at: Option<Instant>,
}

impl CacheEntry {
    fn is_expired(&self) -> bool {
        self.expires_at.is_some_and(|t| t <= Instant::now())
    }
}

impl MemoryCache {
    /// Create a new in-memory cache with an LRU capacity.
    ///
    /// A capacity of `0` is clamped to `1` to keep the backend usable in tests.
    #[must_use]
    pub fn new(max_capacity: usize) -> Self {
        let capacity = NonZeroUsize::new(max_capacity).unwrap_or(NonZeroUsize::MIN);
        Self {
            inner: Arc::new(RwLock::new(LruCache::new(capacity))),
        }
    }
}

#[async_trait::async_trait]
impl super::CacheBackend for MemoryCache {
    async fn get<T>(&self, key: &str) -> Result<Option<T>, CacheError>
    where
        T: serde::de::DeserializeOwned + serde::Serialize + Send + Sync + Clone + 'static,
    {
        // Write-lock because `LruCache::get` updates recency.
        let mut cache = self.inner.write().await;

        let Some(entry) = cache.get(key) else {
            return Ok(None);
        };

        if entry.is_expired() {
            cache.pop(key);
            return Ok(None);
        }

        let value: T = serde_json::from_str(&entry.value)
            .map_err(|e| CacheError::Deserialization(e.to_string()))?;
        Ok(Some(value))
    }

    async fn set<T>(&self, key: &str, value: &T, ttl: Option<Duration>) -> Result<(), CacheError>
    where
        T: serde::Serialize + Send + Sync,
    {
        let value_str =
            serde_json::to_string(value).map_err(|e| CacheError::Serialization(e.to_string()))?;

        let expires_at = ttl.map(|t| Instant::now() + t);

        let entry = CacheEntry {
            value: value_str,
            expires_at,
        };

        let mut cache = self.inner.write().await;
        cache.put(key.to_string(), entry);
        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<(), CacheError> {
        let mut cache = self.inner.write().await;
        cache.pop(key);
        Ok(())
    }

    async fn exists(&self, key: &str) -> Result<bool, CacheError> {
        // `exists` should NOT refresh recency, but must be TTL-aware.
        let mut cache = self.inner.write().await;
        match cache.peek(key) {
            None => Ok(false),
            Some(entry) if entry.is_expired() => {
                cache.pop(key);
                Ok(false)
            }
            Some(_) => Ok(true),
        }
    }

    async fn increment(&self, key: &str, value: i64) -> Result<i64, CacheError> {
        let mut cache = self.inner.write().await;

        let current: i64 = if let Some(entry) = cache.get(key) {
            // Ignore parse error => treat as 0.
            serde_json::from_str(&entry.value).unwrap_or(0)
        } else {
            0
        };

        let new_value = current + value;
        let new_entry = CacheEntry {
            value: serde_json::to_string(&new_value).unwrap_or_else(|_| "0".to_string()),
            expires_at: None,
        };

        cache.put(key.to_string(), new_entry);
        Ok(new_value)
    }

    async fn clear(&self) -> Result<(), CacheError> {
        let mut cache = self.inner.write().await;
        cache.clear();
        Ok(())
    }

    async fn health_check(&self) -> Result<bool, CacheError> {
        Ok(true)
    }
}
