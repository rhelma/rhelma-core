#![forbid(unsafe_code)]

//! Platform-level event envelope and append-only event stores.
//!
//! This module extends the existing `rhelma-event` crate instead of creating a
//! second platform-events crate. Transport publishers can still use
//! [`crate::EventEnvelope`]; this envelope is the durable, hashable platform
//! event record used by audit/event stores.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};
use thiserror::Error;
use uuid::Uuid;

use crate::canonicalization::{canonical_json_string, canonical_payload_hash_hex};
use crate::{generate_event_id, EventSource};

/// Schema reference for the durable platform event envelope.
pub const SCHEMA_PLATFORM_EVENT_ENVELOPE_V1: &str = "rhelma://schemas/platform.event@v1";

/// Shared platform-level event envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformEventEnvelope {
    /// Globally unique event id.
    pub event_id: String,
    /// Canonical event type, for example `platform.improvement.applied.v1`.
    pub event_type: String,
    /// Numeric event schema version.
    pub event_version: i32,
    /// When the event occurred.
    pub occurred_at: DateTime<Utc>,
    /// Producer service metadata.
    pub source: EventSource,
    /// Optional workspace id when the event belongs to a workspace.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,
    /// Optional tenant id when the event has an isolation context.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
    /// Optional user id when a human/user actor is known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    /// Required correlation id for system flows.
    pub correlation_id: String,
    /// Optional causation id linking this event to a prior event/proposal.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub causation_id: Option<String>,
    /// Typed payload serialized as JSON.
    pub payload: Value,
    /// SHA-256 of the canonical payload JSON.
    pub payload_sha256: String,
    /// Previous platform event hash in the append-only chain.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_event_hash: Option<String>,
    /// SHA-256 hash of this envelope's hash input.
    pub event_hash: String,
}

impl PlatformEventEnvelope {
    /// Build an envelope from an already-serialized payload.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        event_type: impl Into<String>,
        event_version: i32,
        occurred_at: DateTime<Utc>,
        source: EventSource,
        workspace_id: Option<String>,
        tenant_id: Option<String>,
        user_id: Option<String>,
        correlation_id: impl Into<String>,
        causation_id: Option<String>,
        payload: Value,
    ) -> Result<Self, PlatformEventError> {
        let event_id = generate_event_id();
        let payload_sha256 = canonical_payload_hash_hex(&payload);
        let mut event = Self {
            event_id,
            event_type: event_type.into(),
            event_version,
            occurred_at,
            source,
            workspace_id,
            tenant_id,
            user_id,
            correlation_id: correlation_id.into(),
            causation_id,
            payload,
            payload_sha256,
            previous_event_hash: None,
            event_hash: String::new(),
        };
        event.event_hash = event.compute_event_hash();
        event.validate()?;
        Ok(event)
    }

    /// Build an envelope from a typed serializable payload.
    #[allow(clippy::too_many_arguments)]
    pub fn from_payload<T: Serialize>(
        event_type: impl Into<String>,
        event_version: i32,
        occurred_at: DateTime<Utc>,
        source: EventSource,
        workspace_id: Option<String>,
        tenant_id: Option<String>,
        user_id: Option<String>,
        correlation_id: impl Into<String>,
        causation_id: Option<String>,
        payload: &T,
    ) -> Result<Self, PlatformEventError> {
        let payload = serde_json::to_value(payload)
            .map_err(|e| PlatformEventError::Serialization(e.to_string()))?;
        Self::new(
            event_type,
            event_version,
            occurred_at,
            source,
            workspace_id,
            tenant_id,
            user_id,
            correlation_id,
            causation_id,
            payload,
        )
    }

    /// Return a clone linked to `previous_event_hash`.
    pub fn with_previous_event_hash(mut self, previous_event_hash: Option<String>) -> Self {
        self.previous_event_hash = previous_event_hash;
        self.event_hash = self.compute_event_hash();
        self
    }

    /// Validate required fields and stored hashes.
    pub fn validate(&self) -> Result<(), PlatformEventError> {
        if self.event_id.trim().is_empty() {
            return Err(PlatformEventError::Validation(
                "event_id required".to_string(),
            ));
        }
        Uuid::parse_str(&self.event_id)
            .map_err(|_| PlatformEventError::Validation("event_id must be UUID".to_string()))?;
        if self.event_type.trim().is_empty() {
            return Err(PlatformEventError::Validation(
                "event_type required".to_string(),
            ));
        }
        if self.event_version <= 0 {
            return Err(PlatformEventError::Validation(
                "event_version must be positive".to_string(),
            ));
        }
        if self.source.service.trim().is_empty() {
            return Err(PlatformEventError::Validation(
                "source required".to_string(),
            ));
        }
        if self.correlation_id.trim().is_empty() {
            return Err(PlatformEventError::Validation(
                "correlation_id required".to_string(),
            ));
        }
        let computed_payload_hash = canonical_payload_hash_hex(&self.payload);
        if computed_payload_hash != self.payload_sha256 {
            return Err(PlatformEventError::Validation(
                "payload_sha256 mismatch".to_string(),
            ));
        }
        let computed_event_hash = self.compute_event_hash();
        if computed_event_hash != self.event_hash {
            return Err(PlatformEventError::Validation(
                "event_hash mismatch".to_string(),
            ));
        }
        if contains_obvious_secret_material(&self.payload) {
            return Err(PlatformEventError::Validation(
                "platform event payload contains obvious secret material".to_string(),
            ));
        }
        Ok(())
    }

    /// Stable event hash for this envelope.
    pub fn compute_event_hash(&self) -> String {
        let input = json!({
            "event_id": self.event_id,
            "event_type": self.event_type,
            "event_version": self.event_version,
            "occurred_at": self.occurred_at,
            "source": self.source,
            "workspace_id": self.workspace_id,
            "tenant_id": self.tenant_id,
            "user_id": self.user_id,
            "correlation_id": self.correlation_id,
            "causation_id": self.causation_id,
            "payload_sha256": self.payload_sha256,
            "previous_event_hash": self.previous_event_hash,
        });
        let canonical = canonical_json_string(&input);
        hex::encode(Sha256::digest(canonical.as_bytes()))
    }

    /// Compact string used in the `platform_events.source` column.
    pub fn source_label(&self) -> String {
        format!(
            "{}/{}/{}",
            self.source.service, self.source.version, self.source.region
        )
    }
}

