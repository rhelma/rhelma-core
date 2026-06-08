// crates/rhelma-cache/src/error.rs

use thiserror::Error;

/// Represents errors that can occur in cache operations.
///
/// This enum defines all possible error types that the cache system can produce.
/// Each variant includes a descriptive error message for debugging and logging.
///
/// # Variants
/// - `Connection`: Failed to establish or maintain a connection to the cache backend.
/// - `Serialization`: Failed to serialize data before storing it in the cache.
/// - `Deserialization`: Failed to deserialize data retrieved from the cache.
/// - `Timeout`: Operation timed out while waiting for cache response.
/// - `NotFound`: Requested key does not exist in the cache or has expired.
/// - `Config`: Invalid or malformed cache configuration.
/// - `Backend`: Generic backend-specific error (e.g., Redis, Memcached issues).
///
/// # Examples
/// ```
/// use rhelma_cache::error::CacheError;
///
/// // Creating different types of cache errors
/// let conn_error = CacheError::Connection("Failed to connect to Redis".to_string());
/// let not_found = CacheError::NotFound("user:1234".to_string());
/// ```
#[derive(Error, Debug)]
pub enum CacheError {
    /// Failed to establish or maintain a connection to the cache backend.
    ///
    /// This typically indicates network issues, authentication problems,
    /// or the cache service being unavailable.
    #[error("Cache connection error: {0}")]
    /// Variant `Connection`.
    Connection(String),

    /// Failed to serialize data before storing it in the cache.
    ///
    /// This occurs when the data structure cannot be converted to a format
    /// suitable for the cache backend (e.g., JSON serialization failure).
    #[error("Cache serialization error: {0}")]
    /// Variant `Serialization`.
    Serialization(String),

    /// Failed to deserialize data retrieved from the cache.
    ///
    /// This occurs when cached data is corrupted or in an unexpected format.
    /// It may indicate version mismatches or data corruption.
    #[error("Cache deserialization error: {0}")]
    /// Variant `Deserialization`.
    Deserialization(String),

    /// Operation timed out while waiting for cache response.
    ///
    /// This indicates that the cache backend took too long to respond,
    /// possibly due to high load, network latency, or backend issues.
    #[error("Cache timeout: {0}")]
    /// Variant `Timeout`.
    Timeout(String),

    /// Requested key does not exist in the cache or has expired.
    ///
    /// This is a normal error that indicates a cache miss and should
    /// typically trigger recomputation or database lookup.
    #[error("Cache key not found: {0}")]
    /// Variant `NotFound`.
    NotFound(String),

    /// Invalid or malformed cache configuration.
    ///
    /// This includes errors like invalid TTL values, missing required
    /// configuration parameters, or unsupported configuration options.
    #[error("Cache configuration error: {0}")]
    /// Variant `Config`.
    Config(String),

    /// Generic backend-specific error.
    ///
    /// This captures errors that don't fit into other categories but are
    /// specific to the cache backend implementation (Redis, Memcached, etc.).
    #[error("Cache backend error: {0}")]
    /// Variant `Backend`.
    Backend(String),
}

impl From<CacheError> for rhelma_core::RhelmaError {
    /// Converts a `CacheError` into a `RhelmaError` for integration with the core error system.
    ///
    /// This conversion allows cache errors to be seamlessly integrated into the
    /// broader application error handling system while preserving error context.
    ///
    /// # Mapping
    /// - `CacheError::NotFound` → `RhelmaError::NotFound`
    /// - `CacheError::Config` → `RhelmaError::Config`
    /// - All other variants → `RhelmaError::Cache` with prefixed error type
    ///
    /// # Arguments
    /// - `err`: The cache error to convert
    ///
    /// # Returns
    /// A `RhelmaError` variant appropriate for the cache error type.
    ///
    /// # Examples
    /// ```
    /// use rhelma_cache::error::CacheError;
    /// use rhelma_core::RhelmaError;
    ///
    /// let cache_err = CacheError::NotFound("user:1234".to_string());
    /// let rhelma_err: RhelmaError = cache_err.into();
    ///
    /// // This would produce RhelmaError::NotFound("user:1234")
    /// ```
    fn from(err: CacheError) -> Self {
        use rhelma_core::RhelmaError;

        match err {
            CacheError::NotFound(msg) => RhelmaError::NotFound(msg),
            CacheError::Config(msg) => RhelmaError::Config(msg),

            CacheError::Connection(msg) => RhelmaError::Cache(format!("connection: {msg}")),
            CacheError::Timeout(msg) => RhelmaError::Cache(format!("timeout: {msg}")),
            CacheError::Serialization(msg) => RhelmaError::Cache(format!("serialization: {msg}")),
            CacheError::Deserialization(msg) => {
                RhelmaError::Cache(format!("deserialization: {msg}"))
            }
            CacheError::Backend(msg) => RhelmaError::Cache(format!("backend: {msg}")),
        }
    }
}
