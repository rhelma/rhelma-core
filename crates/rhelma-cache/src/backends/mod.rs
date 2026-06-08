#![forbid(unsafe_code)]

//! Cache backends.
//!
//! `rhelma-cache` provides a minimal, async, serde-friendly abstraction over
//! multiple caching backends.
//!
//! Design notes:
//! - The trait is intentionally small to keep implementations straightforward.
//! - Values are serialized/deserialized via `serde_json` for portability.
//! - Backends are expected to be **best-effort** and return [`CacheError`]
//!   without panicking.

mod layered;
mod memory;
/// Redis cache backend.
pub mod redis;

// Export the types.
pub use layered::LayeredCache;
pub use memory::MemoryCache;
pub use redis::RedisCache;

use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};
use std::time::Duration;

use crate::CacheError;

/// Async cache backend interface.
///
/// Implementations must be thread-safe (`Send + Sync`).
///
/// The generic `get`/`set` use serde serialization so callers can store
/// structured values without hand-encoding.
#[async_trait]
pub trait CacheBackend: Send + Sync {
    /// Fetch a value by key.
    ///
    /// Returns `Ok(None)` if the key does not exist or has expired.
    async fn get<T>(&self, key: &str) -> Result<Option<T>, CacheError>
    where
        T: DeserializeOwned + Serialize + Send + Sync + Clone + 'static;

    /// Set a value for a key, optionally with a TTL.
    ///
    /// A `None` TTL means the backend chooses its default expiration behavior.
    async fn set<T>(&self, key: &str, value: &T, ttl: Option<Duration>) -> Result<(), CacheError>
    where
        T: Serialize + Send + Sync;

    /// Delete a key.
    async fn delete(&self, key: &str) -> Result<(), CacheError>;

    /// Check whether a key exists (TTL-aware).
    async fn exists(&self, key: &str) -> Result<bool, CacheError>;

    /// Atomically increment an integer value stored at `key`.
    ///
    /// Backends should treat missing or non-integer values as zero.
    async fn increment(&self, key: &str, value: i64) -> Result<i64, CacheError>;

    /// Clear all keys in the backend.
    ///
    /// This is mainly intended for tests and local development.
    async fn clear(&self) -> Result<(), CacheError>;

    /// Basic backend health check.
    ///
    /// Returns `Ok(true)` when the backend is reachable and operational.
    async fn health_check(&self) -> Result<bool, CacheError>;
}
