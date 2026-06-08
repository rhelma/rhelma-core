// crates/rhelma-cache/src/metrics.rs

//! Cache metrics collection helpers.
//!
//! This module provides lightweight, backend-agnostic metrics recording functions
//! for monitoring cache performance and behavior in production environments.
//!
//! # Features
//! - Standardized cache metrics following observability best practices
//! - Backend-agnostic design usable from any cache implementation
//! - Low-overhead recording suitable for high-throughput systems
//! - Rich labeling with backend type, operation, and key space dimensions
//!
//! # Metrics Exported
//!
//! | Metric Name | Type | Description | Labels |
//! |-------------|------|-------------|--------|
//! | `cache_hit_total` | Counter | Total cache hits | `backend`, `operation`, `key_space` |
//! | `cache_miss_total` | Counter | Total cache misses | `backend`, `operation`, `key_space` |
//! | `cache_op_duration_seconds` | Histogram | Operation latency in seconds | `backend`, `operation`, `key_space` |
//! | `cache_errors_total` | Counter | Total cache operation errors | `backend`, `operation`, `key_space` |
//!
//! # Usage
//!
//! ```rust
//! use rhelma_cache::metrics::{record_hit, record_miss, CacheBackendKind};
//! use std::time::{Instant, Duration};
//!
//! // Record metrics around cache operations
//! let backend = CacheBackendKind::Redis;
//! let operation = "get";
//! let key_space = "users";
//!
//! let start = Instant::now();
//!
//! // Perform cache operation...
//! let hit = true;
//!
//! if hit {
//!     record_hit(backend, operation, key_space);
//! } else {
//!     record_miss(backend, operation, key_space);
//! }
//!
//! let duration = start.elapsed();
//! rhelma_cache::metrics::record_duration(backend, operation, key_space, duration);
//! ```
//!
//! # Integration with Cache Backends
//!
//! These helpers are designed to be called from:
//! - Memory cache implementations
//! - Redis cache implementations
//! - Layered cache implementations
//! - Cache macros and higher-level abstractions
//!
//! # Label Dimensions
//!
//! 1. **backend**: Which cache backend handled the operation (`memory`, `redis`, `layered`)
//! 2. **operation**: The cache operation (`get`, `set`, `delete`, `exists`, `increment`, `clear`)
//! 3. **key_space**: Logical grouping of keys (e.g., `users`, `sessions`, `config`)
//!
//! # Performance Considerations
//!
//! - Metrics recording is designed to be low-overhead
//! - Functions use `&'static str` for labels to avoid allocations
//! - Duration recording uses `as_secs_f64()` for precision
//! - Backend kind uses enum dispatch for efficient string conversion

use std::time::Duration;

/// Cache backend kind for metrics labeling.
///
/// This enum identifies which cache backend handled an operation,
/// allowing metrics to be segmented by backend type for detailed analysis.
///
/// # Variants
/// - `Memory`: In-memory LRU cache backend
/// - `Redis`: Redis-based distributed cache backend
/// - `Layered`: Two-tier memory+Redis layered cache backend
/// - `Other(&'static str)`: Custom backend type for extensibility
///
/// # Examples
/// ```
/// use rhelma_cache::metrics::CacheBackendKind;
///
/// // Standard backends
/// let memory = CacheBackendKind::Memory;
/// let redis = CacheBackendKind::Redis;
/// let layered = CacheBackendKind::Layered;
///
/// // Custom backend
/// let custom = CacheBackendKind::Other("memcached");
///
/// assert_eq!(memory.as_str(), "memory");
/// assert_eq!(custom.as_str(), "memcached");
/// ```
#[derive(Debug, Clone, Copy)]
pub enum CacheBackendKind {
    /// In-memory LRU cache backend.
    Memory,

    /// Redis-based distributed cache backend.
    Redis,

    /// Two-tier memory+Redis layered cache backend.
    Layered,

    /// Custom cache backend type.
    ///
    /// Use this variant for custom cache implementations or
    /// future backend types not covered by the standard variants.
    Other(&'static str),
}

impl CacheBackendKind {
    /// Returns the string representation of the backend kind.
    ///
    /// This is used for metrics label values and should match the
    /// backend identifiers used in your monitoring configuration.
    ///
    /// # Returns
    /// - `"memory"` for `Memory`
    /// - `"redis"` for `Redis`
    /// - `"layered"` for `Layered`
    /// - Custom string for `Other`
    ///
    /// # Examples
    /// ```
    /// use rhelma_cache::metrics::CacheBackendKind;
    ///
    /// assert_eq!(CacheBackendKind::Memory.as_str(), "memory");
    /// assert_eq!(CacheBackendKind::Other("postgres").as_str(), "postgres");
    /// ```
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            CacheBackendKind::Memory => "memory",
            CacheBackendKind::Redis => "redis",
            CacheBackendKind::Layered => "layered",
            CacheBackendKind::Other(s) => s,
        }
    }
}

