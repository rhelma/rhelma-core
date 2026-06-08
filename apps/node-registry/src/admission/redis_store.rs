#![forbid(unsafe_code)]

use std::time::Duration;

use redis::aio::ConnectionManager;
use redis::{AsyncCommands, Script};

use crate::admission::AdmissionChallengeRecord;

/// Redis-backed admission store for PoW challenges and rate-limiting.
///
/// Used by node-registry when `RHELMA_NODE_REGISTRY__ADMISSION__REDIS_URL` (or `RHELMA_REDIS_URL`) is set.
#[derive(Clone)]
pub struct RedisAdmissionStore {
    prefix: String,
    challenge_ttl: Duration,
    rate_limit_ttl: Duration,
    rate_limit_max: u32,
    conn: ConnectionManager,
}

impl RedisAdmissionStore {
    /// Connect to Redis and build a store.
    pub async fn connect(
        redis_url: &str,
        prefix: String,
        challenge_ttl: Duration,
        rate_limit_ttl: Duration,
        rate_limit_max: u32,
    ) -> Result<Self, String> {
        let client = redis::Client::open(redis_url).map_err(|e| e.to_string())?;
        let conn = client
            .get_connection_manager()
            .await
            .map_err(|e| e.to_string())?;

        Ok(Self {
            prefix,
            challenge_ttl,
            rate_limit_ttl,
            rate_limit_max,
            conn,
        })
    }

    fn challenge_key(&self, nonce_hex: &str) -> String {
        format!("{}:challenge:{}", self.prefix, nonce_hex)
    }

    fn ratelimit_key(&self, ip: &str) -> String {
        let mut out = String::with_capacity(ip.len());
        for ch in ip.chars() {
            if ch.is_ascii_alphanumeric() {
                out.push(ch);
            } else {
                out.push('_');
            }
        }
        format!("{}:ratelimit:{}", self.prefix, out)
    }

    /// Store a new PoW challenge under the given nonce.
    pub async fn put_challenge(
        &self,
        nonce_hex: &str,
        record: &AdmissionChallengeRecord,
    ) -> Result<(), String> {
        let key = self.challenge_key(nonce_hex);
        let value = serde_json::to_string(record).map_err(|e| e.to_string())?;
        let ttl_secs = self.challenge_ttl.as_secs().max(1);

        let mut conn = self.conn.clone();
        conn.set_ex::<_, _, ()>(key, value, ttl_secs)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Atomically fetch and delete a challenge.
    pub async fn take_challenge(
        &self,
        nonce_hex: &str,
    ) -> Result<Option<AdmissionChallengeRecord>, String> {
        let key = self.challenge_key(nonce_hex);

        // Atomic GET + DEL.
        let script = Script::new(
            r#"
            local val = redis.call('GET', KEYS[1])
            if val then
                redis.call('DEL', KEYS[1])
                return val
            end
            return nil
        "#,
        );

        let mut conn = self.conn.clone();
        let result: Option<String> = script
            .key(&key)
            .invoke_async(&mut conn)
            .await
            .map_err(|e| e.to_string())?;

        match result {
            Some(json) => {
                let record: AdmissionChallengeRecord =
                    serde_json::from_str(&json).map_err(|e| e.to_string())?;
                Ok(Some(record))
            }
            None => Ok(None),
        }
    }

    /// Rate-limit check based on the configured window and max.
    pub async fn rate_limit_allow(&self, ip: &str) -> Result<bool, String> {
        let key = self.ratelimit_key(ip);
        let mut conn = self.conn.clone();

        let count: u32 = conn.incr(&key, 1_u32).await.map_err(|e| e.to_string())?;
        if count == 1 {
            let ttl_secs: i64 = self
                .rate_limit_ttl
                .as_secs()
                .max(1)
                .try_into()
                .unwrap_or(i64::MAX);
            conn.expire::<_, ()>(&key, ttl_secs)
                .await
                .map_err(|e| e.to_string())?;
        }

        Ok(count <= self.rate_limit_max)
    }
}
