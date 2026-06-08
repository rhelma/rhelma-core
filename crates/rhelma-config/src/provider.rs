//! Provider traits for rhelma-config.

use std::time::{Duration, Instant};

use async_trait::async_trait;
use serde_json::Value;

use crate::errors::ConfigResult;

/// Synchronous configuration provider.
///
/// Suitable for tests, in-memory and simple JSON/file-based use cases.
pub trait SyncConfigProvider {
    /// fn `load_defaults`.
    fn load_defaults(&self) -> ConfigResult<Option<Value>>;
    /// fn `load_region_config`.
    fn load_region_config(&self, _region: &str) -> ConfigResult<Option<Value>> {
        Ok(None)
    }
    /// fn `load_service_config`.
    fn load_service_config(&self, _region: &str, _service: &str) -> ConfigResult<Option<Value>> {
        Ok(None)
    }
}

/// Asynchronous configuration provider.
///
/// Suitable for networked / database-backed config services.
#[async_trait]
pub trait AsyncConfigProvider: Send + Sync {
    async fn load_defaults(&self) -> ConfigResult<Option<Value>>;

    async fn load_region_config(&self, _region: &str) -> ConfigResult<Option<Value>> {
        Ok(None)
    }

    async fn load_service_config(
        &self,
        _region: &str,
        _service: &str,
    ) -> ConfigResult<Option<Value>> {
        Ok(None)
    }
}

#[derive(Debug, Clone)]
struct CacheEntry {
    value: Value,
    inserted_at: Instant,
}

// -----------------------------------------------------------------------------
// Cache backend (feature-gated)
// -----------------------------------------------------------------------------

#[cfg(not(feature = "dashmap-cache"))]
mod cache_backend {
    use super::CacheEntry;
    use std::collections::HashMap;
    use std::sync::{Arc, RwLock};

    #[derive(Clone)]
    pub(super) struct CacheStore {
        inner: Arc<RwLock<HashMap<String, CacheEntry>>>,
    }

    impl CacheStore {
        pub(super) fn new() -> Self {
            Self {
                inner: Arc::new(RwLock::new(HashMap::new())),
            }
        }

        pub(super) fn get(&self, key: &str) -> Option<CacheEntry> {
            let r = self.inner.read().ok()?;
            r.get(key).cloned()
        }

        pub(super) fn insert(&self, key: String, entry: CacheEntry) {
            if let Ok(mut w) = self.inner.write() {
                w.insert(key, entry);
            }
        }

        pub(super) fn remove(&self, key: &str) {
            if let Ok(mut w) = self.inner.write() {
                w.remove(key);
            }
        }

        /// Remove a key only if it satisfies the predicate.
        ///
        /// This keeps `get` + `remove` atomic under the same write lock, avoiding
        /// a small race where another thread could refresh the key between the two.
        pub(super) fn remove_if<F>(&self, key: &str, should_remove: F)
        where
            F: FnOnce(&CacheEntry) -> bool,
        {
            if let Ok(mut w) = self.inner.write() {
                if let Some(entry) = w.get(key) {
                    if should_remove(entry) {
                        w.remove(key);
                    }
                }
            }
        }

        pub(super) fn clear(&self) {
            if let Ok(mut w) = self.inner.write() {
                w.clear();
            }
        }
    }
}

#[cfg(feature = "dashmap-cache")]
mod cache_backend {
    use super::CacheEntry;
    use dashmap::DashMap;
    use std::sync::Arc;
    use std::time::Instant;

    #[derive(Clone)]
    pub(super) struct CacheStore {
        inner: Arc<DashMap<String, CacheEntry>>,
    }

    impl CacheStore {
        pub(super) fn new() -> Self {
            Self {
                inner: Arc::new(DashMap::new()),
            }
        }

        pub(super) fn get(&self, key: &str) -> Option<CacheEntry> {
            self.inner.get(key).map(|e| e.value().clone())
        }

        pub(super) fn insert(&self, key: String, entry: CacheEntry) {
            self.inner.insert(key, entry);
        }

        pub(super) fn remove(&self, key: &str) {
            self.inner.remove(key);
        }

        /// Remove a key only if the cached entry's insertion timestamp matches `expected`.
        ///
        /// This is a best-effort mitigation for a small race window: another thread may
        /// refresh the same key between a stale read and a remove attempt. If that happens,
        /// we restore the removed value *only if* the key is currently absent, avoiding
        /// overwriting a newer refresh.
        pub(super) fn remove_if_inserted_at(&self, key: &str, expected: Instant) {
            if let Some((k, removed)) = self.inner.remove(key) {
                if removed.inserted_at != expected {
                    use dashmap::mapref::entry::Entry;
                    match self.inner.entry(k) {
                        Entry::Occupied(_) => {
                            // A newer value exists; keep it.
                        }
                        Entry::Vacant(e) => {
                            e.insert(removed);
                        }
                    }
                }
            }
        }

