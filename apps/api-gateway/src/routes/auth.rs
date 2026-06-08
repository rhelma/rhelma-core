#![forbid(unsafe_code)]

use axum::{
    extract::Extension,
    routing::{get, post},
    Json, Router,
};
use rhelma_auth::crypto::password::{hash_password, validate_password_policy, verify_password};
use rhelma_auth::types::{Permission, Role, SessionId, UserPrincipal};
use rhelma_core::{prelude::UserId, RequestContext};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, sqlx::FromRow)]
struct LoginRow {
    user_id: Uuid,
    roles: Option<Value>,
    password_hash: Option<String>,
    active: Option<bool>,
}

#[derive(Debug, sqlx::FromRow)]
struct CountRow {
    count: Option<i64>,
}

use crate::error::{ApiResult, GatewayError};
use crate::middleware::auth_extractor::AuthPrincipal;
use crate::state::AppState;

pub fn router() -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/refresh", post(refresh))
        .route("/logout", post(logout))
}

async fn health() -> &'static str {
    "ok"
}

/// Login request payload.
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Debug, Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 8, max = 256))]
    pub password: String,
}

/// Register request payload.
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Debug, Deserialize, Validate)]
pub struct RegisterRequest {
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 8, max = 256))]
    pub password: String,
    #[validate(length(min = 1, max = 200))]
    pub name: Option<String>,
}

/// Refresh token request payload.
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Debug, Deserialize, Validate)]
pub struct RefreshRequest {
    #[validate(length(min = 32, max = 512))]
    pub refresh_token: String,
}

/// Auth response payload (access + refresh tokens).
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub access_exp: i64,
}

async fn login(
    Extension(ctx): Extension<RequestContext>,
    Extension(state): Extension<std::sync::Arc<AppState>>,
    Json(payload): Json<LoginRequest>,
) -> ApiResult<Json<AuthResponse>> {
    payload
        .validate()
        .map_err(|e| GatewayError::bad_request(format!("validation: {e}")))?;

    let tenant_id = ctx
        .tenant_id()
        .ok_or_else(|| GatewayError::bad_request("missing tenant_id"))?
        .clone();
    let tenant_id_db = tenant_id.clone();
    let email = payload.email.trim().to_ascii_lowercase();

    // Load user + credential.
    let row = state
        .base_repo
        .run_db(
            &ctx,
            rhelma_db::metrics::DbOperation::Select,
            Some("users"),
            move |db| async move {
                sqlx::query_as::<_, LoginRow>(
                    r#"
                SELECT
                    u.id as user_id,
                    u.roles as roles,
                    c.password_hash as password_hash,
                    c.active as active
                FROM users u
                INNER JOIN user_credentials c
                    ON c.user_id = u.id AND c.tenant_id = u.tenant_id
                WHERE u.tenant_id = $1 AND u.email = $2
                "#,
                )
                .bind(tenant_id_db.as_str())
                .bind(email)
                .fetch_optional(&db)
                .await
            },
        )
        .await?;

    let Some(row) = row else {
        return Err(GatewayError::unauthorized("invalid credentials"));
    };
    if row.active == Some(false) {
        return Err(GatewayError::forbidden("account disabled"));
    }

    let password_hash = row
        .password_hash
        .ok_or_else(|| GatewayError::internal("missing password hash"))?;
    let ok = verify_password(&payload.password, &password_hash)
        .map_err(|_| GatewayError::unauthorized("invalid credentials"))?;
    if !ok {
        return Err(GatewayError::unauthorized("invalid credentials"));
    }

    let roles = parse_roles(row.roles);

    let principal = UserPrincipal {
        user_id: UserId(row.user_id),
        tenant_id: Some(tenant_id),
        session_id: SessionId::new(),
        roles,
        permissions: Vec::<Permission>::new(),
    };

    let pair = state
        .auth_service
        .issue_for_principal(&principal)
        .await
        .map_err(|_| GatewayError::internal("auth token issuance failed"))?;

    Ok(Json(AuthResponse {
        access_token: pair.access_token,
        refresh_token: pair.refresh_token,
        access_exp: pair.access_exp,
    }))
}

