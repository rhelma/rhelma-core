//! Rhelma Contract v5.2 event envelope (canonical JSON shape).
//!
//! This module is **additive**. It does not replace the existing `EventEnvelope`
//! (v5.1 simplified). Use it as the canonical envelope for new publishers.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::PolicyMeta;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EventEnvelopeV52 {
    /// Field `meta`.
    pub meta: EventMetaV52,
    /// Field `payload`.
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EventMetaV52 {
    /// Globally unique event id (uuidv7 recommended).
    pub event_id: String,

    /// Topic name (e.g. "obs.heartbeat", "ops.audit.user_action").
    pub topic: String,

    /// Strong schema reference (e.g. "obs.heartbeat@v2").
    pub schema_ref: String,

    /// Fully-qualified payload type (e.g. "rhelma.obs.HeartbeatV2").
    pub payload_type: String,

    /// UTC publish time.
    pub published_at: DateTime<Utc>,

    /// Producer info.
    pub source: EventSourceV52,

    /// Request + identity context.
    pub request: EventRequestV52,

    /// Purpose/consent policy metadata.
    #[serde(default)]
    pub policy: PolicyMeta,

    /// Optional trace context (W3C).
    #[serde(default)]
    pub trace: EventTraceV52,

    /// Optional payload hash.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<EventHashV52>,

    /// Optional detached signature (required for audit topics by policy).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<EventSignatureV52>,

    /// Optional encryption metadata (if payload is encrypted).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encryption: Option<EventEncryptionV52>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EventSourceV52 {
    /// Field `service`.
    pub service: String,
    /// Field `version`.
    pub version: String,
    /// Field `region`.
    pub region: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct EventRequestV52 {
    /// Field `request_id`.
    pub request_id: String,
    /// Field `correlation_id`.
    pub correlation_id: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Field `tenant_id`.
    pub tenant_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Field `user_id`.
    pub user_id: Option<String>,

    /// `GLOBAL | REGIONAL_PREFERRED | REGIONAL_STRICT`
    pub residency: String,

    /// W3C traceparent propagated from ingress.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub traceparent: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct EventTraceV52 {
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Field `trace_id`.
    pub trace_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Field `span_id`.
    pub span_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EventHashV52 {
    /// Field `alg`.
    pub alg: String,
    /// Field `value`.
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EventSignatureV52 {
    /// Field `alg`.
    pub alg: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Field `key_id`.
    pub key_id: Option<String>,
    /// Field `value`.
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EventEncryptionV52 {
    /// Field `alg`.
    pub alg: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Field `key_id`.
    pub key_id: Option<String>,
}