/// Append result returned by platform event stores.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredPlatformEvent {
    /// Stored platform event.
    pub event: PlatformEventEnvelope,
}

/// Append-only platform event store.
#[async_trait]
pub trait PlatformEventStore: Send + Sync {
    /// Append the event, linking it to the previous event hash when supported.
    async fn append(
        &self,
        event: PlatformEventEnvelope,
    ) -> Result<StoredPlatformEvent, PlatformEventError>;
}

/// In-memory append-only platform event store for tests/local development.
#[derive(Clone, Default)]
pub struct MemoryPlatformEventStore {
    inner: Arc<Mutex<Vec<PlatformEventEnvelope>>>,
}

impl MemoryPlatformEventStore {
    /// Create an empty memory store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return all stored events in append order.
    pub fn events(&self) -> Result<Vec<PlatformEventEnvelope>, PlatformEventError> {
        self.inner
            .lock()
            .map_err(|_| PlatformEventError::Storage("memory store lock poisoned".to_string()))
            .map(|events| events.clone())
    }
}

#[async_trait]
impl PlatformEventStore for MemoryPlatformEventStore {
    async fn append(
        &self,
        event: PlatformEventEnvelope,
    ) -> Result<StoredPlatformEvent, PlatformEventError> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| PlatformEventError::Storage("memory store lock poisoned".to_string()))?;
        let previous = guard.last().map(|event| event.event_hash.clone());
        let event = event.with_previous_event_hash(previous);
        event.validate()?;
        guard.push(event.clone());
        Ok(StoredPlatformEvent { event })
    }
}

/// Postgres-backed append-only platform event store.
#[derive(Clone)]
pub struct PgPlatformEventStore {
    pool: PgPool,
}

impl PgPlatformEventStore {
    /// Create a Postgres platform event store from an existing pool.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Verify that the `platform_events` table exists and is queryable.
    pub async fn health_check(&self) -> Result<(), PlatformEventError> {
        sqlx::query("SELECT 1 FROM platform_events LIMIT 0")
            .execute(&self.pool)
            .await
            .map_err(|e| PlatformEventError::Storage(e.to_string()))?;
        Ok(())
    }
}