async fn register(
    Extension(ctx): Extension<RequestContext>,
    Extension(state): Extension<std::sync::Arc<AppState>>,
    Json(payload): Json<RegisterRequest>,
) -> ApiResult<Json<AuthResponse>> {
    payload
        .validate()
        .map_err(|e| GatewayError::bad_request(format!("validation: {e}")))?;

    validate_password_policy(&payload.password)
        .map_err(|_| GatewayError::bad_request("password policy violation"))?;

    let tenant_id = ctx
        .tenant_id()
        .ok_or_else(|| GatewayError::bad_request("missing tenant_id"))?
        .clone();
    let email = payload.email.trim().to_ascii_lowercase();
    let name = payload
        .name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| email.clone());

    // Ensure email not already used in this tenant.
    let exists = state
        .base_repo
        .run_db(&ctx, rhelma_db::metrics::DbOperation::Select, Some("users"), {
            let tenant = tenant_id.as_str().to_string();
            let email = email.clone();
            move |db| async move {
                sqlx::query_as::<_, CountRow>(r#"SELECT COUNT(*)::bigint as count FROM users WHERE tenant_id = $1 AND email = $2"#)
                    .bind(tenant.as_str())
                    .bind(email)
                    .fetch_one(&db)
                    .await
            }
        })
        .await?;
    if exists.count.unwrap_or(0) > 0 {
        return Err(GatewayError::conflict("email already registered"));
    }

    let user_uuid = Uuid::new_v4();
    let password_hash = hash_password(&payload.password)
        .map_err(|_| GatewayError::internal("password hashing failed"))?;

    // Create user + credential in a single transaction.
    state
        .base_repo
        .run_db(
            &ctx,
            rhelma_db::metrics::DbOperation::Insert,
            Some("users"),
            {
                let tenant = tenant_id.as_str().to_string();
                let email = email.clone();
                let name = name.clone();
                let password_hash = password_hash.clone();
                move |db| async move {
                    let mut tx = db.begin().await?;

                    sqlx::query(
                        r#"
                    INSERT INTO users (tenant_id, id, email, name, roles)
                    VALUES ($1, $2, $3, $4, '[]'::jsonb)
                    "#,
                    )
                    .bind(tenant.as_str())
                    .bind(user_uuid)
                    .bind(email)
                    .bind(name)
                    .execute(&mut *tx)
                    .await?;

                    sqlx::query(
                        r#"
                    INSERT INTO user_credentials (tenant_id, user_id, password_hash, active)
                    VALUES ($1, $2, $3, true)
                    "#,
                    )
                    .bind(tenant.as_str())
                    .bind(user_uuid)
                    .bind(password_hash)
                    .execute(&mut *tx)
                    .await?;

                    tx.commit().await?;
                    Ok(())
                }
            },
        )
        .await?;

    let principal = UserPrincipal {
        user_id: UserId(user_uuid),
        tenant_id: Some(tenant_id),
        session_id: SessionId::new(),
        roles: vec![Role("user".to_string())],
        permissions: Vec::<Permission>::new(),
    };

    let pair = state
        .auth_service
        .issue_for_principal(&principal)
        .await
        .map_err(|_| GatewayError::internal("auth token issuance failed"))?;

    Ok(Json(AuthResponse {
        access_token: pair.access_token,
        refresh_token: pair.refresh_token,
        access_exp: pair.access_exp,
    }))
}

async fn refresh(
    Extension(ctx): Extension<RequestContext>,
    Extension(state): Extension<std::sync::Arc<AppState>>,
    Json(payload): Json<RefreshRequest>,
) -> ApiResult<Json<AuthResponse>> {
    payload
        .validate()
        .map_err(|e| GatewayError::bad_request(format!("validation: {e}")))?;

    let pair = state
        .auth_service
        .refresh(&payload.refresh_token)
        .await
        .map_err(|_| GatewayError::unauthorized("invalid refresh token"))?;

    // best-effort: keep ctx used for logging/metrics.
    let _ = ctx;

    Ok(Json(AuthResponse {
        access_token: pair.access_token,
        refresh_token: pair.refresh_token,
        access_exp: pair.access_exp,
    }))
}

async fn logout(
    Extension(state): Extension<std::sync::Arc<AppState>>,
    AuthPrincipal(principal): AuthPrincipal,
) -> ApiResult<()> {
    state
        .auth_service
        .revoke_session(principal.session_id)
        .await
        .map_err(|_| GatewayError::internal("failed to revoke session"))?;
    Ok(())
}

fn parse_roles(v: Option<Value>) -> Vec<Role> {
    let items = match v {
        Some(Value::Array(a)) => a,
        Some(Value::String(s)) => vec![Value::String(s)],
        _ => return Vec::new(),
    };

    items
        .into_iter()
        .filter_map(|x| x.as_str().map(|s| s.trim().to_string()))
        .filter(|s| !s.is_empty())
        .map(Role)
        .collect()
}
