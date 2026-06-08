#![forbid(unsafe_code)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------
// Requests
// ---------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct CreatePostRequest {
    /// One of: post | article | link
    pub kind: String,

    /// One of: draft | published | removed (optional; default = published)
    pub status: Option<String>,

    /// Optional title (recommended for article/link)
    pub title: Option<String>,

    /// Optional body (required for post/article)
    pub body: Option<String>,

    /// Optional URL (required for link)
    pub url: Option<String>,

    /// Optional tags
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateCommentRequest {
    pub body: String,
    pub parent_id: Option<Uuid>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SetReactionRequest {
    /// If omitted, the service will toggle the reaction.
    pub active: Option<bool>,
}

// ---------------------------------------------------------------------
// Responses / DB rows
// ---------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct SocialPost {
    pub id: Uuid,
    pub author_id: Uuid,
    pub kind: String,
    pub status: String,
    pub title: Option<String>,
    pub body: Option<String>,
    pub url: Option<String>,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub published_at: Option<DateTime<Utc>>,

    // Aggregates (computed via subqueries)
    pub comments_count: i64,
    pub likes_count: i64,
    pub bookmarks_count: i64,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct SocialComment {
    pub id: Uuid,
    pub post_id: Uuid,
    pub author_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub body: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FeedResponse {
    pub items: Vec<SocialPost>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReactionResponse {
    pub kind: String,
    pub active: bool,
}
