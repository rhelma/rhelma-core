#![forbid(unsafe_code)]

//! Event handling library for Rhelma platform (Contract v5.2).

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use uuid::Uuid;

/// Event contract definitions for typed event handling.
pub mod contracts;
/// Contract v5.2 canonical envelope (additive).
pub mod envelope_v52;
/// Transport-level header helpers (NATS/HTTP/etc.).
pub mod transport_headers;

/// Audit cryptography utilities
pub mod audit_crypto;
/// Audit verification utilities
pub mod audit_verify;
/// Canonicalization utilities
pub mod canonicalization;
/// Platform-level event envelope and append-only stores.
pub mod platform;

/// Observer Evidence contracts (Signal / Incident / EvidenceBundle) — Stage 11B.
pub mod evidence;

pub use audit_crypto::{
    build_audit_failure, verify_audit_envelope, AuditKeyRing, AuditSigError, AuditSigner,
};

/// Source service and region/version.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventSource {
    /// Service name
    pub service: String,
    /// Service version
    pub version: String,
    /// Region name
    pub region: String,
}

impl EventSource {
    /// Creates a new event source
    ///
    /// # Arguments
    /// * `service` - Service name
    /// * `version` - Service version
    /// * `region` - Region name
    ///
    /// # Returns
    /// A new event source instance
    pub fn new(
        service: impl Into<String>,
        version: impl Into<String>,
        region: impl Into<String>,
    ) -> Self {
        Self {
            service: service.into(),
            version: version.into(),
            region: region.into(),
        }
    }
}

/// Residency policy (Contract v5.2 canonical values).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Residency {
    /// Global residency
    Global,
    /// Regional only residency
    RegionalOnly,
    /// Region strict residency
    RegionStrict,
}

impl Residency {
    /// Parses residency from string
    ///
    /// # Arguments
    /// * `input` - Residency string
    ///
    /// # Returns
    /// Residency enum or None if invalid
    pub fn parse(input: &str) -> Option<Self> {
        match input.trim().to_ascii_uppercase().as_str() {
            "GLOBAL" => Some(Self::Global),
            "REGIONAL_ONLY" | "REGIONAL_PREFERRED" => Some(Self::RegionalOnly),
            "REGION_STRICT" | "REGIONAL_STRICT" => Some(Self::RegionStrict),
            _ => None,
        }
    }

    /// Returns residency as canonical contract string
    ///
    /// # Returns
    /// Canonical residency string
    pub fn as_str(&self) -> &'static str {
        match self {
            Residency::Global => "GLOBAL",
            Residency::RegionalOnly => "REGIONAL_ONLY",
            Residency::RegionStrict => "REGION_STRICT",
        }
    }
}

/// Backward-compatible alias.
pub use Residency as EventResidency;
// -----------------------------------------------------------------------------
// Purpose/consent policy metadata (Rhelma6 rule: purpose-bound, consent-verifiable)
// -----------------------------------------------------------------------------

/// Data tier classification for an event payload.
///
/// This aligns with Rhelma6 policy concepts (public commons, consent-based, jury-gated, no-train).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum DataTier {
    /// Public/commons data; may be processed broadly.
    #[default]
    /// Variant `PublicCommons`.
    PublicCommons,
    /// Requires explicit consent to be true.
    ConsentBased,
    /// Sensitive data; gated by jury/guardians policy.
    SensitiveJuryGated,
    /// Private data; must not be used for training/distillation.
    PrivateNoTrain,
}

/// Policy metadata that must travel with the event and be enforceable at boundaries.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PolicyMeta {
    /// Declared purpose for this event (e.g. "chat", "auth", "moderation", "analytics").
    pub purpose: String,
    /// Data classification tier.
    #[serde(default)]
    pub data_tier: DataTier,
    /// Whether user consent is present (required for `ConsentBased`).
    #[serde(default = "PolicyMeta::default_consent")]
    pub consent: bool,
    /// Optional retention/TTL for downstream storage (seconds).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retention_seconds: Option<u64>,
    /// Optional policy ruleset hash / version pin.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy_hash: Option<String>,
}

