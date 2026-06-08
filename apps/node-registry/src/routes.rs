#![forbid(unsafe_code)]

use axum::{
    extract::{ConnectInfo, Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use serde::Deserialize;
use subtle::ConstantTimeEq;
use tracing::{info, warn};

use crate::{
    crypto::verify_signature_hex,
    error::RegistryError,
    models::{
        AdmissionChallengeResponseV1, DiscoverResponseV1, NodeAttestRequestV1,
        NodeAttestResponseV1, NodeHeartbeatRequestV1, NodeHeartbeatResponseV1,
        NodeRegisterRequestV1, NodeRegisterResponseV1, NodeReportRequestV1, NodeReportResponseV1,
    },
    state::{AppState, SharedState},
};

/// Start background tasks (best-effort) for the registry.
pub fn spawn_background_tasks(state: SharedState) {
    let prune_state = state.clone();
    tokio::spawn(async move {
        let interval = prune_state.cfg.tuning.prune_interval;
        loop {
            tokio::time::sleep(interval).await;
            let pruned = prune_state
                .store
                .prune_stale(prune_state.cfg.tuning.node_ttl)
                .await;
            if pruned > 0 {
                info!("pruned {pruned} stale nodes");
            }
        }
    });
}

#[must_use = "router should be used"]
pub fn build_public_router(state: SharedState) -> Router {
    let router = Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/metrics", get(crate::metrics_endpoint::metrics_handler))
        .route("/docs", get(crate::docs::docs_landing))
        .route("/api-docs/openapi.json", get(crate::docs::openapi_json))
        .route("/v1/admission/challenge", get(admission_challenge))
        .route("/v1/nodes/register", post(register_node))
        .route("/v1/nodes/heartbeat", post(heartbeat))
        .route("/v1/nodes/attest", post(attest))
        .route("/v1/nodes/discover", get(discover))
        .with_state(state);

    #[cfg(feature = "openapi")]
    let router = router.merge(crate::openapi::swagger_router());

    apply_common_layers(router)
}

#[must_use = "router should be used"]
pub fn build_internal_router(state: SharedState) -> Router {
    let router = Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/docs", get(crate::docs::docs_landing))
        .route("/api-docs/openapi.json", get(crate::docs::openapi_json))
        .route("/v1/internal/nodes/report", post(report_outcome))
        .with_state(state);

    #[cfg(feature = "openapi")]
    let router = router.merge(crate::openapi::swagger_router());

    apply_common_layers(router)
}

fn apply_common_layers(app: Router) -> Router {
    app
        // Stage 2 hardening: rate limit + audit logs for internal/admin endpoints.
        .layer(rhelma_http_observability::security::rate_limit_layer_sensitive())
        .layer(rhelma_http_observability::security::audit_layer_sensitive())
        // Stage 3: idempotency + backpressure for safe retries and load shedding.
        .layer(rhelma_http_observability::security::idempotency_layer())
        .layer(rhelma_http_observability::security::concurrency_limit_layer())
        // Standard v6.0 observability stack.
        .layer(rhelma_http_observability::axum::trace_layer_v60())
        .layer(rhelma_http_observability::axum::ScopeHeadersLayer)
        .layer(rhelma_http_observability::axum::ContractV60Layer)
}

async fn healthz() -> (StatusCode, &'static str) {
    (StatusCode::OK, "ok")
}

async fn readyz() -> (StatusCode, &'static str) {
    (StatusCode::OK, "ready")
}

#[derive(Debug, Deserialize)]
struct AdmissionChallengeQuery {
    /// Optional node_id to bind the challenge.
    node_id: Option<String>,
}

async fn admission_challenge(
    State(state): State<SharedState>,
    ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>,
    Query(q): Query<AdmissionChallengeQuery>,
) -> Result<Json<AdmissionChallengeResponseV1>, RegistryError> {
    if !state.cfg.admission.pow_enabled {
        return Err(RegistryError::not_found("pow admission disabled"));
    }

    let ip = addr.ip().to_string();

    // Rate-limit challenge issuance.
    let allowed = match &state.admission {
        crate::state::AdmissionBackend::Memory(m) => {
            let mut adm = m.lock().await;
            adm.prune_expired(chrono::Utc::now().timestamp());
            let count = adm
                .register_counter
                .bump(&ip, state.cfg.admission.register_rate_limit_ttl);
            count <= state.cfg.admission.register_rate_limit_max
        }
        crate::state::AdmissionBackend::Redis(r) => r
            .rate_limit_allow(&ip)
            .await
            .map_err(|e| RegistryError::internal(format!("redis rate_limit: {e}")))?,
    };

    if !allowed {
        return Err(RegistryError::too_many_requests("rate limit exceeded"));
    }

    // Issue challenge.
    let nonce = crate::admission::pow::generate_nonce();
    let nonce_hex = hex::encode(nonce);

    let now_unix = chrono::Utc::now().timestamp();
    let ttl_secs = state
        .cfg
        .admission
        .pow_challenge_ttl
        .as_secs()
        .min(u64::from(i64::MAX as u32));

    let rec = crate::admission::AdmissionChallengeRecord {
        difficulty_bits: u32::from(state.cfg.admission.pow_difficulty_bits),
        expires_at_unix: now_unix.saturating_add(ttl_secs as i64),
        node_id: q.node_id,
    };

    match &state.admission {
        crate::state::AdmissionBackend::Memory(m) => {
            let mut adm = m.lock().await;
            adm.prune_expired(now_unix);
            adm.challenges.insert(nonce_hex.clone(), rec.clone());
        }
        crate::state::AdmissionBackend::Redis(r) => r
            .put_challenge(&nonce_hex, &rec)
            .await
            .map_err(|e| RegistryError::internal(format!("redis put_challenge: {e}")))?,
    }

    Ok(Json(AdmissionChallengeResponseV1 {
        nonce_hex,
        difficulty_bits: rec.difficulty_bits,
        expires_unix: rec.expires_at_unix,
    }))
}

async fn register_node(
    State(state): State<SharedState>,
    Json(req): Json<NodeRegisterRequestV1>,
) -> Result<Json<NodeRegisterResponseV1>, RegistryError> {
    validate_manifest(&req.manifest, &state.cfg.policy)?;

    // Verify signature (optionally required by policy).
    verify_manifest_signature(&req.manifest, state.cfg.policy.require_manifest_signature)?;

    // Permissionless admission controls (opt-in via env).
    enforce_admission(&state, &req).await?;

    state.store.upsert_manifest(req.manifest.clone()).await?;

    Ok(Json(NodeRegisterResponseV1 {
        ok: true,
        node_id: req.manifest.node_id,
    }))
}

async fn attest(
    State(state): State<SharedState>,
    Json(req): Json<NodeAttestRequestV1>,
) -> Result<Json<NodeAttestResponseV1>, RegistryError> {
    // Load existing record (to use its public key).
    let summary = state
        .store
        .get(&req.node_id)
        .await
        .ok_or_else(|| RegistryError::bad_request("unknown node_id"))?;

    // Canonical payload: same object with signature_hex = None
    let mut canonical = req.clone();
    canonical.signature_hex = None;
    let payload = serde_json::to_vec(&canonical)
        .map_err(|e| RegistryError::bad_request(format!("invalid attest payload: {e}")))?;

    let sig = req
        .signature_hex
        .as_deref()
        .ok_or_else(|| RegistryError::bad_request("signature_hex is required"))?;

    let ok = verify_signature_hex(&payload, sig, &summary.node_id)?;
    if !ok {
        return Err(RegistryError::unauthorized("invalid signature"));
    }

    let kind = req.attestation.kind.trim().to_ascii_lowercase();
    let evidence = req.attestation.evidence.as_deref().unwrap_or("").trim();

    const MAX_EVIDENCE_LEN: usize = 16 * 1024;
    if evidence.len() > MAX_EVIDENCE_LEN {
        return Err(RegistryError::bad_request("evidence too large"));
    }

    let is_hardware_kind = matches!(
        kind.as_str(),
        "tpm" | "tpm2" | "sgx" | "sev-snp" | "sev_snp" | "snp"
    );

    if kind == "none" {
        if !evidence.is_empty() {
            return Err(RegistryError::bad_request(
                "attestation.kind=none must not include evidence",
            ));
        }
    } else if is_hardware_kind && evidence.is_empty() {
        return Err(RegistryError::bad_request(
            "hardware attestation requires non-empty evidence",
        ));
    } else if evidence.is_empty() {
        if state.cfg.policy.require_attestation_evidence {
            return Err(RegistryError::bad_request(
                "attestation evidence required by policy",
            ));
        }
        warn!(node_id = %req.node_id, kind = %kind, "attestation evidence missing (accepted for non-hardware kinds)");
    }

    validate_attestation_evidence(&kind, evidence)?;

    let decision = crate::attestation_verify::verify_attestation(
        &state.cfg.policy,
        &req.node_id,
        &summary.node_id,
        &req.attestation,
    )
    .await?;

    let is_attested = decision.attested;
    state
        .store
        .set_attestation(&req.node_id, is_attested, req.attestation.clone())
        .await?;

    Ok(Json(NodeAttestResponseV1 {
        ok: true,
        node_id: req.node_id,
        attested: is_attested,
    }))
}

async fn heartbeat(
    State(state): State<SharedState>,
    Json(req): Json<NodeHeartbeatRequestV1>,
) -> Result<Json<NodeHeartbeatResponseV1>, RegistryError> {
    state.store.heartbeat(req.clone()).await?;

    let next = state
        .cfg
        .tuning
        .node_ttl
        .as_secs()
        .saturating_div(2)
        .max(10);
    Ok(Json(NodeHeartbeatResponseV1 {
        ok: true,
        node_id: req.node_id,
        next_heartbeat_seconds: next,
    }))
}

#[derive(Debug, Deserialize)]
pub struct DiscoverQuery {
    /// Field `capability`.
    pub capability: Option<String>,
    /// Field `region`.
    pub region: Option<String>,
    /// Field `residency`.
    pub residency: Option<String>,
    /// Field `limit`.
    pub limit: Option<usize>,

    /// Optional filters.
    pub min_reputation: Option<i32>,
    /// Field `require_attested`.
    pub require_attested: Option<bool>,
    /// Field `status`.
    pub status: Option<String>,
}

async fn discover(
    State(state): State<SharedState>,
    Query(q): Query<DiscoverQuery>,
) -> Result<Json<DiscoverResponseV1>, RegistryError> {
    let limit = q.limit.unwrap_or(25).min(200);

    let min_rep = q
        .min_reputation
        .or(Some(state.cfg.policy.default_min_reputation));
    let req_att = q
        .require_attested
        .or(Some(state.cfg.policy.default_require_attested));

    let nodes = state
        .store
        .discover(crate::store::DiscoverFilter {
            capability: q.capability.as_deref(),
            region: q.region.as_deref(),
            residency: q.residency.as_deref(),
            min_reputation: min_rep,
            require_attested: req_att,
            only_status: q.status.as_deref(),
            limit,
        })
        .await;

    Ok(Json(DiscoverResponseV1 { nodes }))
}

/// Report task outcomes for reputation updates.
///
/// This endpoint is intentionally under `/v1/internal` and requires `RHELMA_NODE_REGISTRY__ADMIN_TOKEN`.
async fn report_outcome(
    State(state): State<SharedState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<NodeReportRequestV1>,
) -> Result<Json<NodeReportResponseV1>, RegistryError> {
    let token = match &state.cfg.admin_token {
        Some(t) => t,
        None => return Err(RegistryError::not_found("endpoint disabled")),
    };

    let provided = headers
        .get("x-registry-admin-token")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if !constant_time_eq(provided, token.as_str()) {
        return Err(RegistryError::unauthorized("invalid admin token"));
    }

    let (rep, status) = state
        .store
        .report_outcome(req.clone(), &state.cfg.policy)
        .await?;
    Ok(Json(NodeReportResponseV1 {
        ok: true,
        node_id: req.node_id,
        reputation: rep,
        status,
    }))
}

fn validate_manifest(
    m: &crate::models::NodeManifestV1,
    policy: &crate::config::NodeRegistryPolicy,
) -> Result<(), RegistryError> {
    // Minimal checks (signature/attestation can be enforced by policy).
    // NodeId should be a 32-byte ed25519 public key in lower hex (64 chars).
    let s = m.node_id.trim();
    if s.len() != 64 {
        return Err(RegistryError::bad_request(
            "node_id must be 64-char lower-hex public key",
        ));
    }
    if !s
        .chars()
        .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
    {
        return Err(RegistryError::bad_request("node_id must be lower-hex"));
    }
    if m.public_key_hex.trim() != s {
        return Err(RegistryError::bad_request(
            "public_key_hex must match node_id",
        ));
    }
    if m.region.trim().is_empty() {
        return Err(RegistryError::bad_request("region is required"));
    }
    if m.capabilities.is_empty() {
        return Err(RegistryError::bad_request("capabilities must be non-empty"));
    }
    if m.allowed_residencies.is_empty() {
        return Err(RegistryError::bad_request(
            "allowed_residencies must be non-empty",
        ));
    }

    // Manifest-level attestation validation (format-only).
    if let Some(att) = &m.attestation {
        let kind = att.kind.trim().to_ascii_lowercase();
        let evidence = att.evidence.as_deref().unwrap_or("").trim();

        if kind == "none" {
            if !evidence.is_empty() {
                return Err(RegistryError::bad_request(
                    "attestation.kind=none must not include evidence",
                ));
            }
        } else if evidence.is_empty() && policy.require_attestation_evidence {
            return Err(RegistryError::bad_request(
                "attestation evidence required by policy",
            ));
        }

        validate_attestation_evidence(&kind, evidence)?;
    }
    Ok(())
}

fn verify_manifest_signature(
    m: &crate::models::NodeManifestV1,
    require: bool,
) -> Result<(), RegistryError> {
    // If signature_hex is present, verify; if absent, allow unless policy requires it.
    let Some(sig) = m.signature_hex.as_deref() else {
        if require {
            return Err(RegistryError::bad_request(
                "manifest signature_hex is required by policy",
            ));
        }
        return Ok(());
    };

    let mut canonical = m.clone();
    canonical.signature_hex = None;
    let payload = serde_json::to_vec(&canonical)
        .map_err(|e| RegistryError::bad_request(format!("invalid manifest: {e}")))?;
    let ok = verify_signature_hex(&payload, sig, &m.public_key_hex)?;
    if !ok {
        return Err(RegistryError::unauthorized("invalid manifest signature"));
    }
    Ok(())
}

async fn enforce_admission(
    state: &AppState,
    req: &NodeRegisterRequestV1,
) -> Result<(), RegistryError> {
    if !state.cfg.admission.pow_enabled {
        return Ok(());
    }

    let proof = req
        .admission
        .as_ref()
        .ok_or_else(|| RegistryError::bad_request("pow required: missing admission proof"))?;

    // Decode nonce.
    let nonce_bytes = hex::decode(&proof.nonce_hex)
        .map_err(|_| RegistryError::bad_request("invalid nonce_hex"))?;
    if nonce_bytes.len() != 32 {
        return Err(RegistryError::bad_request("nonce must be 32 bytes"));
    }
    let mut nonce = [0u8; 32];
    nonce.copy_from_slice(&nonce_bytes[..32]);

    // Decode solution.
    let solution = hex::decode(&proof.solution_hex)
        .map_err(|_| RegistryError::bad_request("invalid solution_hex"))?;

    let now_unix = chrono::Utc::now().timestamp();

    // Consume challenge (prevents replay).
    let rec = match &state.admission {
        crate::state::AdmissionBackend::Memory(m) => {
            let mut adm = m.lock().await;
            adm.prune_expired(now_unix);
            adm.challenges
                .remove(&proof.nonce_hex)
                .ok_or_else(|| RegistryError::bad_request("unknown or already-used challenge"))?
        }
        crate::state::AdmissionBackend::Redis(r) => r
            .take_challenge(&proof.nonce_hex)
            .await
            .map_err(|e| RegistryError::internal(format!("redis take_challenge: {e}")))?
            .ok_or_else(|| RegistryError::bad_request("unknown or already-used challenge"))?,
    };

    if rec.is_expired(now_unix) {
        return Err(RegistryError::bad_request("challenge expired"));
    }

    // Optional binding: if challenge was issued for a node_id, require match.
    if let Some(bound) = rec.node_id {
        if bound != req.manifest.node_id {
            return Err(RegistryError::bad_request(
                "challenge not issued for this node_id",
            ));
        }
    }

    // Difficulty must match issued.
    if proof.difficulty_bits != rec.difficulty_bits {
        return Err(RegistryError::bad_request("difficulty mismatch"));
    }

    // Verify PoW.
    let difficulty_u8: u8 = proof
        .difficulty_bits
        .try_into()
        .map_err(|_| RegistryError::bad_request("difficulty_bits out of range"))?;

    if !crate::admission::pow::verify_pow(&nonce, difficulty_u8, &solution) {
        return Err(RegistryError::bad_request("invalid pow"));
    }

    Ok(())
}

fn validate_attestation_evidence(kind: &str, evidence: &str) -> Result<(), RegistryError> {
    let ev = evidence.trim();
    if ev.is_empty() {
        return Ok(());
    }

    // Only validate content we can reason about at the edge. Cryptographic verification is out of
    // scope for Phase 4 scaffolding.
    let kind = kind.trim().to_ascii_lowercase();

    match kind.as_str() {
        // Software evidence can either be a raw artifact hash (sha256 hex) or a JSON envelope.
        "software" | "sw" => validate_software_evidence(ev),
        // Hardware evidence is treated as an opaque blob, but we validate that it's a valid
        // encoding to avoid accidental log / storage injection.
        _ => validate_binary_evidence(ev),
    }
}

fn validate_software_evidence(evidence: &str) -> Result<(), RegistryError> {
    // Plain sha256 hex.
    if is_hex_len(evidence, 64) {
        return Ok(());
    }

    // JSON envelope: { "artifact_hash": "...", "signature_b64": "...", ... }
    if evidence.starts_with('{') {
        let v: serde_json::Value = serde_json::from_str(evidence)
            .map_err(|_| RegistryError::bad_request("invalid software evidence json"))?;
        let Some(hash) = v.get("artifact_hash").and_then(|x| x.as_str()) else {
            return Err(RegistryError::bad_request(
                "software evidence json requires artifact_hash",
            ));
        };
        if !is_hex_len(hash, 64) {
            return Err(RegistryError::bad_request(
                "software artifact_hash must be 64 hex chars",
            ));
        }

        if let Some(sig) = v.get("signature_b64").and_then(|x| x.as_str()) {
            let sig = sig.trim();
            // We accept any decoded size up to 4KB for forward-compat.
            let decoded = B64
                .decode(sig)
                .map_err(|_| RegistryError::bad_request("invalid signature_b64"))?;
            if decoded.is_empty() || decoded.len() > 4096 {
                return Err(RegistryError::bad_request("signature_b64 size invalid"));
            }
        }

        return Ok(());
    }

    Err(RegistryError::bad_request(
        "invalid software evidence (expected 64-hex hash or JSON envelope)",
    ))
}

fn validate_binary_evidence(evidence: &str) -> Result<(), RegistryError> {
    let ev = evidence.trim();

    // Allow an explicit prefix for hex.
    let maybe_hex = ev.strip_prefix("hex:").unwrap_or(ev);

    let decoded = if ev.starts_with("hex:") {
        hex::decode(maybe_hex).map_err(|_| RegistryError::bad_request("invalid hex evidence"))?
    } else {
        // Prefer base64 decoding; if it fails and the string looks like hex, fall back.
        match B64.decode(ev) {
            Ok(b) => b,
            Err(_) if looks_like_hex(ev) => {
                hex::decode(ev).map_err(|_| RegistryError::bad_request("invalid hex evidence"))?
            }
            Err(_) => return Err(RegistryError::bad_request("invalid base64 evidence")),
        }
    };

    if decoded.is_empty() {
        return Err(RegistryError::bad_request(
            "evidence must not decode to empty",
        ));
    }
    if decoded.len() > 8192 {
        return Err(RegistryError::bad_request("decoded evidence too large"));
    }
    Ok(())
}

fn looks_like_hex(s: &str) -> bool {
    let s = s.trim();
    if s.is_empty() || !s.len().is_multiple_of(2) {
        return false;
    }
    s.bytes().all(|b| b.is_ascii_hexdigit())
}

fn is_hex_len(s: &str, len: usize) -> bool {
    let s = s.trim();
    if s.len() != len {
        return false;
    }
    s.bytes().all(|b| b.is_ascii_hexdigit())
}

fn constant_time_eq(a: &str, b: &str) -> bool {
    a.as_bytes().ct_eq(b.as_bytes()).unwrap_u8() == 1
}