        pub(super) fn clear(&self) {
            self.inner.clear();
        }
    }
}

use cache_backend::CacheStore;

/// Simple caching decorator for async providers.
///
/// Caches JSON layers (defaults / region / service) in-memory.
///
/// Notes:
/// - Caching is best-effort and purely in-process.
/// - You can opt into TTL-based expiry using [`CachedProvider::new_with_ttl`].
pub struct CachedProvider<P> {
    inner: P,
    cache: CacheStore,
    ttl: Option<Duration>,
}

impl<P> CachedProvider<P> {
    /// Create a cached provider with no TTL (entries live for the process lifetime).
    pub fn new(inner: P) -> Self {
        Self {
            inner,
            cache: CacheStore::new(),
            ttl: None,
        }
    }

    /// Create a cached provider with a TTL.
    ///
    /// When TTL elapses, the next read triggers a refresh from the inner provider.
    pub fn new_with_ttl(inner: P, ttl: Duration) -> Self {
        Self {
            inner,
            cache: CacheStore::new(),
            ttl: Some(ttl),
        }
    }

    /// Invalidate all cached entries.
    pub fn invalidate_all(&self) {
        self.cache.clear();
    }

    /// Invalidate a specific cache key.
    pub fn invalidate_key(&self, key: &str) {
        self.cache.remove(key);
    }

    fn is_fresh(&self, entry: &CacheEntry) -> bool {
        match self.ttl {
            None => true,
            Some(ttl) => entry.inserted_at.elapsed() <= ttl,
        }
    }

    fn key_defaults() -> String {
        "defaults".into()
    }

    fn key_region(region: &str) -> String {
        format!("region:{region}")
    }

    fn key_service(region: &str, service: &str) -> String {
        format!("service:{region}:{service}")
    }

    fn get_if_fresh(&self, key: &str) -> Option<Value> {
        let entry = self.cache.get(key)?;
        if self.is_fresh(&entry) {
            Some(entry.value)
        } else {
            None
        }
    }

    fn evict_if_expired(&self, key: &str) {
        if self.ttl.is_none() {
            return;
        }

        #[cfg(not(feature = "dashmap-cache"))]
        {
            // Under the RwLock backend we can perform the freshness check and the
            // eviction under the same write lock to avoid a small race.
            self.cache.remove_if(key, |entry| !self.is_fresh(entry));
        }

        #[cfg(feature = "dashmap-cache")]
        {
            // Under DashMap, refreshes can race with eviction. We mitigate this by
            // removing only if the insertion timestamp matches what we observed.
            if let Some(entry) = self.cache.get(key) {
                if !self.is_fresh(&entry) {
                    self.cache.remove_if_inserted_at(key, entry.inserted_at);
                }
            }
        }
    }
}

#[async_trait]
impl<P> AsyncConfigProvider for CachedProvider<P>
where
    P: AsyncConfigProvider + Send + Sync,
{
    async fn load_defaults(&self) -> ConfigResult<Option<Value>> {
        let key = Self::key_defaults();

        // Fast path: read lock.
        if let Some(cached) = self.get_if_fresh(&key) {
            return Ok(Some(cached));
        }
        self.evict_if_expired(&key);

        // Load from inner.
        let loaded = self.inner.load_defaults().await?;
        if let Some(ref v) = loaded {
            self.cache.insert(
                key,
                CacheEntry {
                    value: v.clone(),
                    inserted_at: Instant::now(),
                },
            );
        }
        Ok(loaded)
    }

    async fn load_region_config(&self, region: &str) -> ConfigResult<Option<Value>> {
        let key = Self::key_region(region);

        if let Some(cached) = self.get_if_fresh(&key) {
            return Ok(Some(cached));
        }
        self.evict_if_expired(&key);

        let loaded = self.inner.load_region_config(region).await?;
        if let Some(ref v) = loaded {
            self.cache.insert(
                key,
                CacheEntry {
                    value: v.clone(),
                    inserted_at: Instant::now(),
                },
            );
        }
        Ok(loaded)
    }

    async fn load_service_config(
        &self,
        region: &str,
        service: &str,
    ) -> ConfigResult<Option<Value>> {
        let key = Self::key_service(region, service);

        if let Some(cached) = self.get_if_fresh(&key) {
            return Ok(Some(cached));
        }
        self.evict_if_expired(&key);

        let loaded = self.inner.load_service_config(region, service).await?;
        if let Some(ref v) = loaded {
            self.cache.insert(
                key,
                CacheEntry {
                    value: v.clone(),
                    inserted_at: Instant::now(),
                },
            );
        }
        Ok(loaded)
    }
}