/// Records a cache hit event.
///
/// Increments the `cache_hit_total` counter with appropriate labels.
/// Call this function when a cache operation successfully retrieves
/// a value from the cache.
///
/// # Arguments
/// - `backend`: Which cache backend handled the operation
/// - `operation`: Cache operation name (e.g., `"get"`, `"exists"`)
/// - `key_space`: Logical grouping of cache keys (e.g., `"users"`, `"sessions"`)
///
/// # Metrics
/// Increments: `cache_hit_total{backend="...", operation="...", key_space="..."}`
///
/// # Examples
/// ```
/// use rhelma_cache::metrics::{record_hit, CacheBackendKind};
///
/// // Record a hit from Redis backend for a user lookup
/// record_hit(CacheBackendKind::Redis, "get", "users");
///
/// // Record a hit from memory cache for config lookup
/// record_hit(CacheBackendKind::Memory, "get", "config");
/// ```
pub fn record_hit(backend: CacheBackendKind, operation: &'static str, key_space: &'static str) {
    metrics::counter!(
        "cache_hit_total",
        "backend" => backend.as_str(),
        "operation" => operation,
        "key_space" => key_space,
    )
    .increment(1);
}

/// Records a cache miss event.
///
/// Increments the `cache_miss_total` counter with appropriate labels.
/// Call this function when a cache operation fails to find a value
/// (e.g., key doesn't exist or has expired).
///
/// # Arguments
/// - `backend`: Which cache backend handled the operation
/// - `operation`: Cache operation name (e.g., `"get"`, `"exists"`)
/// - `key_space`: Logical grouping of cache keys (e.g., `"users"`, `"sessions"`)
///
/// # Metrics
/// Increments: `cache_miss_total{backend="...", operation="...", key_space="..."}`
///
/// # Examples
/// ```
/// use rhelma_cache::metrics::{record_miss, CacheBackendKind};
///
/// // Record a miss from Redis backend for a user lookup
/// record_miss(CacheBackendKind::Redis, "get", "users");
///
/// // Record a miss from layered cache for session lookup
/// record_miss(CacheBackendKind::Layered, "get", "sessions");
/// ```
pub fn record_miss(backend: CacheBackendKind, operation: &'static str, key_space: &'static str) {
    metrics::counter!(
        "cache_miss_total",
        "backend" => backend.as_str(),
        "operation" => operation,
        "key_space" => key_space,
    )
    .increment(1);
}

/// Records cache operation duration.
///
/// Records the duration of a cache operation to the `cache_op_duration_seconds`
/// histogram with appropriate labels. Use this to monitor cache latency.
///
/// # Arguments
/// - `backend`: Which cache backend handled the operation
/// - `operation`: Cache operation name (e.g., `"get"`, `"set"`)
/// - `key_space`: Logical grouping of cache keys (e.g., `"users"`, `"sessions"`)
/// - `duration`: How long the operation took
///
/// # Metrics
/// Records to: `cache_op_duration_seconds{backend="...", operation="...", key_space="..."}`
///
/// # Note
/// Duration is recorded as seconds with nanosecond precision using `as_secs_f64()`.
///
/// # Examples
/// ```
/// use rhelma_cache::metrics::record_duration;
/// use rhelma_cache::metrics::CacheBackendKind;
/// use std::time::{Instant, Duration};
///
/// let start = Instant::now();
///
/// // Perform cache operation...
/// std::thread::sleep(Duration::from_millis(10));
///
/// let duration = start.elapsed();
/// record_duration(CacheBackendKind::Redis, "get", "users", duration);
/// ```
pub fn record_duration(
    backend: CacheBackendKind,
    operation: &'static str,
    key_space: &'static str,
    duration: Duration,
) {
    metrics::histogram!(
        "cache_op_duration_seconds",
        "backend" => backend.as_str(),
        "operation" => operation,
        "key_space" => key_space,
    )
    .record(duration.as_secs_f64());
}

/// Records a cache error occurrence.
///
/// Increments the `cache_errors_total` counter with appropriate labels.
/// Call this function when a cache operation fails due to an error
/// (e.g., connection failure, serialization error, timeout).
///
/// # Arguments
/// - `backend`: Which cache backend encountered the error
/// - `operation`: Cache operation name that failed (e.g., `"get"`, `"set"`)
/// - `key_space`: Logical grouping of cache keys (e.g., `"users"`, `"sessions"`)
///
/// # Metrics
/// Increments: `cache_errors_total{backend="...", operation="...", key_space="..."}`
///
/// # Examples
/// ```
/// use rhelma_cache::metrics::{record_error, CacheBackendKind};
///
/// // Record a Redis connection error during a get operation
/// record_error(CacheBackendKind::Redis, "get", "users");
///
/// // Record a serialization error during a set operation
/// record_error(CacheBackendKind::Memory, "set", "config");
/// ```
pub fn record_error(backend: CacheBackendKind, operation: &'static str, key_space: &'static str) {
    metrics::counter!(
        "cache_errors_total",
        "backend" => backend.as_str(),
        "operation" => operation,
        "key_space" => key_space,
    )
    .increment(1);
}
