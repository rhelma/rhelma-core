use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Basic search request for the `/search` endpoint.
#[derive(Debug, Clone, Deserialize)]
pub struct SearchRequest {
    /// Field `query`.
    pub query: String,
    #[serde(default = "default_limit")]
    /// Field `limit`.
    pub limit: u32,
    #[serde(default)]
    /// Field `filters`.
    pub filters: Option<Value>,
}

/// Enhanced search request for `/search/enhanced`.
///
/// This endpoint may be backed by additional query understanding and/or
/// personalization modules.
#[derive(Debug, Clone, Deserialize)]
pub struct EnhancedSearchRequest {
    /// Field `query`.
    pub query: String,
    #[serde(default = "default_limit")]
    /// Field `limit`.
    pub limit: u32,
    #[serde(default)]
    /// Field `filters`.
    pub filters: Option<Value>,
    #[serde(default)]
    /// Field `personalization`.
    pub personalization: Option<Value>,
}

fn default_limit() -> u32 {
    10
}

/// A single search hit.
#[derive(Debug, Serialize)]
pub struct SearchHit {
    /// Field `id`.
    pub id: String,
    /// Field `score`.
    pub score: f32,
    /// Field `source`.
    pub source: Value,
}

/// Standard search response payload.
#[derive(Debug, Serialize)]
pub struct SearchResponse {
    /// Field `total`.
    pub total: u64,
    /// Field `hits`.
    pub hits: Vec<SearchHit>,
}
