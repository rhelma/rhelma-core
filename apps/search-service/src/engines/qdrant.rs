#![forbid(unsafe_code)]

use metrics::{counter, histogram};
use qdrant_client::qdrant::{point_id, SearchPoints};
use qdrant_client::Qdrant;
use serde_json::Value;
use std::time::Instant;
use thiserror::Error;
use tracing::debug;

/// Thin wrapper around Qdrant client for vector search.
///
/// Notes (Rhelma v5.2 alignment):
/// - Uses the non-deprecated `qdrant_client::Qdrant` API.
/// - Metrics labels are owned (`String`) to satisfy `metrics` macros' `'static` requirement.
pub struct QdrantSearchEngine {
    client: Qdrant,
    collection: String,
}

#[derive(Debug, Error)]
pub enum QdrantError {
    #[error("qdrant client error: {0}")]
    /// Variant `Client`.
    Client(String),
}

impl QdrantSearchEngine {
    pub async fn new(endpoint: &str, collection: &str) -> Result<Self, QdrantError> {
        let client = Qdrant::from_url(endpoint)
            .build()
            .map_err(|e| QdrantError::Client(e.to_string()))?;

        Ok(Self {
            client,
            collection: collection.to_string(),
        })
    }

    pub async fn vector_search(
        &self,
        query_vector: Vec<f32>,
        limit: u64,
    ) -> Result<Vec<(String, f32, Value)>, QdrantError> {
        let start = Instant::now();

        // Build request explicitly to stay compatible with qdrant-client 1.x
        // and avoid relying on deprecated prelude re-exports.
        let req = SearchPoints {
            collection_name: self.collection.clone(),
            vector: query_vector,
            limit,
            with_payload: Some(true.into()),
            with_vectors: None,
            filter: None,
            params: None,
            score_threshold: None,
            offset: None,
            vector_name: None,
            read_consistency: None,
            // Newer qdrant-client versions require these fields:
            shard_key_selector: None,
            sparse_indices: None,
            timeout: None,
        };

        let search_result = self
            .client
            .search_points(req)
            .await
            .map_err(|e| QdrantError::Client(e.to_string()))?;

        let elapsed = start.elapsed();
        let collection_label = self.collection.clone();
        histogram!(
            "search_qdrant_duration_seconds",
            "collection" => collection_label.clone()
        )
        .record(elapsed.as_secs_f64());

        counter!(
            "search_qdrant_requests_total",
            "collection" => collection_label
        )
        .increment(1);

        let mut out = Vec::with_capacity(search_result.result.len());

        for point in search_result.result {
            // Point id is optional; keep a stable fallback.
            let id = point
                .id
                .and_then(|pid| pid.point_id_options)
                .map(|opt| match opt {
                    point_id::PointIdOptions::Uuid(u) => u,
                    point_id::PointIdOptions::Num(n) => n.to_string(),
                })
                .unwrap_or_else(|| "unknown".to_string());

            let score = point.score;

            // Payload in qdrant is a protobuf map; serialize it into JSON.
            let payload = serde_json::to_value(&point.payload).unwrap_or(Value::Null);

            out.push((id, score, payload));
        }

        debug!(hits = out.len(), "qdrant search completed");
        Ok(out)
    }
}
