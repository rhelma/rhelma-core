#![forbid(unsafe_code)]

//! Layered cache backend.
//!
//! A two-layer cache combining:
//! - **L1**: in-process [`MemoryCache`] for low-latency hot keys
//! - **L2**: [`RedisCache`] for shared, cross-instance caching
//!
//! Production hardening:
//! - Request coalescing (anti thundering herd) with a per-key semaphore
//! - Synchronous L1 backfill while holding the permit
//! - Best-effort cleanup to avoid unbounded inflight growth

use crate::CacheResult;
use dashmap::DashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

/// A two-level cache (L1 memory + L2 redis) with request coalescing.
#[derive(Clone)]
pub struct LayeredCache {
    memory: super::MemoryCache,
    redis: super::RedisCache,
    config: crate::config::LayeredCacheConfig,
    inflight: Arc<DashMap<String, Arc<Semaphore>>>,
}

impl LayeredCache {
    /// Create a new layered cache from an L1 memory cache and an L2 redis cache.
    #[must_use]
    pub fn new(
        memory: super::MemoryCache,
        redis: super::RedisCache,
        config: crate::config::LayeredCacheConfig,
    ) -> Self {
        Self {
            memory,
            redis,
            config,
            inflight: Arc::new(DashMap::new()),
        }
    }

    fn try_cleanup_inflight(&self, key: &str, sem: &Arc<Semaphore>) {
        // When nobody else is holding a reference except the map + this scope,
        // remove the semaphore entry to keep the map bounded.
        if Arc::strong_count(sem) == 2 {
            self.inflight.remove(key);
        }
    }
}

#[async_trait::async_trait]
impl super::CacheBackend for LayeredCache {
    async fn get<T>(&self, key: &str) -> CacheResult<Option<T>>
    where
        T: serde::de::DeserializeOwned + serde::Serialize + Send + Sync + Clone + 'static,
    {
        // 1) L1 fast path
        if let Some(value) = self.memory.get::<T>(key).await? {
            return Ok(Some(value));
        }

        // If layered caching is disabled, fall back to Redis only.
        if !self.config.enabled {
            return self.redis.get::<T>(key).await;
        }

        // 2) Coalesce concurrent L2 fetches for this key.
        let sem = self
            .inflight
            .entry(key.to_string())
            .or_insert_with(|| Arc::new(Semaphore::new(1)))
            .clone();

        let _permit = sem.acquire().await.expect("semaphore closed");

        // 3) Double-check after waiting (another task may have backfilled).
        if let Some(value) = self.memory.get::<T>(key).await? {
            self.try_cleanup_inflight(key, &sem);
            return Ok(Some(value));
        }

        // 4) Fetch from Redis
        let value = self.redis.get::<T>(key).await?;

        // 5) Backfill L1 synchronously while holding the permit.
        if let Some(ref v) = value {
            // Keep current behavior: 300s backfill TTL.
            // (If you want a policy-driven TTL, extend LayeredCacheConfig later.)
            let _ = self
                .memory
                .set(key, v, Some(Duration::from_secs(300)))
                .await;
        }

        self.try_cleanup_inflight(key, &sem);
        Ok(value)
    }

    async fn set<T>(&self, key: &str, value: &T, ttl: Option<Duration>) -> CacheResult<()>
    where
        T: serde::Serialize + Send + Sync,
    {
        let redis_future = self.redis.set(key, value, ttl);
        let memory_future = self.memory.set(key, value, ttl);

        let (redis_res, memory_res): (CacheResult<()>, CacheResult<()>) =
            tokio::join!(redis_future, memory_future);

        // Prefer redis error (shared cache) over memory error (best-effort).
        redis_res?;
        memory_res?;
        Ok(())
    }

    async fn delete(&self, key: &str) -> CacheResult<()> {
        let redis_future = self.redis.delete(key);
        let memory_future = self.memory.delete(key);
        let (redis_res, memory_res): (CacheResult<()>, CacheResult<()>) =
            tokio::join!(redis_future, memory_future);
        redis_res?;
        memory_res?;
        Ok(())
    }

    async fn exists(&self, key: &str) -> CacheResult<bool> {
        if self.memory.exists(key).await? {
            return Ok(true);
        }
        if !self.config.enabled {
            return self.redis.exists(key).await;
        }
        self.redis.exists(key).await
    }

    async fn increment(&self, key: &str, value: i64) -> CacheResult<i64> {
        // Increment in Redis (source of truth) then update L1.
        let new_value = self.redis.increment(key, value).await?;
        let _ = self.memory.set(key, &new_value, None).await;
        Ok(new_value)
    }

    async fn clear(&self) -> CacheResult<()> {
        let redis_future = self.redis.clear();
        let memory_future = self.memory.clear();
        let (redis_res, memory_res): (CacheResult<()>, CacheResult<()>) =
            tokio::join!(redis_future, memory_future);
        redis_res?;
        memory_res?;
        Ok(())
    }

    async fn health_check(&self) -> CacheResult<bool> {
        // Both layers should be healthy for the layered backend to be considered healthy.
        let redis_ok = self.redis.health_check().await?;
        let mem_ok = self.memory.health_check().await?;
        Ok(redis_ok && mem_ok)
    }
}
