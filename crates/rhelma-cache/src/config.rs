// crates/rhelma-cache/src/config.rs

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Main configuration structure for the cache system.
///
/// This struct holds all configuration options for the cache system,
/// including backend-specific configurations and global settings.
///
/// # Fields
/// - `default_ttl`: Default time-to-live for cache entries when not explicitly specified.
/// - `max_memory_size`: Maximum memory usage (in bytes) before eviction is triggered.
/// - `redis`: Optional configuration for Redis backend. If `None`, Redis is disabled.
/// - `memory`: Configuration for in-memory cache backend.
/// - `layered`: Configuration for layered caching (combining multiple backends).
///
/// # Examples
/// ```
/// use std::time::Duration;
/// use rhelma_cache::config::{CacheConfig, EvictionPolicy, LayeredCacheConfig, MemoryCacheConfig, RedisCacheConfig};
///
/// let config = CacheConfig {
///     default_ttl: Duration::from_secs(600),
///     max_memory_size: 500_000_000, // 500MB
///     redis: Some(RedisCacheConfig {
///         url: "redis://localhost:6379".to_string(),
///         key_prefix: "myapp:".to_string(),
///         connection_timeout: Duration::from_secs(5),
///         pool_size: 10,
///     }),
///     memory: MemoryCacheConfig {
///         max_capacity: 50_000,
///         eviction_policy: EvictionPolicy::Lru,
///         time_to_idle: Some(Duration::from_secs(300)),
///     },
///     layered: LayeredCacheConfig {
///         enabled: true,
///         memory_to_redis_ratio: 0.7,
///     },
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Default time-to-live duration for cache entries.
    ///
    /// This is used when no explicit TTL is provided for a cache operation.
    /// Entries with no explicit TTL will use this value.
    pub default_ttl: Duration,

    /// Maximum memory usage in bytes for the cache system.
    ///
    /// When total memory usage approaches this limit, the cache will
    /// start evicting entries according to the configured eviction policy.
    pub max_memory_size: usize,

    /// Optional Redis cache configuration.
    ///
    /// If `Some`, Redis will be used as a cache backend (either standalone
    /// or as part of a layered cache). If `None`, Redis functionality is disabled.
    pub redis: Option<RedisCacheConfig>,

    /// In-memory cache configuration.
    ///
    /// Controls the behavior of the local in-memory cache, which is always enabled
    /// and provides the fastest access but is limited to single process/instance.
    pub memory: MemoryCacheConfig,

    /// Layered cache configuration.
    ///
    /// Controls how multiple cache layers (e.g., memory + Redis) work together
    /// to provide both fast local access and shared/distributed caching.
    pub layered: LayeredCacheConfig,
}

/// Configuration for Redis cache backend.
///
/// This struct contains all Redis-specific configuration options needed to
/// connect to and interact with a Redis server or cluster.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisCacheConfig {
    /// Redis connection URL.
    ///
    /// Format: `redis://[username:password@]host:port[/database]`
    /// Example: `redis://:password@localhost:6379/0`
    pub url: String,

    /// Prefix to prepend to all cache keys stored in Redis.
    ///
    /// This helps prevent key collisions when multiple applications
    /// or environments share the same Redis instance.
    /// Example: `"myapp:cache:"`
    pub key_prefix: String,

    /// Maximum time to wait for a Redis connection to be established.
    ///
    /// If connection takes longer than this duration, the operation fails.
    pub connection_timeout: Duration,

    /// Maximum number of connections in the Redis connection pool.
    ///
    /// Higher values allow more concurrent operations but consume more resources.
    pub pool_size: u32,
}

/// Configuration for in-memory cache backend.
///
/// Controls the behavior of the local, process-internal cache that provides
/// the fastest possible access but is not shared between processes or instances.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryCacheConfig {
    /// Maximum number of entries in the memory cache.
    ///
    /// When this limit is reached, older entries are evicted according to
    /// the configured eviction policy.
    pub max_capacity: usize,

    /// Policy used to decide which entries to evict when cache is full.
    ///
    /// See [`EvictionPolicy`] for available options.
    pub eviction_policy: EvictionPolicy,

    /// Maximum time an entry can remain idle before being evicted.
    ///
    /// If `Some`, entries that haven't been accessed for this duration will
    /// be removed even if they haven't expired. If `None`, no idle eviction.
    pub time_to_idle: Option<Duration>,
}

/// Configuration for layered cache system.
///
/// Controls how multiple cache layers work together. Typically, this involves
/// a fast local memory cache (L1) and a slower but shared Redis cache (L2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayeredCacheConfig {
    /// Whether layered caching is enabled.
    ///
    /// If `true`, the cache system will use multiple layers (e.g., memory + Redis).
    /// If `false`, only the primary cache backend is used.
    pub enabled: bool,

    /// Ratio of memory cache size to Redis cache size for layered caching.
    ///
    /// This controls how much data is kept in the fast memory cache relative
    /// to the Redis cache. Typical values are between 0.1 and 0.9.
    /// A value of 0.8 means memory cache holds 80% of what Redis holds.
    pub memory_to_redis_ratio: f64,
}

/// Defines the policy for evicting entries when cache is full.
///
/// Different eviction policies optimize for different access patterns.
/// Choose based on your application's typical cache usage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EvictionPolicy {
    /// Least Recently Used (LRU).
    ///
    /// Evicts entries that haven't been accessed for the longest time.
    /// Good for general-purpose caching with temporal locality.
    Lru,

    /// Least Frequently Used (LFU).
    ///
    /// Evicts entries with the fewest accesses.
    /// Good for caching where some items are accessed much more frequently than others.
    Lfu,

    /// Time-To-Live based eviction.
    ///
    /// Evicts entries based on expiration time (nearest expiry first).
    /// Good for time-sensitive data where freshness is more important than access patterns.
    Ttl,
}

impl Default for CacheConfig {
    /// Creates a default cache configuration with sensible defaults.
    ///
    /// # Returns
    /// A `CacheConfig` with:
    /// - `default_ttl`: 5 minutes (300 seconds)
    /// - `max_memory_size`: 100,000 entries
    /// - `redis`: `None` (disabled by default)
    /// - `memory`: Default `MemoryCacheConfig`
    /// - `layered`: Default `LayeredCacheConfig`
    fn default() -> Self {
        Self {
            default_ttl: Duration::from_secs(300),
            max_memory_size: 100_000,
            redis: None,
            memory: MemoryCacheConfig::default(),
            layered: LayeredCacheConfig::default(),
        }
    }
}

impl Default for MemoryCacheConfig {
    /// Creates a default memory cache configuration.
    ///
    /// # Returns
    /// A `MemoryCacheConfig` with:
    /// - `max_capacity`: 10,000 entries
    /// - `eviction_policy`: `EvictionPolicy::Lru`
    /// - `time_to_idle`: 5 minutes (300 seconds)
    fn default() -> Self {
        Self {
            max_capacity: 10_000,
            eviction_policy: EvictionPolicy::Lru,
            time_to_idle: Some(Duration::from_secs(300)),
        }
    }
}

impl Default for LayeredCacheConfig {
    /// Creates a default layered cache configuration.
    ///
    /// # Returns
    /// A `LayeredCacheConfig` with:
    /// - `enabled`: `true`
    /// - `memory_to_redis_ratio`: 0.8 (memory cache holds 80% of Redis cache size)
    fn default() -> Self {
        Self {
            enabled: true,
            memory_to_redis_ratio: 0.8,
        }
    }
}
