#![cfg(feature = "openapi")]
#![forbid(unsafe_code)]

//! Optional OpenAPI + Swagger UI integration for `node-registry`.
//!
//! Feature-gated behind `node-registry/openapi` so default builds remain lightweight.

use axum::Router;
use serde::{Deserialize, Serialize};
use utoipa::{OpenApi, ToSchema};
use utoipa_swagger_ui::SwaggerUi;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Rhelma Node Registry",
        version = env!("CARGO_PKG_VERSION"),
        description = "Node registration, discovery and attestation endpoints for Rhelma6."
    ),
    paths(
        healthz_doc,
        readyz_doc,
        admission_challenge_doc,
        register_node_doc,
        heartbeat_doc,
        attest_doc,
        discover_doc,
        report_outcome_doc,
    ),
    components(
        schemas(
            AdmissionChallengeResponse,
            RegisterNodeRequest,
            RegisterNodeResponse,
            HeartbeatRequest,
            HeartbeatResponse,
            AttestRequest,
            AttestResponse,
            DiscoverResponse,
            ReportOutcomeRequest,
            ReportOutcomeResponse,
        )
    ),
    tags(
        (name = "health", description = "Health checks"),
        (name = "admission", description = "Admission / anti-abuse"),
        (name = "nodes", description = "Node lifecycle"),
        (name = "internal", description = "Internal/admin endpoints"),
    )
)]
pub struct ApiDoc;

#[must_use]
pub fn swagger_router() -> Router {
    Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
}

// -----------------------------------------------------------------------------
// Schemas (doc-only, minimal).
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AdmissionChallengeResponse {
    pub node_id: Option<String>,
    pub difficulty: u32,
    pub challenge_b64: String,
    pub expires_at_unix: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RegisterNodeRequest {
    pub node_id: String,
    pub public_key_hex: String,
    pub pow_solution: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RegisterNodeResponse {
    pub accepted: bool,
    pub request_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HeartbeatRequest {
    pub node_id: String,
    pub signature_hex: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HeartbeatResponse {
    pub ok: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AttestRequest {
    pub node_id: String,
    pub kind: String,
    pub evidence: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AttestResponse {
    pub accepted: bool,
    pub attested: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DiscoverResponse {
    pub nodes: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReportOutcomeRequest {
    pub node_id: String,
    pub outcome: String,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReportOutcomeResponse {
    pub ok: bool,
}

// -----------------------------------------------------------------------------
// Documentation-only stubs.
// -----------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/healthz",
    tag = "health",
    responses((status = 200, description = "Liveness probe", body = String))
)]
async fn healthz_doc() {}

#[utoipa::path(
    get,
    path = "/readyz",
    tag = "health",
    responses((status = 200, description = "Readiness probe", body = String))
)]
async fn readyz_doc() {}

#[utoipa::path(
    get,
    path = "/v1/admission/challenge",
    tag = "admission",
    params(
        ("node_id" = Option<String>, Query, description = "Optional node_id to bind the challenge")
    ),
    responses((status = 200, description = "PoW admission challenge", body = AdmissionChallengeResponse))
)]
async fn admission_challenge_doc() {}

#[utoipa::path(
    post,
    path = "/v1/nodes/register",
    tag = "nodes",
    request_body = RegisterNodeRequest,
    responses((status = 200, description = "Register node", body = RegisterNodeResponse))
)]
async fn register_node_doc() {}

#[utoipa::path(
    post,
    path = "/v1/nodes/heartbeat",
    tag = "nodes",
    request_body = HeartbeatRequest,
    responses((status = 200, description = "Node heartbeat", body = HeartbeatResponse))
)]
async fn heartbeat_doc() {}

#[utoipa::path(
    post,
    path = "/v1/nodes/attest",
    tag = "nodes",
    request_body = AttestRequest,
    responses((status = 200, description = "Submit attestation", body = AttestResponse))
)]
async fn attest_doc() {}

#[utoipa::path(
    get,
    path = "/v1/nodes/discover",
    tag = "nodes",
    responses((status = 200, description = "Discover nodes", body = DiscoverResponse))
)]
async fn discover_doc() {}

#[utoipa::path(
    post,
    path = "/v1/internal/nodes/report",
    tag = "internal",
    request_body = ReportOutcomeRequest,
    responses((status = 200, description = "Report outcome", body = ReportOutcomeResponse))
)]
async fn report_outcome_doc() {}