impl PolicyMeta {
    fn default_consent() -> bool {
        true
    }

    /// Convenience: create a public/commons policy for a given purpose.
    pub fn public(purpose: impl Into<String>) -> Self {
        Self {
            purpose: purpose.into(),
            data_tier: DataTier::PublicCommons,
            consent: true,
            retention_seconds: None,
            policy_hash: None,
        }
    }

    /// Convenience: derive from a parent policy while overriding the publishing purpose.
    ///
    /// This preserves data tier/consent/retention across a processing pipeline, while
    /// making the emitting service's purpose explicit.
    pub fn derived_from(parent: &PolicyMeta, purpose: impl Into<String>) -> Self {
        let mut p = parent.clone();
        p.purpose = purpose.into();
        p
    }
}

impl Default for PolicyMeta {
    fn default() -> Self {
        Self::public("unspecified")
    }
}

/// Canonical purpose strings (service-level registry).
///
/// If `RHELMA_POLICY_ENFORCE_PURPOSE_REGISTRY=1` is set, envelopes must declare a `policy.purpose`
/// that is present in this registry (or in `RHELMA_POLICY_PURPOSE_REGISTRY_EXTRA`).
pub mod purpose {
    pub const AI_ORCH: &str = "ai_orch";
    pub const AUTH: &str = "auth";
    pub const SEARCH: &str = "search";
    pub const SEARCH_ANALYTICS: &str = "search_analytics";
    pub const REALTIME: &str = "realtime";
    pub const API_GATEWAY: &str = "api_gateway";
    pub const NODE_REGISTRY: &str = "node_registry";
    pub const OBSERVABILITY_AGENT: &str = "obs_agent";
    pub const PATCH_APPLIER: &str = "patch_applier";
    pub const SANDBOX_RUNNER: &str = "sandbox_runner";
    pub const VALUE_LEDGER_FEDERATION: &str = "value_ledger_federation";
    pub const BRIDGE_ADAPTER: &str = "bridge_adapter";
    pub const EDGE_WORKER: &str = "edge_worker";
    pub const BRIDGE_DRIVERS: &str = "bridge_drivers";
    pub const OPS_AUDIT: &str = "ops_audit";
    pub const CONTRACTS: &str = "contracts";
    pub const TESTS: &str = "tests";
    pub const KAFKA: &str = "kafka";
}