#[async_trait]
impl PlatformEventStore for PgPlatformEventStore {
    async fn append(
        &self,
        event: PlatformEventEnvelope,
    ) -> Result<StoredPlatformEvent, PlatformEventError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| PlatformEventError::Storage(e.to_string()))?;

        let previous_event_hash: Option<String> = sqlx::query(
            r#"
            SELECT event_hash
            FROM platform_events
            ORDER BY created_at DESC, id DESC
            LIMIT 1
            "#,
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| PlatformEventError::Storage(e.to_string()))?
        .and_then(|row| row.try_get::<String, _>("event_hash").ok());

        let event = event.with_previous_event_hash(previous_event_hash);
        event.validate()?;

        let event_id = Uuid::parse_str(&event.event_id)
            .map_err(|_| PlatformEventError::Validation("event_id must be UUID".to_string()))?;
        let workspace_id = parse_optional_uuid("workspace_id", event.workspace_id.as_deref())?;
        let user_id = parse_optional_uuid("user_id", event.user_id.as_deref())?;

        sqlx::query(
            r#"
            INSERT INTO platform_events (
                id,
                event_type,
                event_version,
                occurred_at,
                source,
                workspace_id,
                tenant_id,
                user_id,
                correlation_id,
                causation_id,
                payload,
                payload_sha256,
                previous_event_hash,
                event_hash
            )
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)
            "#,
        )
        .bind(event_id)
        .bind(&event.event_type)
        .bind(event.event_version)
        .bind(event.occurred_at)
        .bind(event.source_label())
        .bind(workspace_id)
        .bind(&event.tenant_id)
        .bind(user_id)
        .bind(&event.correlation_id)
        .bind(&event.causation_id)
        .bind(event.payload.clone())
        .bind(&event.payload_sha256)
        .bind(&event.previous_event_hash)
        .bind(&event.event_hash)
        .execute(&mut *tx)
        .await
        .map_err(|e| PlatformEventError::Storage(e.to_string()))?;

        tx.commit()
            .await
            .map_err(|e| PlatformEventError::Storage(e.to_string()))?;

        Ok(StoredPlatformEvent { event })
    }
}

/// Error type for platform event construction and storage.
#[derive(Debug, Error)]
pub enum PlatformEventError {
    /// Serialization error.
    #[error("serialization error: {0}")]
    Serialization(String),
    /// Validation error.
    #[error("validation error: {0}")]
    Validation(String),
    /// Storage error.
    #[error("storage error: {0}")]
    Storage(String),
}

fn parse_optional_uuid(
    field: &'static str,
    value: Option<&str>,
) -> Result<Option<Uuid>, PlatformEventError> {
    value
        .map(Uuid::parse_str)
        .transpose()
        .map_err(|_| PlatformEventError::Validation(format!("{field} must be UUID")))
}

/// Detect obvious secret material that must not be persisted in platform events.
pub fn contains_obvious_secret_material(value: &Value) -> bool {
    match value {
        Value::Object(map) => map.iter().any(|(key, value)| {
            key_names_secret_material(key)
                || contains_obvious_secret_material(value)
                || string_contains_secret_path(key)
        }),
        Value::Array(items) => items.iter().any(contains_obvious_secret_material),
        Value::String(s) => string_contains_secret_path(s) || string_contains_raw_secret(s),
        _ => false,
    }
}

fn key_names_secret_material(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    lower == "token"
        || lower.ends_with("_token")
        || lower.contains("password")
        || lower.contains("secret")
        || lower.contains("private_key")
        || lower.contains("access_key")
}

fn string_contains_raw_secret(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.contains("-----begin private key-----")
        || lower.contains("-----begin rsa private key-----")
        || lower.contains("aws_secret_access_key")
}

fn string_contains_secret_path(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower == ".env"
        || lower.ends_with("/.env")
        || lower.contains("/.env.")
        || lower.ends_with(".pem")
        || lower.ends_with(".key")
        || lower.ends_with(".crt")
        || lower.contains("/.ssh/")
        || lower.contains("infra/secrets")
        || lower.contains("nginx/certs")
        || lower.contains("keys/")
}
