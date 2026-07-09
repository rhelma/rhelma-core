#![forbid(unsafe_code)]

use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

use tokio::sync::Mutex;

/// Number of shards used by the idempotency cache.
///
/// Rationale:
/// - This cache is intentionally *in-memory* and best-effort.
/// - A single global mutex can become a hot lock under high-throughput consumers.
/// - Sharding drastically reduces contention while preserving the same external behavior.
///
/// This is kept as a small power-of-two to make shard selection fast.
const SHARD_COUNT: usize = 16;

/// Simple in-memory idempotency cache keyed by event_id.
///
/// - `check_and_mark(key)` returns `true` if this is the first time the key is seen (within TTL),
///   otherwise returns `false` (duplicate).
/// - Evicts oldest entries when `max_entries` is exceeded.
#[derive(Debug)]
pub struct IdempotencyCache {
    /// Time to live for cache entries
    ttl: Duration,
    /// Maximum entries per shard
    per_shard_max_entries: usize,
    /// Shard array
    shards: Vec<Mutex<Inner>>,
}

#[derive(Debug)]
struct Inner {
    /// Key to timestamp mapping
    entries: std::collections::HashMap<String, Instant>,
    /// Order of entries for LRU eviction
    order: VecDeque<(String, Instant)>,
}

impl IdempotencyCache {
    /// Creates a new idempotency cache
    ///
    /// # Arguments
    /// * `ttl_secs` - Time to live in seconds (minimum 1)
    /// * `max_entries` - Maximum total entries (minimum 1000)
    ///
    /// # Returns
    /// New idempotency cache instance
    pub fn new(ttl_secs: u64, max_entries: usize) -> Self {
        let ttl = Duration::from_secs(ttl_secs.max(1));
        let max_entries = max_entries.max(1_000);

        // Distribute the max across shards; total upper bound becomes
        // `max_entries + SHARD_COUNT - 1` (remainder spread), which is a tight bound.
        let per_shard_max_entries = max_entries.div_ceil(SHARD_COUNT);

        Self {
            ttl,
            per_shard_max_entries,
            shards: (0..SHARD_COUNT)
                .map(|_| {
                    Mutex::new(Inner {
                        entries: std::collections::HashMap::new(),
                        order: VecDeque::new(),
                    })
                })
                .collect(),
        }
    }

    /// Checks if a key is new and marks it as seen
    ///
    /// # Arguments
    /// * `key` - Cache key (typically event ID)
    ///
    /// # Returns
    /// `true` if key is new (first time seen within TTL), `false` if duplicate
    pub async fn check_and_mark(&self, key: &str) -> bool {
        let now = Instant::now();
        let shard_idx = shard_for_key(key);
        let mut g = self
            .shards
            .get(shard_idx)
            .expect("SHARD_COUNT must match shards length")
            .lock()
            .await;

        // prune a bit
        self.prune_locked(&mut g, now);

        if let Some(seen) = g.entries.get(key).copied() {
            if now.duration_since(seen) <= self.ttl {
                return false; // duplicate within TTL
            }
            // expired => refresh
        }

        let key_s = key.to_string();
        g.entries.insert(key_s.clone(), now);
        g.order.push_back((key_s, now));

        // enforce max_entries
        while g.entries.len() > self.per_shard_max_entries {
            if let Some((old_key, old_ts)) = g.order.pop_front() {
                // only remove if still matches (might have been refreshed)
                if g.entries.get(&old_key).copied() == Some(old_ts) {
                    g.entries.remove(&old_key);
                }
            } else {
                break;
            }
        }

        true
    }

    /// Prunes expired entries from a shard
    ///
    /// # Arguments
    /// * `g` - Shard inner state
    /// * `now` - Current time
    fn prune_locked(&self, g: &mut Inner, now: Instant) {
        // prune from front while expired
        while let Some((k, ts)) = g.order.front().cloned() {
            if now.duration_since(ts) <= self.ttl {
                break;
            }
            g.order.pop_front();
            if g.entries.get(&k).copied() == Some(ts) {
                g.entries.remove(&k);
            }
        }
    }
}

/// Fast, stable shard selection for a string key.
///
/// Uses FNV-1a (64-bit) and a power-of-two mask.
///
/// # Arguments
/// * `key` - String key
///
/// # Returns
/// Shard index (0 to SHARD_COUNT-1)
fn shard_for_key(key: &str) -> usize {
    debug_assert!(SHARD_COUNT.is_power_of_two());
    let mut h: u64 = 0xcbf29ce484222325; // FNV offset basis
    for b in key.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3); // FNV prime
    }
    (h as usize) & (SHARD_COUNT - 1)
}
