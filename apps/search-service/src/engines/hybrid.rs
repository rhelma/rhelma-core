#![forbid(unsafe_code)]

use crate::engines::embedding::EmbeddingEngine;
use crate::engines::{meilisearch::MeiliSearchEngine, qdrant::QdrantSearchEngine};
use crate::models::query::SearchHit;
use metrics::histogram;
use serde_json::Value;
use std::sync::Arc;
use std::time::Instant;

/// Hybrid search engine combining semantic (Qdrant) and lexical (Meilisearch).
///
/// Rhelma guidance:
/// - Prefer deterministic, score-calibration-free fusion (RRF) over raw-score max/merge.
/// - Keep weights to allow tuning without changing call sites.
pub struct HybridSearchEngine {
    /// Field `embedding`.
    pub embedding: Arc<EmbeddingEngine>,
    /// Field `qdrant`.
    pub qdrant: Arc<QdrantSearchEngine>,
    /// Field `meili`.
    pub meili: Arc<MeiliSearchEngine>,
    /// Weight applied to semantic rank contribution.
    pub semantic_weight: f32,
    /// Weight applied to lexical rank contribution.
    pub lexical_weight: f32,
    /// RRF constant (larger = flatter).
    pub rrf_k: f32,
}

impl HybridSearchEngine {
    pub fn new(
        embedding: Arc<EmbeddingEngine>,
        qdrant: Arc<QdrantSearchEngine>,
        meili: Arc<MeiliSearchEngine>,
    ) -> Self {
        Self {
            embedding,
            qdrant,
            meili,
            semantic_weight: 0.6,
            lexical_weight: 0.4,
            rrf_k: 60.0,
        }
    }

    pub async fn search(&self, query: &str, limit: u32) -> anyhow::Result<Vec<SearchHit>> {
        let start = Instant::now();

        let embeddings = self
            .embedding
            .embed(&[query.to_string()])
            .await
            .map_err(|e| anyhow::anyhow!(e))?;
        let vec = embeddings.into_iter().next().unwrap_or_default();

        let semantic = self
            .qdrant
            .vector_search(vec, limit as u64)
            .await
            .map_err(|e| anyhow::anyhow!(e))?;

        let lexical = self
            .meili
            .text_search(query, limit as usize)
            .await
            .map_err(|e| anyhow::anyhow!(e))?;

        let elapsed = start.elapsed();
        histogram!("search_hybrid_duration_seconds").record(elapsed.as_secs_f64());

        Ok(fuse_results_rrf(
            semantic,
            lexical,
            limit as usize,
            self.rrf_k,
            self.semantic_weight,
            self.lexical_weight,
        ))
    }
}

/// Reciprocal Rank Fusion (RRF) over semantic + lexical result lists.
///
/// - Robust to different score scales.
/// - Deterministic.
/// - Easy to tune via k and weights.
fn fuse_results_rrf(
    semantic: Vec<(String, f32, Value)>,
    lexical: Vec<(String, f32, Value)>,
    limit: usize,
    k: f32,
    semantic_w: f32,
    lexical_w: f32,
) -> Vec<SearchHit> {
    use std::collections::HashMap;

    let mut score_map: HashMap<String, f32> = HashMap::new();
    let mut payload_map: HashMap<String, Value> = HashMap::new();

    for (rank, (id, _score, payload)) in semantic.into_iter().enumerate() {
        let rr = 1.0 / (k + rank as f32);
        *score_map.entry(id.clone()).or_insert(0.0) += semantic_w * rr;
        // Prefer semantic payload when present.
        payload_map.entry(id).or_insert(payload);
    }

    for (rank, (id, _score, payload)) in lexical.into_iter().enumerate() {
        let rr = 1.0 / (k + rank as f32);
        *score_map.entry(id.clone()).or_insert(0.0) += lexical_w * rr;
        // Only set payload if we don't already have one.
        payload_map.entry(id).or_insert(payload);
    }

    let mut out: Vec<SearchHit> = score_map
        .into_iter()
        .map(|(id, score)| SearchHit {
            id: id.clone(),
            score,
            source: payload_map.remove(&id).unwrap_or(Value::Null),
        })
        .collect();

    out.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    out.truncate(limit);
    out
}
