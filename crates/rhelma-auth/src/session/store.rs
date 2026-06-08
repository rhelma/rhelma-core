use async_trait::async_trait;
use chrono::Utc;
use redis::AsyncCommands;
use secrecy::{ExposeSecret, SecretString};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::error::{AuthError, AuthResult};
use crate::metrics;
use crate::types::{RefreshRecord, Session, SessionId};
use rhelma_core::prelude::UserId;

#[async_trait]
/// trait (documented for contract compliance).
pub trait SessionStore: Send + Sync {
    async fn save_session(&self, session: &Session) -> AuthResult<()>;
    async fn get_session(&self, sid: &SessionId) -> AuthResult<Option<Session>>;
    async fn delete_session(&self, sid: &SessionId) -> AuthResult<()>;

    async fn bind_jti(&self, jti: &str, sid: &SessionId, ttl_secs: u64) -> AuthResult<()>;
    async fn get_session_by_jti(&self, jti: &str) -> AuthResult<Option<Session>>;
    async fn delete_jti(&self, jti: &str) -> AuthResult<()>;

    async fn save_refresh(
        &self,
        refresh_hash: &str,
        rec: &RefreshRecord,
        ttl_secs: u64,
    ) -> AuthResult<()>;
    async fn get_refresh(&self, refresh_hash: &str) -> AuthResult<Option<RefreshRecord>>;
    async fn delete_refresh(&self, refresh_hash: &str) -> AuthResult<()>;

    async fn list_user_sessions(&self, user_id: &UserId) -> AuthResult<Vec<SessionId>>;
    async fn delete_user_session_index(&self, user_id: &UserId) -> AuthResult<()>;
}

/// Redis-backed implementation.
#[derive(Clone)]
/// struct (documented for contract compliance).
pub struct RedisSessionStore {
    prefix: String,
    conn: Arc<Mutex<redis::aio::ConnectionManager>>,
}

impl RedisSessionStore {
    /// async fn (documented for contract compliance).
    pub async fn new(redis_url: &SecretString, prefix: String) -> AuthResult<Self> {
        let client = redis::Client::open(redis_url.expose_secret().as_str())
            .map_err(|_| AuthError::SessionStore)?;

        let manager = client
            .get_connection_manager()
            .await
            .map_err(|_| AuthError::SessionStore)?;

        Ok(Self {
            prefix,
            conn: Arc::new(Mutex::new(manager)),
        })
    }

    fn k_session(&self, sid: &SessionId) -> String {
        format!("{}:sess:{}", self.prefix, sid)
    }

    fn k_user_set(&self, user_id: &UserId) -> String {
        format!("{}:user:{}:sessions", self.prefix, user_id)
    }

    fn k_jti(&self, jti: &str) -> String {
        format!("{}:jti:{}", self.prefix, jti)
    }

    fn k_refresh(&self, refresh_hash: &str) -> String {
        format!("{}:refresh:{}", self.prefix, refresh_hash)
    }

    fn ttl_for_session(session: &Session) -> u64 {
        let now = Utc::now();
        if session.expires_at <= now {
            1
        } else {
            (session.expires_at - now).num_seconds().max(1) as u64
        }
    }
}

#[async_trait]
impl SessionStore for RedisSessionStore {
    async fn save_session(&self, session: &Session) -> AuthResult<()> {
        let _span = crate::tracing_ext::auth_span("redis.save_session");
        let started = std::time::Instant::now();

        let ttl = Self::ttl_for_session(session);
        let payload = serde_json::to_string(session)?;

        let session_key = self.k_session(&session.id);
        let user_set = self.k_user_set(&session.user_id);

        let mut con = self.conn.lock().await;

        // 1) sess:{sid} -> json (ttl)
        con.set_ex::<_, _, ()>(&session_key, payload, ttl).await?;

        // 2) user:{uid}:sessions contains sid (best-effort TTL)
        con.sadd::<_, _, ()>(&user_set, session.id.to_string())
            .await?;
        let _ = con.expire::<_, ()>(&user_set, ttl as i64).await;

        metrics::record_session_store("save_session", "ok", started.elapsed().as_secs_f64());
        Ok(())
    }

