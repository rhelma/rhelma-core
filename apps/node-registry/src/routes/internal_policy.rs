use axum::{
    extract::State,
    http::StatusCode,
    Json,
};

use serde::{Deserialize, Serialize};

use crate::state::AppState;

/// Token-gated internal endpoints for jury-approved policy actions.
///
/// NOTE:
/// - This file is intended to be *merged* into your existing node-registry.
/// - Wire these routes under `/v1/internal/nodes/*`.
/// - Enforce `x-registry-admin-token` using your existing middleware or a simple header check.

#[derive(Debug, Deserialize)]
pub struct QuarantineReq {
    /// Field `node_id`.
    pub node_id: String,
    /// Field `incident_id`.
    pub incident_id: String,
    /// Field `reason`.
    pub reason: String,
    /// Field `ttl_seconds`.
    pub ttl_seconds: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct UnquarantineReq {
    /// Field `node_id`.
    pub node_id: String,
    /// Field `incident_id`.
    pub incident_id: String,
    /// Field `reason`.
    pub reason: String,
}

#[derive(Debug, Serialize)]
pub struct PolicyActionResp {
    /// Field `ok`.
    pub ok: bool,
    /// Field `node_id`.
    pub node_id: String,
    /// Field `status`.
    pub status: String,
}

pub async fn quarantine_node(
    State(state): State<AppState>,
    Json(req): Json<QuarantineReq>,
) -> Result<Json<PolicyActionResp>, (StatusCode, String)> {
    state.policy.quarantine(req).await.map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok(Json(PolicyActionResp { ok: true, node_id: state.policy.last_node_id(), status: "quarantined".into() }))
}

pub async fn unquarantine_node(
    State(state): State<AppState>,
    Json(req): Json<UnquarantineReq>,
) -> Result<Json<PolicyActionResp>, (StatusCode, String)> {
    state.policy.unquarantine(req).await.map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok(Json(PolicyActionResp { ok: true, node_id: state.policy.last_node_id(), status: "active".into() }))
}