/// Derive a reasonable `policy.purpose` from the event `topic` prefix.
///
/// This is intended as a **safe fallback** for legacy publishers that haven't been
/// updated to set `policy.purpose` explicitly.
///
/// Enable by setting:
/// - `RHELMA_POLICY_PURPOSE_FROM_TOPIC_FALLBACK=1`
///
/// You can extend/override mappings with:
/// - `RHELMA_POLICY_PURPOSE_TOPIC_MAP_EXTRA="prefix=purpose,prefix2=purpose2"`
///
/// Matching is done with a case-insensitive `starts_with` over the normalized topic.
fn derive_purpose_from_topic(topic: &str) -> Option<String> {
    let t = topic.trim().to_ascii_lowercase();

    // Built-in canonical prefixes.
    let builtin = if t.starts_with("ops.audit")
        || t.starts_with("ops/audit")
        || t.starts_with("ops_audit")
    {
        Some(purpose::OPS_AUDIT)
    } else if t.starts_with("ai.orch")
        || t.starts_with("ai/orch")
        || t.starts_with("ai_orch")
        || t.starts_with("orchestrator.")
        || t.starts_with("ai-orchestrator.")
    {
        Some(purpose::AI_ORCH)
    } else if t.starts_with("auth.")
        || t.starts_with("auth/")
        || t.starts_with("rhelma-auth.")
        || t.starts_with("rhelma_auth.")
    {
        Some(purpose::AUTH)
    } else if t.starts_with("search.analytics")
        || t.starts_with("search/analytics")
        || t.starts_with("search_analytics")
    {
        Some(purpose::SEARCH_ANALYTICS)
    } else if t.starts_with("search.") || t.starts_with("search/") || t.starts_with("search_") {
        Some(purpose::SEARCH)
    } else if t.starts_with("realtime.") || t.starts_with("realtime/") || t.starts_with("realtime_")
    {
        Some(purpose::REALTIME)
    } else if t.starts_with("contracts.")
        || t.starts_with("contracts/")
        || t.starts_with("rhelma-contracts.")
        || t.starts_with("rhelma_contracts.")
    {
        Some(purpose::CONTRACTS)
    } else if t.starts_with("kafka.")
        || t.starts_with("kafka/")
        || t.starts_with("rhelma-event-kafka.")
        || t.starts_with("rhelma_event_kafka.")
    {
        Some(purpose::KAFKA)
    } else {
        None
    };

    // Extra mappings: "prefix=purpose,prefix2=purpose2"
    // These have priority over builtin mappings.
    if let Ok(extra) = std::env::var("RHELMA_POLICY_PURPOSE_TOPIC_MAP_EXTRA") {
        for pair in extra.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            let mut it = pair.splitn(2, '=');
            let prefix = it.next().unwrap_or("").trim().to_ascii_lowercase();
            let purpose = it.next().unwrap_or("").trim();
            if !prefix.is_empty() && !purpose.is_empty() && t.starts_with(&prefix) {
                return Some(purpose.to_string());
            }
        }
    }

    builtin.map(|p| p.to_string())
}

/// Flags describing origin and safety properties.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct EventRequestFlags {
    /// System event flag
    pub system: bool,
    /// AI-safe event flag
    pub ai_safe: bool,
    /// Read-only event flag
    pub read_only: bool,
}

/// Request context propagated with the event.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EventRequestContext {
    /// Optional request ID (uuidv7)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    /// Optional correlation ID (uuidv7)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
    /// Optional tenant ID
    pub tenant_id: Option<String>,
    /// Optional user ID
    pub user_id: Option<String>,
    /// Request flags
    pub flags: EventRequestFlags,
}

impl EventRequestContext {
    /// Inherit fields from `parent` and generate missing request/correlation IDs.
    pub fn inherit_or_generate(parent: &EventRequestContext) -> EventRequestContext {
        let mut out = parent.clone();
        if out.request_id.is_none() {
            out.request_id = Some(Uuid::now_v7().to_string());
        }
        if out.correlation_id.is_none() {
            out.correlation_id = Some(Uuid::now_v7().to_string());
        }
        out
    }
}

/// Distributed tracing context (W3C IDs).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EventTraceContext {
    /// Optional trace ID (32 hex)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    /// Optional span ID (16 hex)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span_id: Option<String>,
    /// Optional W3C tracestate header value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tracestate: Option<String>,
    /// Optional W3C baggage header value (bounded allowlist recommended)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub baggage: Option<String>,
    /// Optional parent span ID
    pub parent_span_id: Option<String>,
}

impl EventTraceContext {
    /// Generates a new trace context with random IDs
    ///
    /// # Returns
    /// A new trace context with generated IDs
    pub fn generate() -> Self {
        // IMPORTANT: span_id must not be derived from the timestamp prefix of UUIDv7, otherwise two
        // spans created in the same millisecond can collide. Use the shared helpers below.
        Self {
            trace_id: Some(generate_trace_id()),
            span_id: Some(generate_span_id()),
            tracestate: None,
            baggage: None,
            parent_span_id: None,
        }
    }

