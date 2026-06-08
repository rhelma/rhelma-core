//! In-memory TTL counters for admission control.
//!
//! This is a minimal sketch for Phase 45. Prefer a shared store for multi-instance.

use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Debug, Default)]
pub struct TtlCounter {
    map: HashMap<String, (u32, Instant)>,
}

impl TtlCounter {
    pub fn bump(&mut self, key: &str, ttl: Duration) -> u32 {
        let now = Instant::now();
        let ent = self.map.entry(key.to_string()).or_insert((0, now + ttl));
        if now > ent.1 {
            *ent = (0, now + ttl);
        }
        ent.0 += 1;
        ent.0
    }

    pub fn prune(&mut self) {
        let now = Instant::now();
        self.map.retain(|_, v| now <= v.1);
    }
}
