#![forbid(unsafe_code)]

use axum::{routing::get, Extension, Json, Router};
use rhelma_core::RequestContext;
use rhelma_db::metrics::DbOperation;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::error::GatewayError;
use crate::state::AppState;

#[derive(Debug, Serialize, sqlx::FromRow)]
struct UserRow {
    id: String,
    email: String,
    created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
struct CreateUserReq {
    email: String,
}

pub fn router() -> Router {
    Router::new().route("/", get(list_users).post(create_user))
}

async fn list_users(
    Extension(state): Extension<Arc<AppState>>,
    Extension(ctx): Extension<RequestContext>,
) -> Result<Json<Vec<UserRow>>, GatewayError> {
    let rows = state
        .base_repo
        .run_db(
            &ctx,
            DbOperation::Select,
            Some("users"),
            |pool| async move {
                let rows = sqlx::query_as::<_, UserRow>(
                    r#"
                SELECT id, email, created_at
                FROM users
                ORDER BY created_at DESC
                LIMIT 100
                "#,
                )
                .fetch_all(&pool)
                .await?;
                Ok::<_, sqlx::Error>(rows)
            },
        )
        .await
        .map_err(|e| GatewayError::internal(format!("db list users: {e}")))?;

    Ok(Json(rows))
}

async fn create_user(
    Extension(state): Extension<Arc<AppState>>,
    Extension(ctx): Extension<RequestContext>,
    Json(body): Json<CreateUserReq>,
) -> Result<Json<UserRow>, GatewayError> {
    let row = state
        .base_repo
        .run_db(
            &ctx,
            DbOperation::Insert,
            Some("users"),
            |pool| async move {
                let row = sqlx::query_as::<_, UserRow>(
                    r#"
                INSERT INTO users (email)
                VALUES ($1)
                RETURNING id, email, created_at
                "#,
                )
                .bind(body.email)
                .fetch_one(&pool)
                .await?;
                Ok::<_, sqlx::Error>(row)
            },
        )
        .await
        .map_err(|e| GatewayError::internal(format!("db create user: {e}")))?;

    Ok(Json(row))
}
