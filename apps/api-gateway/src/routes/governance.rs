#![forbid(unsafe_code)]

use axum::{
    routing::{get, post},
    Extension, Json, Router,
};
use rhelma_auth::UserPrincipal;
use rhelma_core::RequestContext;
use serde_json::Value;
use std::sync::Arc;

use crate::error::{ApiResult, GatewayError};
use crate::middleware::AuthUserExtractor;
use crate::state::AppState;

/// Governance admin routes (read current policy state + ingest policy bundles).
pub fn router() -> Router {
    Router::new()
        .route("/policy/runtime", get(runtime_policy_handler))
        .route("/policy/db_current", get(db_current_policy_handler))
        .route("/policy/ingest", post(ingest_policy_bundle_handler))
}

async fn runtime_policy_handler(
    Extension(_state): Extension<Arc<AppState>>,
    auth_user: AuthUserExtractor,
) -> ApiResult<Json<Value>> {
    let principal: UserPrincipal = auth_user.0;

    let st = rhelma_core::governance::current_policy_state();
    let pol = rhelma_core::governance::current_policy();

    tracing::info!(
        target: "ops.audit",
        user_id = %principal.user_id,
        operation = "governance_policy_runtime_read",
        "Governance policy runtime read"
    );

    Ok(Json(serde_json::json!({
        "state": st.map(|s| serde_json::json!({
            "emergency_mode": s.emergency_mode,
            "safe_mode": s.safe_mode,
            "bundle_hash": s.bundle_hash,
            "bundle_class": s.bundle_class,
            "created_at": s.created_at,
            "warnings": s.warnings,
        })),
        "bundle": pol.map(|p| match serde_json::to_value(p) {
            Ok(v) => v,
            Err(e) => serde_json::json!({"error": format!("serialize: {e}")}),
        }),
    })))
}

async fn db_current_policy_handler(
    Extension(state): Extension<Arc<AppState>>,
    Extension(_ctx): Extension<RequestContext>,
    auth_user: AuthUserExtractor,
) -> ApiResult<Json<Value>> {
    let principal: UserPrincipal = auth_user.0;

    tracing::info!(
        target: "ops.audit",
        user_id = %principal.user_id,
        operation = "governance_policy_db_read",
        "Governance policy DB read"
    );

    let row = sqlx::query(
        r#"
        SELECT bundle_id, version, hash, issued_at, expires_at, payload
        FROM policy_bundles
        ORDER BY issued_at DESC
        LIMIT 1
        "#,
    )
    .fetch_optional(&state.database)
    .await
    .map_err(|e| GatewayError::internal(format!("db select policy_bundles: {e}")))?;

    let Some(row) = row else {
        return Ok(Json(serde_json::json!({"current": null})));
    };

    let bundle_id: String = row.get("bundle_id");
    let version: String = row.get("version");
    let hash: String = row.get("hash");
    let issued_at: chrono::DateTime<chrono::Utc> = row.get("issued_at");
    let expires_at: Option<chrono::DateTime<chrono::Utc>> = row.get("expires_at");
    let payload: Value = row.get("payload");

    Ok(Json(serde_json::json!({
        "current": {
            "bundle_id": bundle_id,
            "version": version,
            "hash": hash,
            "issued_at": issued_at,
            "expires_at": expires_at,
            "payload": payload,
        }
    })))
}

async fn ingest_policy_bundle_handler(
    Extension(state): Extension<Arc<AppState>>,
    Extension(ctx): Extension<RequestContext>,
    auth_user: AuthUserExtractor,
    Json(bundle_json): Json<Value>,
) -> ApiResult<Json<Value>> {
    let principal: UserPrincipal = auth_user.0;

    // Governance writes are denied in Safe Mode.
    if rhelma_core::governance::current_policy_state().is_some_and(|s| s.safe_mode)
        && !std::env::var("RHELMA_GOVERNANCE_SAFE_MODE_ADMIN_OVERRIDE")
            .ok()
            .map(|v| {
                matches!(
                    v.trim().to_ascii_lowercase().as_str(),
                    "1" | "true" | "yes" | "y" | "on"
                )
            })
            .unwrap_or(false)
    {
        return Err(GatewayError::forbidden(
            "governance safe mode active; policy ingest denied (set RHELMA_GOVERNANCE_SAFE_MODE_ADMIN_OVERRIDE=true to override)",
        ));
    }

    let bundle: rhelma_core::governance::PolicyBundleV1 =
        serde_json::from_value(bundle_json.clone())
            .map_err(|e| GatewayError::bad_request(format!("invalid policy bundle: {e}")))?;

    let keys = rhelma_core::governance::crypto::GovernanceKeySets::from_env();
    let verified = rhelma_core::governance::policy::verify_policy_bundle_v1(
        bundle.clone(),
        &keys,
        None,
        chrono::Utc::now(),
    )
    .map_err(|e| GatewayError::bad_request(format!("policy verification failed: {e}")))?;

    let quorum_signatures = serde_json::json!(verified.verified_signers);

    tracing::info!(
        target: "ops.audit",
        request_id = %ctx.request_id(),
        user_id = %principal.user_id,
        operation = "governance_policy_ingest",
        bundle_id = %verified.bundle.bundle_id,
        class = ?verified.bundle.class,
        "Governance policy bundle ingested"
    );

    sqlx::query(
        r#"
        INSERT INTO policy_bundles (bundle_id, version, hash, quorum_signatures, issuer, issued_at, expires_at, payload)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        ON CONFLICT (bundle_id) DO UPDATE
          SET version = EXCLUDED.version,
              hash = EXCLUDED.hash,
              quorum_signatures = EXCLUDED.quorum_signatures,
              issuer = EXCLUDED.issuer,
              issued_at = EXCLUDED.issued_at,
              expires_at = EXCLUDED.expires_at,
              payload = EXCLUDED.payload
        "#,
    )
    .bind(&verified.bundle.bundle_id)
    .bind(&verified.bundle.version)
    .bind(&verified.bundle_hash)
    .bind(&quorum_signatures)
    .bind(Some(principal.user_id.to_string()))
    .bind(verified.bundle.created_at)
    .bind(verified.bundle.expires_at)
    .bind(&bundle_json)
    .execute(&state.database)
    .await
    .map_err(|e| GatewayError::internal(format!("db insert policy_bundles: {e}")))?;

    Ok(Json(serde_json::json!({
        "ok": true,
        "bundle_id": verified.bundle.bundle_id,
        "bundle_hash": verified.bundle_hash,
        "verified_signers": verified.verified_signers,
        "quorum_required": verified.quorum_required,
        "council_size": verified.council_size,
    })))
}

// sqlx::Row is only needed here.
use sqlx::Row;
