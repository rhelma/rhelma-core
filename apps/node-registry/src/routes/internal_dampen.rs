#![forbid(unsafe_code)]

#![cfg(feature = "internal-dampen")]

//! Token-gated internal API: dampen/undampen routing weight.
//!
//! This module is a **scaffold** and is **not wired** into the node-registry router by default.
//! It is feature-gated to prevent accidental exposure.
//!
//! Enable with `--features internal-dampen` and wire routes manually.

use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};

use crate::state::SharedState;

#[derive(Debug, Deserialize)]
pub struct DampenRequest {
    /// Field `node_id`.
    pub node_id: String,
    /// Field `weight`.
    pub weight: f64, // 0.0..=1.0
    /// Field `ttl_seconds`.
    pub ttl_seconds: u64,
    /// Field `reason`.
    pub reason: String,
    /// Field `evidence_ref`.
    pub evidence_ref: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DampenResponse {
    /// Field `node_id`.
    pub node_id: String,
    /// Field `routing_weight`.
    pub routing_weight: f64,
    /// Field `dampened_until_unix`.
    pub dampened_until_unix: i64,
}

pub async fn dampen_node(
    State(_state): State<SharedState>,
    Json(req): Json<DampenRequest>,
) -> Result<(StatusCode, Json<DampenResponse>), (StatusCode, String)> {
    // NOTE: NodeRegistry's current contract does not expose routing weights yet.
    // This endpoint remains a placeholder until store/models are extended.
    let now = chrono::Utc::now().timestamp();
    let until = now.saturating_add(req.ttl_seconds as i64);
    Ok((
        StatusCode::NOT_IMPLEMENTED,
        Json(DampenResponse {
            node_id: req.node_id,
            routing_weight: req.weight,
            dampened_until_unix: until,
        }),
    ))
}

#[derive(Debug, Deserialize)]
pub struct UndampenRequest {
    /// Field `node_id`.
    pub node_id: String,
    /// Field `reason`.
    pub reason: String,
    /// Field `evidence_ref`.
    pub evidence_ref: Option<String>,
}

pub async fn undampen_node(
    State(_state): State<SharedState>,
    Json(_req): Json<UndampenRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    Ok(StatusCode::NOT_IMPLEMENTED)
}