    /// Creates a child span context from a parent, preserving trace_id and linking parent_span_id.
    pub fn child_of(parent: &EventTraceContext) -> Self {
        let mut out = Self::generate();
        if let Some(t) = parent.trace_id.as_ref() {
            out.trace_id = Some(t.clone());
        }
        // Preserve tracestate/baggage across hops when present.
        out.tracestate = parent.tracestate.clone();
        out.baggage = parent.baggage.clone();
        out.parent_span_id = parent.span_id.clone();

        // Defensive: ensure we never reuse the parent's span_id.
        if out.span_id == parent.span_id {
            for _ in 0..5 {
                out.span_id = Some(generate_span_id());
                if out.span_id != parent.span_id {
                    break;
                }
            }
        }

        out
    }
}

/// Contract v5.2 event envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEnvelope {
    // Identity
    /// Event ID (uuidv7)
    pub event_id: String,
    /// Event version
    pub event_version: i32,

    // Routing
    /// Event topic
    pub topic: String,
    /// Optional key for partitioning
    pub key: Option<String>,

    // Timestamps
    /// Event timestamp
    pub timestamp: DateTime<Utc>,
    /// Publication timestamp
    pub published_at: DateTime<Utc>,

    // Source & context
    /// Event source
    pub source: EventSource,
    /// Request context
    pub request: EventRequestContext,
    /// Trace context
    pub trace: EventTraceContext,

    // Payload
    /// Event payload (JSON)
    pub payload: Value,
    /// Payload type identifier
    pub payload_type: String,
    /// Schema reference
    pub schema_ref: String,

    // Residency & security
    /// Purpose/consent policy metadata
    #[serde(default)]
    pub policy: PolicyMeta,
    /// Residency policy
    pub residency: Residency,
    /// Optional encryption metadata
    pub encryption: Option<Value>,

    // Integrity
    /// Optional signature
    pub signature: Option<String>,
    /// Optional hash
    pub hash: Option<String>,
}

// -----------------------------------------------------------------------------
// Contract v5.2 enforcement helpers
// -----------------------------------------------------------------------------

fn is_registered_purpose(p: &str) -> bool {
    let p = p.trim();
    if p.is_empty() {
        return false;
    }
    // Built-in registry
    let builtin = matches!(
        p,
        purpose::AI_ORCH
            | purpose::AUTH
            | purpose::SEARCH
            | purpose::SEARCH_ANALYTICS
            | purpose::REALTIME
            | purpose::API_GATEWAY
            | purpose::NODE_REGISTRY
            | purpose::OBSERVABILITY_AGENT
            | purpose::VALUE_LEDGER_FEDERATION
            | purpose::BRIDGE_ADAPTER
            | purpose::EDGE_WORKER
            | purpose::BRIDGE_DRIVERS
            | purpose::PATCH_APPLIER
            | purpose::SANDBOX_RUNNER
            | purpose::OPS_AUDIT
            | purpose::CONTRACTS
            | purpose::TESTS
            | purpose::KAFKA
    );
    if builtin {
        return true;
    }
    // Optional extra registry (comma-separated)
    if let Ok(extra) = std::env::var("RHELMA_POLICY_PURPOSE_REGISTRY_EXTRA") {
        for item in extra.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
            if item == p {
                return true;
            }
        }
    }
    false
}

