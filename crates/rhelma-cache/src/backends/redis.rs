#![forbid(unsafe_code)]

use std::time::Duration;

use async_trait::async_trait;
use redis::Commands;
use serde::{de::DeserializeOwned, Serialize};

use crate::{CacheBackend, CacheError, CacheResult};

/// Redis-backed cache backend.
///
/// Notes:
/// - Values are serialized as JSON bytes using `serde_json`.
/// - This backend uses `tokio::task::spawn_blocking` for the synchronous Redis client.
#[derive(Debug, Clone)]
pub struct RedisCache {
    client: redis::Client,
    key_prefix: Option<String>,
}

impl RedisCache {
    /// Create a new Redis cache backend.
    ///
    /// `redis_url` example: `redis://127.0.0.1/`
    ///
    /// # Errors
    ///
    /// Returns an error if the Redis client cannot be created from `redis_url`.
    pub fn new(redis_url: &str) -> CacheResult<Self> {
        let client =
            redis::Client::open(redis_url).map_err(|e| CacheError::Connection(e.to_string()))?;
        Ok(Self {
            client,
            key_prefix: None,
        })
    }

    /// Set a prefix for all keys stored by this backend.
    #[must_use]
    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.key_prefix = Some(prefix.into());
        self
    }

    #[must_use]
    fn full_key(&self, key: &str) -> String {
        match &self.key_prefix {
            Some(prefix) => format!("{prefix}{key}"),
            None => key.to_string(),
        }
    }

    fn serialize<T: Serialize>(value: &T) -> CacheResult<Vec<u8>> {
        serde_json::to_vec(value).map_err(|e| CacheError::Serialization(e.to_string()))
    }

    fn deserialize<T: DeserializeOwned>(bytes: &[u8]) -> CacheResult<T> {
        serde_json::from_slice(bytes).map_err(|e| CacheError::Deserialization(e.to_string()))
    }

    async fn with_connection<R, F>(&self, op: F) -> CacheResult<R>
    where
        R: Send + 'static,
        F: FnOnce(&mut redis::Connection) -> CacheResult<R> + Send + 'static,
    {
        let client = self.client.clone();
        tokio::task::spawn_blocking(move || {
            let mut con = client
                .get_connection()
                .map_err(|e| CacheError::Connection(e.to_string()))?;
            op(&mut con)
        })
        .await
        .map_err(|e| CacheError::Backend(format!("redis task join error: {e}")))?
    }
}

#[async_trait]
impl CacheBackend for RedisCache {
    async fn get<T>(&self, key: &str) -> CacheResult<Option<T>>
    where
        T: DeserializeOwned + Send + Sync,
    {
        let k = self.full_key(key);

        let bytes: Option<Vec<u8>> = self
            .with_connection(move |con| con.get(&k).map_err(|e| CacheError::Backend(e.to_string())))
            .await?;

        match bytes {
            Some(bytes) => Self::deserialize::<T>(&bytes).map(Some),
            None => Ok(None),
        }
    }

    async fn set<T>(&self, key: &str, value: &T, ttl: Option<Duration>) -> CacheResult<()>
    where
        T: Serialize + Send + Sync,
    {
        let k = self.full_key(key);
        let bytes = Self::serialize(value)?;

        self.with_connection(move |con| {
            if let Some(ttl) = ttl {
                let secs = ttl.as_secs();
                let _: () = con
                    .set_ex(&k, bytes.as_slice(), secs)
                    .map_err(|e| CacheError::Backend(e.to_string()))?;
            } else {
                let _: () = con
                    .set(&k, bytes.as_slice())
                    .map_err(|e| CacheError::Backend(e.to_string()))?;
            }

            Ok(())
        })
        .await
    }

    async fn delete(&self, key: &str) -> CacheResult<()> {
        let k = self.full_key(key);
        self.with_connection(move |con| {
            let _: () = con
                .del(&k)
                .map_err(|e| CacheError::Backend(e.to_string()))?;
            Ok(())
        })
        .await
    }

    async fn exists(&self, key: &str) -> CacheResult<bool> {
        let k = self.full_key(key);
        self.with_connection(move |con| {
            con.exists(&k)
                .map_err(|e| CacheError::Backend(e.to_string()))
        })
        .await
    }

    async fn increment(&self, key: &str, value: i64) -> CacheResult<i64> {
        let k = self.full_key(key);
        self.with_connection(move |con| {
            con.incr::<_, _, i64>(&k, value)
                .map_err(|e| CacheError::Backend(e.to_string()))
        })
        .await
    }

    async fn clear(&self) -> CacheResult<()> {
        let prefix = self.key_prefix.clone().unwrap_or_default();

        self.with_connection(move |con| {
            let pattern = if prefix.is_empty() {
                "*".to_string()
            } else {
                format!("{prefix}*")
            };

            let keys: Vec<String> = con
                .keys(pattern)
                .map_err(|e| CacheError::Backend(e.to_string()))?;

            if keys.is_empty() {
                return Ok(());
            }

            let _: () = con
                .del(keys)
                .map_err(|e| CacheError::Backend(e.to_string()))?;
            Ok(())
        })
        .await
    }

    async fn health_check(&self) -> CacheResult<bool> {
        self.with_connection(|con| {
            let pong: String = con.ping().map_err(|e| CacheError::Backend(e.to_string()))?;
            Ok(pong == "PONG")
        })
        .await
    }
}