    async fn get_session(&self, sid: &SessionId) -> AuthResult<Option<Session>> {
        let _span = crate::tracing_ext::auth_span("redis.get_session");
        let started = std::time::Instant::now();

        let key = self.k_session(sid);
        let mut con = self.conn.lock().await;

        let payload: Option<String> = con.get(&key).await?;
        let out = match payload {
            None => None,
            Some(s) => Some(serde_json::from_str::<Session>(&s)?),
        };

        metrics::record_session_store("get_session", "ok", started.elapsed().as_secs_f64());
        Ok(out)
    }

    async fn delete_session(&self, sid: &SessionId) -> AuthResult<()> {
        let _span = crate::tracing_ext::auth_span("redis.delete_session");
        let started = std::time::Instant::now();

        // best-effort cleanup: read session first to remove from index + jti mapping
        let session = self.get_session(sid).await?;

        let mut con = self.conn.lock().await;
        let key = self.k_session(sid);
        let _: () = con.del(&key).await?;

        if let Some(sess) = session {
            let user_set = self.k_user_set(&sess.user_id);
            let _ = con.srem::<_, _, ()>(&user_set, sid.to_string()).await;

            if let Some(jti) = sess.current_jti.as_deref() {
                let _ = con.del::<_, ()>(self.k_jti(jti)).await;
            }
        }

        metrics::record_session_store("delete_session", "ok", started.elapsed().as_secs_f64());
        Ok(())
    }

    async fn bind_jti(&self, jti: &str, sid: &SessionId, ttl_secs: u64) -> AuthResult<()> {
        let mut con = self.conn.lock().await;
        con.set_ex::<_, _, ()>(self.k_jti(jti), sid.to_string(), ttl_secs)
            .await?;
        Ok(())
    }

    async fn get_session_by_jti(&self, jti: &str) -> AuthResult<Option<Session>> {
        let mut con = self.conn.lock().await;
        let sid_str: Option<String> = con.get(self.k_jti(jti)).await?;
        drop(con);

        let sid_str = match sid_str {
            None => return Ok(None),
            Some(v) => v,
        };

        let sid = SessionId::parse(&sid_str)?;
        self.get_session(&sid).await
    }

    async fn delete_jti(&self, jti: &str) -> AuthResult<()> {
        let mut con = self.conn.lock().await;
        let _: () = con.del(self.k_jti(jti)).await?;
        Ok(())
    }

    async fn save_refresh(
        &self,
        refresh_hash: &str,
        rec: &RefreshRecord,
        ttl_secs: u64,
    ) -> AuthResult<()> {
        let mut con = self.conn.lock().await;
        let payload = serde_json::to_string(rec)?;
        con.set_ex::<_, _, ()>(self.k_refresh(refresh_hash), payload, ttl_secs)
            .await?;
        Ok(())
    }

    async fn get_refresh(&self, refresh_hash: &str) -> AuthResult<Option<RefreshRecord>> {
        let mut con = self.conn.lock().await;
        let payload: Option<String> = con.get(self.k_refresh(refresh_hash)).await?;
        Ok(match payload {
            None => None,
            Some(v) => Some(serde_json::from_str::<RefreshRecord>(&v)?),
        })
    }

    async fn delete_refresh(&self, refresh_hash: &str) -> AuthResult<()> {
        let mut con = self.conn.lock().await;
        let _: () = con.del(self.k_refresh(refresh_hash)).await?;
        Ok(())
    }

    async fn list_user_sessions(&self, user_id: &UserId) -> AuthResult<Vec<SessionId>> {
        let mut con = self.conn.lock().await;
        let set_key = self.k_user_set(user_id);
        let members: Vec<String> = con.smembers(set_key).await?;
        let mut out = Vec::with_capacity(members.len());
        for m in members {
            if let Ok(sid) = SessionId::parse(&m) {
                out.push(sid);
            }
        }
        Ok(out)
    }

    async fn delete_user_session_index(&self, user_id: &UserId) -> AuthResult<()> {
        let mut con = self.conn.lock().await;
        let _: () = con.del(self.k_user_set(user_id)).await?;
        Ok(())
    }
}