impl EventEnvelope {
    /// Validate and normalize an envelope to satisfy the v5.2 publishing contract.
    ///
    /// This is intended to run at the **publish boundary** (EventBus implementations),
    /// so malformed/partial envelopes fail fast before hitting the transport.
    ///
    /// Rules:
    /// - `event_id` must be a valid UUID (v7 recommended).
    /// - `request.request_id` and `request.correlation_id` must exist and be valid UUIDs.
    /// - `trace.trace_id` must be 32-char lowercase hex; `trace.span_id` must be 16-char lowercase hex.
    ///   If trace fields are missing, they will be generated.
    /// - `schema_ref` must be present and non-empty.
    /// - topics starting with `ops.audit` must include a non-empty `signature`.
    /// - legacy mirror fields (`region`, `tenant_id`) are auto-filled if missing.
    ///
    /// # Returns
    /// `Result<Self, EventBusError>` - Validated envelope or error
    pub fn finalize_strict(mut self) -> Result<Self, EventBusError> {
        // event_id
        if self.event_id.trim().is_empty() {
            self.event_id = generate_event_id();
        }
        validate_uuid("event_id", &self.event_id)?;

        // schema_ref
        if self.schema_ref.trim().is_empty() {
            return Err(contract_err("missing schema_ref"));
        }

        // policy (Rhelma6): purpose-bound + consent-verifiable
        {
            let purpose = self.policy.purpose.trim();
            if purpose.is_empty() {
                self.policy.purpose = "unspecified".to_string();
            }

            // Optional fallback: infer purpose from topic prefix for legacy publishers.
            if std::env::var("RHELMA_POLICY_PURPOSE_FROM_TOPIC_FALLBACK")
                .ok()
                .as_deref()
                == Some("1")
                && self.policy.purpose == "unspecified"
            {
                if let Some(p) = derive_purpose_from_topic(&self.topic) {
                    self.policy.purpose = p;
                }
            }
            if std::env::var("RHELMA_POLICY_REQUIRE_PURPOSE")
                .ok()
                .as_deref()
                == Some("1")
                && self.policy.purpose == "unspecified"
            {
                return Err(contract_err("missing policy.purpose"));
            }
            if std::env::var("RHELMA_POLICY_ENFORCE_PURPOSE_REGISTRY")
                .ok()
                .as_deref()
                == Some("1")
                && !is_registered_purpose(&self.policy.purpose)
            {
                return Err(contract_err("unknown policy.purpose"));
            }

            if matches!(self.policy.data_tier, DataTier::ConsentBased) && !self.policy.consent {
                return Err(contract_err(
                    "policy.consent must be true when policy.data_tier=consent_based",
                ));
            }
            if matches!(self.policy.data_tier, DataTier::PrivateNoTrain) {
                let p = self.policy.purpose.to_ascii_lowercase();
                if matches!(
                    p.as_str(),
                    "training" | "train" | "distill" | "distillation" | "fine_tune" | "finetune"
                ) {
                    return Err(contract_err(
                        "policy.purpose not allowed for policy.data_tier=private_no_train",
                    ));
                }
            }
            if let Some(ttl) = self.policy.retention_seconds {
                if ttl == 0 {
                    return Err(contract_err("policy.retention_seconds must be > 0"));
                }
            }
        }

        // request context
        let rid = self
            .request
            .request_id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| contract_err("missing request.request_id"))?;
        validate_uuid_v7("request.request_id", rid)?;

        let cid = self
            .request
            .correlation_id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| contract_err("missing request.correlation_id"))?;
        validate_uuid_v7("request.correlation_id", cid)?;

        // trace context (generate if absent)
        if self
            .trace
            .trace_id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .is_none()
        {
            self.trace.trace_id = Some(generate_trace_id());
        }
        if self
            .trace
            .span_id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .is_none()
        {
            self.trace.span_id = Some(generate_span_id());
        }

        let trace_id = self.trace.trace_id.as_deref().unwrap();
        let span_id = self.trace.span_id.as_deref().unwrap();

        if trace_id.len() != 32 || !is_lower_hex(trace_id) || trace_id.chars().all(|c| c == '0') {
            return Err(contract_err("invalid trace.trace_id"));
        }
        if span_id.len() != 16 || !is_lower_hex(span_id) || span_id.chars().all(|c| c == '0') {
            return Err(contract_err("invalid trace.span_id"));
        }

        // audit signature policy
        if self.topic.starts_with("ops.audit") {
            // Hash policy: ops.audit* MUST include payload hash (sha256 hex of canonical payload).
            // We compute it deterministically and either fill it or validate a provided value.
            let computed_hash =
                crate::canonicalization::canonical_audit_payload_hash_hex(&self.payload);
            match self
                .hash
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                None => {
                    self.hash = Some(computed_hash.clone());
                }
                Some(existing) => {
                    if !existing.eq_ignore_ascii_case(&computed_hash) {
                        return Err(contract_err("hash mismatch for ops.audit* event"));
                    }
                }
            }

            // Signature policy: ops.audit* MUST include a signature.
            // If one is not provided, we attempt to auto-sign using RHELMA_AUDIT_SIGNING_KEY.
            // This avoids a common footgun at publish boundaries while keeping the policy strict.
            if self
                .signature
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .is_none()
            {
                match crate::audit_crypto::AuditSigner::from_env() {
                    Ok(signer) => {
                        let digest = crate::audit_crypto::audit_payload_digest(&self.payload);
                        self.signature = Some(signer.sign_digest(&digest));
                    }
                    Err(_) => {
                        // If a signing key is *configured* but invalid, surface a clearer error.
                        if std::env::var("RHELMA_AUDIT_SIGNING_KEY").is_ok() {
                            return Err(contract_err("invalid RHELMA_AUDIT_SIGNING_KEY"));
                        }
                        return Err(contract_err("missing signature for ops.audit* event"));
                    }
                }
            }

            // Enforce signature format for audit topics: ed25519[:key_id]:base64(64 bytes)
            validate_audit_signature_format(self.signature.as_deref().unwrap())?;
        }
        // Normalize empty hash/signature
        if self
            .hash
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .is_none()
        {
            self.hash = None;
        }
        if self
            .signature
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .is_none()
        {
            self.signature = None;
        }

        Ok(self)
    }
}

