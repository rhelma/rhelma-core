// crates/rhelma-cache/src/types.rs

use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};

/// A cache entry that stores a value along with metadata for cache management.
///
/// This struct represents a single entry in the cache, containing the cached value,
/// creation timestamp, optional expiration time, and hit counter for tracking access frequency.
///
/// # Type Parameters
/// - `T`: The type of the cached value. Must implement `Clone`, `Serialize`, and `Deserialize`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry<T> {
    /// The actual cached value.
    pub value: T,

    /// The system time when this cache entry was created.
    pub created_at: SystemTime,

    /// The optional system time when this cache entry expires.
    /// If `None`, the entry never expires (unless explicitly removed).
    pub expires_at: Option<SystemTime>,

    /// Counter tracking how many times this entry has been accessed.
    pub hits: u64,
}

impl<T> CacheEntry<T> {
    /// Creates a new cache entry with the given value and time-to-live (TTL).
    ///
    /// # Arguments
    /// - `value`: The value to cache.
    /// - `ttl`: Optional time-to-live duration. If `None`, the entry never expires.
    ///
    /// # Returns
    /// A new `CacheEntry` instance with the current system time as creation timestamp,
    /// calculated expiration time (if TTL provided), and hit counter initialized to zero.
    ///
    /// # Examples
    /// ```
    /// use std::time::Duration;
    /// use rhelma_cache::types::CacheEntry;
    ///
    /// // Create a non-expiring cache entry
    /// let entry = CacheEntry::new("my_value", None);
    ///
    /// // Create a cache entry that expires after 5 minutes
    /// let entry = CacheEntry::new("my_value", Some(Duration::from_secs(300)));
    /// ```
    pub fn new(value: T, ttl: Option<Duration>) -> Self {
        let created_at = SystemTime::now();
        let expires_at = ttl.map(|ttl| created_at + ttl);

        Self {
            value,
            created_at,
            expires_at,
            hits: 0,
        }
    }

    /// Checks whether this cache entry has expired.
    ///
    /// # Returns
    /// - `true` if the entry has an expiration time and the current system time is past that time.
    /// - `false` if the entry never expires or hasn't reached its expiration time yet.
    ///
    /// # Note
    /// This method uses the current system time, so calling it at different times
    /// may yield different results for the same cache entry.
    ///
    /// # Examples
    /// ```
    /// use std::time::Duration;
    /// use rhelma_cache::types::CacheEntry;
    ///
    /// let mut entry = CacheEntry::new("value", Some(Duration::from_secs(1)));
    /// // Immediately after creation
    /// assert!(!entry.is_expired());
    /// // After 2 seconds (if you actually wait)
    /// // assert!(entry.is_expired());
    /// ```
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            SystemTime::now() > expires_at
        } else {
            false
        }
    }

    /// Records a hit (access) to this cache entry.
    ///
    /// Increments the hit counter by one. This is typically called whenever
    /// the cache entry is successfully retrieved from the cache.
    ///
    /// # Examples
    /// ```
    /// use rhelma_cache::types::CacheEntry;
    ///
    /// let mut entry = CacheEntry::new("value", None);
    /// assert_eq!(entry.hits, 0);
    ///
    /// entry.record_hit();
    /// assert_eq!(entry.hits, 1);
    ///
    /// entry.record_hit();
    /// assert_eq!(entry.hits, 2);
    /// ```
    pub fn record_hit(&mut self) {
        self.hits += 1;
    }
}

/// Statistics for tracking cache performance and usage.
///
/// This struct provides metrics that can be used to monitor cache effectiveness,
/// memory usage, and eviction patterns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    /// Total number of successful cache retrievals.
    pub hits: u64,

    /// Total number of cache misses (requests for non-existent or expired entries).
    pub misses: u64,

    /// Estimated memory usage of the cache in bytes.
    /// This is an approximation and may not account for all overhead.
    pub memory_usage: usize,

    /// Current number of entries stored in the cache.
    pub entry_count: usize,

    /// Total number of entries that have been evicted from the cache.
    /// This includes both expired entries and those removed due to size constraints.
    pub eviction_count: u64,
}

impl Default for CacheStats {
    /// Creates default cache statistics with all counters set to zero.
    ///
    /// # Returns
    /// A `CacheStats` instance with:
    /// - `hits`: 0
    /// - `misses`: 0
    /// - `memory_usage`: 0
    /// - `entry_count`: 0
    /// - `eviction_count`: 0
    fn default() -> Self {
        Self {
            hits: 0,
            misses: 0,
            memory_usage: 0,
            entry_count: 0,
            eviction_count: 0,
        }
    }
}
