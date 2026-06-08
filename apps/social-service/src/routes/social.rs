#![forbid(unsafe_code)]

use axum::{
    extract::{Path, Query},
    response::IntoResponse,
    routing::{get, post},
    Extension, Json, Router,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

use rhelma_core::RequestContext;

use crate::error::{bad_request, internal, not_found, ApiResult};
use crate::middleware::auth_extractor::AuthUserExtractor;
use crate::models::{
    CreateCommentRequest, CreatePostRequest, FeedResponse, ReactionResponse, SetReactionRequest,
    SocialComment, SocialPost,
};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
struct FeedQuery {
    /// Page size (defaults to config.feed_default_limit).
    limit: Option<u32>,
    /// Cursor: RFC3339 timestamp (created_at). Items older than this will be returned.
    cursor: Option<String>,
}

pub fn router() -> Router {
    Router::new()
        .route("/feed/latest", get(latest_feed))
        .route("/posts", post(create_post))
        .route("/posts/{id}", get(get_post))
        .route(
            "/posts/{id}/comments",
            get(list_comments).post(create_comment),
        )
        .route("/posts/{id}/reactions/{kind}", post(set_reaction))
}

pub(super) async fn health() -> impl IntoResponse {
    Json(json!({"ok": true, "service": "social-service"}))
}
async fn latest_feed(
    Extension(state): Extension<Arc<AppState>>,
    Extension(ctx): Extension<RequestContext>,
    Query(q): Query<FeedQuery>,
) -> ApiResult<Json<FeedResponse>> {
    let tenant_id = ctx.tenant_id().map(|t| t.to_string()).ok_or_else(|| {
        bad_request(
            &ctx,
            "missing tenant_id",
            json!({"hint": "send x-rhelma-tenant-id"}),
        )
    })?;

    let limit = q
        .limit
        .unwrap_or(state.config.feed_default_limit)
        .min(state.config.feed_max_limit) as i64;

    let cursor_dt: Option<DateTime<Utc>> = match q.cursor {
        None => None,
        Some(s) => DateTime::parse_from_rfc3339(s.trim())
            .ok()
            .map(|dt| dt.with_timezone(&Utc)),
    };

    let items: Vec<SocialPost> = if let Some(c) = cursor_dt {
        sqlx::query_as::<_, SocialPost>(
            r#"
            SELECT
              p.id,
              p.author_id,
              p.kind,
              p.status,
              p.title,
              p.body,
              p.url,
              p.tags,
              p.created_at,
              p.updated_at,
              p.published_at,
              (SELECT COUNT(*) FROM social_comments c
                 WHERE c.tenant_id = p.tenant_id AND c.post_id = p.id) AS comments_count,
              (SELECT COUNT(*) FROM social_reactions r
                 WHERE r.tenant_id = p.tenant_id AND r.post_id = p.id AND r.kind = 'like') AS likes_count,
              (SELECT COUNT(*) FROM social_reactions r
                 WHERE r.tenant_id = p.tenant_id AND r.post_id = p.id AND r.kind = 'bookmark') AS bookmarks_count
            FROM social_posts p
            WHERE p.tenant_id = $1
              AND p.status = 'published'
              AND p.created_at < $2
            ORDER BY p.created_at DESC, p.id DESC
            LIMIT $3
            "#,
        )
        .bind(&tenant_id)
        .bind(c)
        .bind(limit)
        .fetch_all(&state.database)
        .await
        .map_err(|e| internal(&ctx, &format!("db: {e}")))?
    } else {
        sqlx::query_as::<_, SocialPost>(
            r#"
            SELECT
              p.id,
              p.author_id,
              p.kind,
              p.status,
              p.title,
              p.body,
              p.url,
              p.tags,
              p.created_at,
              p.updated_at,
              p.published_at,
              (SELECT COUNT(*) FROM social_comments c
                 WHERE c.tenant_id = p.tenant_id AND c.post_id = p.id) AS comments_count,
              (SELECT COUNT(*) FROM social_reactions r
                 WHERE r.tenant_id = p.tenant_id AND r.post_id = p.id AND r.kind = 'like') AS likes_count,
              (SELECT COUNT(*) FROM social_reactions r
                 WHERE r.tenant_id = p.tenant_id AND r.post_id = p.id AND r.kind = 'bookmark') AS bookmarks_count
            FROM social_posts p
            WHERE p.tenant_id = $1
              AND p.status = 'published'
            ORDER BY p.created_at DESC, p.id DESC
            LIMIT $2
            "#,
        )
        .bind(&tenant_id)
        .bind(limit)
        .fetch_all(&state.database)
        .await
        .map_err(|e| internal(&ctx, &format!("db: {e}")))?
    };

    let next_cursor = items.last().map(|p| p.created_at.to_rfc3339());

    Ok(Json(FeedResponse { items, next_cursor }))
}
async fn create_post(
    Extension(state): Extension<Arc<AppState>>,
    Extension(ctx): Extension<RequestContext>,
    AuthUserExtractor(principal): AuthUserExtractor,
    Json(payload): Json<CreatePostRequest>,
) -> ApiResult<Json<SocialPost>> {
    let tenant_id = ctx.tenant_id().map(|t| t.to_string()).ok_or_else(|| {
        bad_request(
            &ctx,
            "missing tenant_id",
            json!({"hint": "send x-rhelma-tenant-id"}),
        )
    })?;

    let kind = payload.kind.trim().to_lowercase();
    if kind != "post" && kind != "article" && kind != "link" {
        return Err(bad_request(
            &ctx,
            "invalid kind",
            json!({"allowed": ["post", "article", "link"]}),
        ));
    }

    let status = payload
        .status
        .as_deref()
        .unwrap_or("published")
        .trim()
        .to_lowercase();
    if status != "draft" && status != "published" && status != "removed" {
        return Err(bad_request(
            &ctx,
            "invalid status",
            json!({"allowed": ["draft", "published", "removed"]}),
        ));
    }

    let title = payload
        .title
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let body = payload
        .body
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let url = payload
        .url
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());

    // Minimal validation by kind
    if (kind == "post" || kind == "article") && body.is_none() {
        return Err(bad_request(
            &ctx,
            "body is required for post/article",
            json!({"field": "body"}),
        ));
    }
    if kind == "link" && url.is_none() {
        return Err(bad_request(
            &ctx,
            "url is required for link",
            json!({"field": "url"}),
        ));
    }

    let tags: Vec<String> = payload
        .tags
        .unwrap_or_default()
        .into_iter()
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .take(32)
        .collect();

    let row = sqlx::query_as::<_, SocialPost>(
        r#"
        INSERT INTO social_posts (tenant_id, author_id, kind, status, title, body, url, tags, published_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8,
                CASE WHEN $4 = 'published' THEN NOW() ELSE NULL END)
        RETURNING
          id,
          author_id,
          kind,
          status,
          title,
          body,
          url,
          tags,
          created_at,
          updated_at,
          published_at,
          0::BIGINT AS comments_count,
          0::BIGINT AS likes_count,
          0::BIGINT AS bookmarks_count
        "#,
    )
    .bind(&tenant_id)
    .bind(principal.user_id.as_uuid())
    .bind(&kind)
    .bind(&status)
    .bind(title)
    .bind(body)
    .bind(url)
    .bind(&tags)
    .fetch_one(&state.database)
    .await
    .map_err(|e| internal(&ctx, &format!("db: {e}")))?;

    Ok(Json(row))
}
async fn get_post(
    Extension(state): Extension<Arc<AppState>>,
    Extension(ctx): Extension<RequestContext>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<SocialPost>> {
    let tenant_id = ctx.tenant_id().map(|t| t.to_string()).ok_or_else(|| {
        bad_request(
            &ctx,
            "missing tenant_id",
            json!({"hint": "send x-rhelma-tenant-id"}),
        )
    })?;

    let row = sqlx::query_as::<_, SocialPost>(
        r#"
        SELECT
          p.id,
          p.author_id,
          p.kind,
          p.status,
          p.title,
          p.body,
          p.url,
          p.tags,
          p.created_at,
          p.updated_at,
          p.published_at,
          (SELECT COUNT(*) FROM social_comments c
             WHERE c.tenant_id = p.tenant_id AND c.post_id = p.id) AS comments_count,
          (SELECT COUNT(*) FROM social_reactions r
             WHERE r.tenant_id = p.tenant_id AND r.post_id = p.id AND r.kind = 'like') AS likes_count,
          (SELECT COUNT(*) FROM social_reactions r
             WHERE r.tenant_id = p.tenant_id AND r.post_id = p.id AND r.kind = 'bookmark') AS bookmarks_count
        FROM social_posts p
        WHERE p.tenant_id = $1
          AND p.id = $2
          AND p.status <> 'removed'
        "#,
    )
    .bind(&tenant_id)
    .bind(id)
    .fetch_optional(&state.database)
    .await
    .map_err(|e| internal(&ctx, &format!("db: {e}")))?;

    match row {
        Some(p) => Ok(Json(p)),
        None => Err(not_found(&ctx, "post not found")),
    }
}
async fn list_comments(
    Extension(state): Extension<Arc<AppState>>,
    Extension(ctx): Extension<RequestContext>,
    Path(post_id): Path<Uuid>,
) -> ApiResult<Json<Vec<SocialComment>>> {
    let tenant_id = ctx.tenant_id().map(|t| t.to_string()).ok_or_else(|| {
        bad_request(
            &ctx,
            "missing tenant_id",
            json!({"hint": "send x-rhelma-tenant-id"}),
        )
    })?;

    let rows = sqlx::query_as::<_, SocialComment>(
        r#"
        SELECT id, post_id, author_id, parent_id, body, created_at, updated_at
        FROM social_comments
        WHERE tenant_id = $1 AND post_id = $2
        ORDER BY created_at ASC, id ASC
        LIMIT 500
        "#,
    )
    .bind(&tenant_id)
    .bind(post_id)
    .fetch_all(&state.database)
    .await
    .map_err(|e| internal(&ctx, &format!("db: {e}")))?;

    Ok(Json(rows))
}
async fn create_comment(
    Extension(state): Extension<Arc<AppState>>,
    Extension(ctx): Extension<RequestContext>,
    AuthUserExtractor(principal): AuthUserExtractor,
    Path(post_id): Path<Uuid>,
    Json(payload): Json<CreateCommentRequest>,
) -> ApiResult<Json<SocialComment>> {
    let tenant_id = ctx.tenant_id().map(|t| t.to_string()).ok_or_else(|| {
        bad_request(
            &ctx,
            "missing tenant_id",
            json!({"hint": "send x-rhelma-tenant-id"}),
        )
    })?;

    let body = payload.body.trim();
    if body.is_empty() {
        return Err(bad_request(
            &ctx,
            "comment body must not be empty",
            json!({"field": "body"}),
        ));
    }
    if body.len() > 10_000 {
        return Err(bad_request(
            &ctx,
            "comment body is too long",
            json!({"max": 10000}),
        ));
    }

    // Ensure post exists (tenant-scoped)
    let exists: Option<(Uuid,)> = sqlx::query_as(
        r#"SELECT id FROM social_posts WHERE tenant_id = $1 AND id = $2 AND status <> 'removed'"#,
    )
    .bind(&tenant_id)
    .bind(post_id)
    .fetch_optional(&state.database)
    .await
    .map_err(|e| internal(&ctx, &format!("db: {e}")))?;

    if exists.is_none() {
        return Err(not_found(&ctx, "post not found"));
    }

    let row = sqlx::query_as::<_, SocialComment>(
        r#"
        INSERT INTO social_comments (tenant_id, post_id, author_id, parent_id, body)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, post_id, author_id, parent_id, body, created_at, updated_at
        "#,
    )
    .bind(&tenant_id)
    .bind(post_id)
    .bind(principal.user_id.as_uuid())
    .bind(payload.parent_id)
    .bind(body)
    .fetch_one(&state.database)
    .await
    .map_err(|e| internal(&ctx, &format!("db: {e}")))?;

    Ok(Json(row))
}
async fn set_reaction(
    Extension(state): Extension<Arc<AppState>>,
    Extension(ctx): Extension<RequestContext>,
    AuthUserExtractor(principal): AuthUserExtractor,
    Path((post_id, kind)): Path<(Uuid, String)>,
    Json(payload): Json<SetReactionRequest>,
) -> ApiResult<Json<ReactionResponse>> {
    let tenant_id = ctx.tenant_id().map(|t| t.to_string()).ok_or_else(|| {
        bad_request(
            &ctx,
            "missing tenant_id",
            json!({"hint": "send x-rhelma-tenant-id"}),
        )
    })?;

    let kind = kind.trim().to_lowercase();
    if kind != "like" && kind != "bookmark" {
        return Err(bad_request(
            &ctx,
            "invalid reaction kind",
            json!({"allowed": ["like", "bookmark"]}),
        ));
    }

    // Decide target state
    let desired = if let Some(a) = payload.active {
        a
    } else {
        // Toggle
        let exists: Option<(i64,)> = sqlx::query_as(
            r#"SELECT 1 FROM social_reactions WHERE tenant_id=$1 AND post_id=$2 AND user_id=$3 AND kind=$4"#,
        )
        .bind(&tenant_id)
        .bind(post_id)
        .bind(principal.user_id.as_uuid())
        .bind(&kind)
        .fetch_optional(&state.database)
        .await
        .map_err(|e| internal(&ctx, &format!("db: {e}")))?;
        exists.is_none()
    };

    if desired {
        sqlx::query(
            r#"INSERT INTO social_reactions (tenant_id, post_id, user_id, kind)
               VALUES ($1,$2,$3,$4)
               ON CONFLICT DO NOTHING"#,
        )
        .bind(&tenant_id)
        .bind(post_id)
        .bind(principal.user_id.as_uuid())
        .bind(&kind)
        .execute(&state.database)
        .await
        .map_err(|e| internal(&ctx, &format!("db: {e}")))?;
    } else {
        sqlx::query(
            r#"DELETE FROM social_reactions WHERE tenant_id=$1 AND post_id=$2 AND user_id=$3 AND kind=$4"#,
        )
        .bind(&tenant_id)
        .bind(post_id)
        .bind(principal.user_id.as_uuid())
        .bind(&kind)
        .execute(&state.database)
        .await
        .map_err(|e| internal(&ctx, &format!("db: {e}")))?;
    }

    Ok(Json(ReactionResponse {
        kind,
        active: desired,
    }))
}
