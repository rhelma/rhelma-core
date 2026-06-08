use meilisearch_sdk::{client::Client, indexes::Index};
use metrics::{counter, histogram};
use serde_json::Value;
use std::time::Instant;
use thiserror::Error;
use tracing::debug;

/// Thin wrapper around Meilisearch for full-text search.
pub struct MeiliSearchEngine {
    client: Client,
    index: Index,
}

#[derive(Debug, Error)]
pub enum MeiliError {
    #[error("meilisearch error: {0}")]
    /// Variant `Client`.
    Client(String),
}

impl MeiliSearchEngine {
    pub async fn new(endpoint: &str, index_name: &str) -> Result<Self, MeiliError> {
        // No api key for simplicity; wire via env if needed.
        let client = Client::new(endpoint, None::<String>);

        // `meilisearch-sdk` v0.25 does not expose a `get_or_create` helper.
        // Construct an index handle; the actual existence is validated on request.
        let index = client.index(index_name);

        Ok(Self { client, index })
    }

    pub async fn text_search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<(String, f32, Value)>, MeiliError> {
        let start = Instant::now();

        let result = self
            .index
            .search()
            .with_query(query)
            .with_limit(limit)
            .execute::<Value>()
            .await
            .map_err(|e| MeiliError::Client(e.to_string()))?;

        let elapsed = start.elapsed();
        histogram!("search_meili_duration_seconds",).record(elapsed.as_secs_f64());
        counter!("search_meili_requests_total").increment(1);

        let mut out = Vec::with_capacity(result.hits.len());
        for hit in result.hits {
            let id = hit
                .result
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let score = hit.ranking_score.unwrap_or(0.0) as f32;
            out.push((id, score, hit.result));
        }

        debug!(hits = out.len(), "meilisearch search completed");
        Ok(out)
    }
}