impl EventEnvelope {
    /// Autofill missing v5.2 context fields from the current in-process observability context.
    ///
    /// This is a **publish-boundary helper**: it makes it harder for producers to accidentally
    /// drop request/correlation/trace propagation. If values already exist on the envelope,
    /// they are preserved.
    ///
    /// Rules (best-effort):
    /// - If `request.request_id` is missing: take it from `rhelma_tracing::context::current_request_id()`
    ///   if present and valid UUIDv7; otherwise generate a new UUIDv7.
    /// - If `request.correlation_id` is missing: take it from `rhelma_tracing::context::current_correlation_id()`
    ///   if present and valid UUIDv7; otherwise generate a new UUIDv7.
    /// - If `trace.trace_id` is missing: take it from `rhelma_tracing::context::current_trace_id()` if valid,
    ///   otherwise generate a new trace-id.
    /// - If `trace.span_id` is missing: create a **child span** of the current span (if any):
    ///   set `parent_span_id = current_span_id`, generate a fresh `span_id`.
    pub fn autofill_from_current_context(mut self) -> Self {
        use uuid::{Uuid, Version};

        fn is_uuid_v7(s: &str) -> bool {
            Uuid::parse_str(s).ok().and_then(|u| u.get_version()) == Some(Version::SortRand)
        }

        // request_id
        let rid_missing = self
            .request
            .request_id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .is_none();
        if rid_missing {
            let from_ctx = rhelma_tracing::context::current_request_id();
            let rid = from_ctx
                .as_deref()
                .filter(|s| is_uuid_v7(s))
                .map(|s| s.to_string())
                .unwrap_or_else(|| Uuid::now_v7().to_string());
            self.request.request_id = Some(rid);
        }

        // correlation_id
        let cid_missing = self
            .request
            .correlation_id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .is_none();
        if cid_missing {
            let from_ctx = rhelma_tracing::context::current_correlation_id();
            let cid = from_ctx
                .as_deref()
                .filter(|s| is_uuid_v7(s))
                .map(|s| s.to_string())
                .unwrap_or_else(|| Uuid::now_v7().to_string());
            self.request.correlation_id = Some(cid);
        }

        // trace_id
        let tid_missing = self
            .trace
            .trace_id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .is_none();
        if tid_missing {
            let from_ctx = rhelma_tracing::context::current_trace_id();
            let tid = from_ctx
                .as_deref()
                .filter(|s| s.len() == 32 && s.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f')))
                .map(|s| s.to_string())
                .unwrap_or_else(generate_trace_id);
            self.trace.trace_id = Some(tid);
        }

        // span_id: create child span if missing
        let sid_missing = self
            .trace
            .span_id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .is_none();
        if sid_missing {
            let parent = rhelma_tracing::context::current_span_id()
                .as_deref()
                .filter(|s| s.len() == 16 && s.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f')))
                .map(|s| s.to_string());

            if self.trace.parent_span_id.is_none() {
                self.trace.parent_span_id = parent;
            }
            self.trace.span_id = Some(generate_span_id());
        }

        // residency (best-effort)
        if matches!(self.residency, Residency::Global) {
            if let Some(r) = rhelma_tracing::context::current_residency() {
                if let Some(parsed) = Residency::parse(&r) {
                    self.residency = parsed;
                }
            }
        }

        self
    }

    /// Finalize an envelope at the publish boundary:
    /// - autofill from current context
    /// - then enforce strict v5.2 requirements
    pub fn finalize_publish_boundary(self) -> Result<Self, EventBusError> {
        self.autofill_from_current_context().finalize_strict()
    }
}

/// Creates a contract violation error
fn contract_err(msg: &str) -> EventBusError {
    EventBusError::Serialization(format!("contract violation: {msg}"))
}

/// Validates a UUID string
fn validate_uuid(field: &str, s: &str) -> Result<(), EventBusError> {
    Uuid::parse_str(s)
        .map(|_| ())
        .map_err(|_| contract_err(&format!("invalid {field} (must be UUID)")))
}

/// Validates a UUIDv7 string
fn validate_uuid_v7(field: &str, s: &str) -> Result<(), EventBusError> {
    use uuid::{Uuid, Version};

    let u =
        Uuid::parse_str(s).map_err(|_| contract_err(&format!("invalid {field} (must be UUID)")))?;

    // v7 == SortRand in uuid crate naming
    if u.get_version() != Some(Version::SortRand) {
        return Err(contract_err(&format!("invalid {field} (must be UUIDv7)")));
    }

    Ok(())
}

/// Validates audit signature format
fn validate_audit_signature_format(sig: &str) -> Result<(), EventBusError> {
    use base64::{engine::general_purpose::STANDARD, Engine as _};

    let s = sig.trim();
    if !s.starts_with("ed25519:") {
        return Err(contract_err(
            "invalid signature for ops.audit* event (expected 'ed25519:...')",
        ));
    }

    let rest = &s["ed25519:".len()..];
    let parts: Vec<&str> = rest.split(':').collect();

    let b64 = match parts.as_slice() {
        // ed25519:<b64>
        [b64] => *b64,
        // ed25519:<key_id>:<b64>
        [_, b64] => *b64,
        _ => {
            return Err(contract_err(
                "invalid signature for ops.audit* event (bad format)",
            ))
        }
    };

    let bytes = STANDARD
        .decode(b64)
        .map_err(|_| contract_err("invalid signature for ops.audit* event (bad base64)"))?;

    if bytes.len() != 64 {
        return Err(contract_err(
            "invalid signature for ops.audit* event (expected 64 bytes)",
        ));
    }

    Ok(())
}

/// Helper to format an ed25519 signature string (with optional key_id).
#[allow(dead_code)]
fn format_ed25519_signature(key_id: Option<&str>, sig64: &[u8; 64]) -> String {
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    let b64 = STANDARD.encode(sig64);
    match key_id.map(str::trim).filter(|s| !s.is_empty()) {
        Some(k) => format!("ed25519:{k}:{b64}"),
        None => format!("ed25519:{b64}"),
    }
}

/// Checks if string contains only lowercase hex characters
fn is_lower_hex(s: &str) -> bool {
    s.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f'))
}

/// Generates a trace ID
fn generate_trace_id() -> String {
    uuid::Uuid::now_v7()
        .to_string()
        .replace('-', "")
        .to_ascii_lowercase()
}

/// Generates a span ID
fn generate_span_id() -> String {
    // W3C traceparent span-id is 8 bytes (16 hex).
    //
    // With UUIDv7, the leading bits are largely timestamp-derived; taking the
    // *prefix* can collide when multiple spans are generated within the same
    // millisecond. Use the *suffix* instead, which carries the randomized part.
    let s = uuid::Uuid::now_v7()
        .to_string()
        .replace('-', "")
        .to_ascii_lowercase();

    // 8 bytes => 16 hex
    s[s.len() - 16..].to_string()
}

/// Error type for event bus operations.
#[derive(Debug, Error)]
pub enum EventBusError {
    /// Transport error
    #[error("transport error: {0}")]
    /// Variant `Transport`.
    Transport(String),
    /// Serialization error
    #[error("serialization error: {0}")]
    /// Variant `Serialization`.
    Serialization(String),
    /// Validation error
    #[error("validation error: {0}")]
    /// Variant `Validation`.
    Validation(String),
}

/// Event bus interface.
#[async_trait]
pub trait EventBus: Send + Sync {
    /// Publishes an event
    ///
    /// # Arguments
    /// * `event` - Event envelope to publish
    ///
    /// # Returns
    /// `Result<(), EventBusError>` - Success or error
    async fn publish(&self, event: EventEnvelope) -> Result<(), EventBusError>;
}

fn enrich_envelope_from_local_context(mut event: EventEnvelope) -> EventEnvelope {
    // Best-effort propagation from rhelma-tracing's local context.
    //
    // This improves cross-service correlation for Kafka events, since the Kafka transport
    // layer can forward W3C `traceparent` and canonical Rhelma request/correlation ids.
    //
    // If no local context exists, this is a no-op except for generating trace ids as needed.
    if event.trace.trace_id.is_none() {
        event.trace.trace_id = rhelma_tracing::context::current_trace_id();
    }
    if event.trace.span_id.is_none() {
        event.trace.span_id = rhelma_tracing::context::current_span_id();
    }
    if event.trace.tracestate.is_none() {
        event.trace.tracestate = rhelma_tracing::context::current_tracestate();
    }
    if event.trace.baggage.is_none() {
        event.trace.baggage = rhelma_tracing::context::current_baggage();
    }

    // Prefer ids from the current request scope when available (override per-event defaults).
    if let Some(rid) = rhelma_tracing::context::current_request_id() {
        event.request.request_id = Some(rid);
    }
    if let Some(cid) = rhelma_tracing::context::current_correlation_id() {
        event.request.correlation_id = Some(cid);
    }

    if event.request.tenant_id.is_none() {
        event.request.tenant_id = rhelma_tracing::context::current_tenant_id();
    }

    // Residency is advisory for routing/audit; only apply if parseable.
    if let Some(res) = rhelma_tracing::context::current_residency()
        .as_deref()
        .and_then(Residency::parse)
    {
        event.residency = res;
    }

    // Source region is required for some deployments; fill if the envelope left it blank.
    if event.source.region.trim().is_empty() {
        if let Some(region) = rhelma_tracing::context::current_region() {
            event.source.region = region;
        }
    }

    event
}

/// Publish an event with standard tracing span.
///
/// This helper finalizes + validates the envelope before publishing.
///
/// # Arguments
/// * `bus` - Event bus instance
/// * `event` - Event envelope to publish
///
/// # Returns
/// `Result<(), EventBusError>` - Success or error
pub async fn publish_with_observability<B: EventBus + ?Sized>(
    bus: &B,
    event: EventEnvelope,
) -> Result<(), EventBusError> {
    let event = enrich_envelope_from_local_context(event);
    let event = event.finalize_publish_boundary()?;
    let span = tracing::info_span!("rhelma_event_publish", topic = %event.topic);
    let _enter = span.enter();
    bus.publish(event).await
}

/// Generate a globally unique event id.
///
/// # Returns
/// UUIDv7 string
pub fn generate_event_id() -> String {
    Uuid::now_v7().to_string()
}
