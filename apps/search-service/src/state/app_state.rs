use crate::analytics::AnalyticsSink;
use crate::config::SearchConfig;
use crate::engines::{
    embedding::EmbeddingEngine, hybrid::HybridSearchEngine, meilisearch::MeiliSearchEngine,
    qdrant::QdrantSearchEngine,
};
use std::sync::Arc;
use thiserror::Error;

/// Shared application state for all routes and handlers.
///
/// This aggregates all backends (vector DB, full-text, embeddings, analytics).
#[derive(Clone)]
pub struct AppState {
    /// Field `config`.
    pub config: SearchConfig,
    /// Field `embedding`.
    pub embedding: Arc<EmbeddingEngine>,
    /// Field `qdrant`.
    pub qdrant: Arc<QdrantSearchEngine>,
    /// Field `meili`.
    pub meili: Arc<MeiliSearchEngine>,
    /// Field `hybrid`.
    pub hybrid: Arc<HybridSearchEngine>,
    /// Field `analytics`.
    pub analytics: Arc<AnalyticsSink>,
}

#[derive(Debug, Error)]
pub enum InitError {
    #[error("embedding engine init failed: {0}")]
    /// Variant `Embedding`.
    Embedding(String),

    #[error("qdrant init failed: {0}")]
    /// Variant `Qdrant`.
    Qdrant(String),

    #[error("meilisearch init failed: {0}")]
    /// Variant `Meili`.
    Meili(String),

    #[error("analytics init failed: {0}")]
    /// Variant `Analytics`.
    Analytics(String),
}

impl AppState {
    pub async fn initialize(config: SearchConfig) -> Result<Self, InitError> {
        let embedding = EmbeddingEngine::new(config.embedding_model.clone())
            .await
            .map_err(|e| InitError::Embedding(e.to_string()))?;

        let qdrant = QdrantSearchEngine::new(&config.qdrant_url, &config.default_index)
            .await
            .map_err(|e| InitError::Qdrant(e.to_string()))?;

        let meili = MeiliSearchEngine::new(&config.meili_url, &config.default_index)
            .await
            .map_err(|e| InitError::Meili(e.to_string()))?;

        let analytics = AnalyticsSink::new(
            config.service_name.clone(),
            env!("CARGO_PKG_VERSION").to_string(),
            config.region.clone(),
            crate::analytics::noop_bus(),
        );

        let hybrid =
            HybridSearchEngine::new(Arc::new(embedding), Arc::new(qdrant), Arc::new(meili));

        Ok(Self {
            config,
            embedding: hybrid.embedding.clone(),
            qdrant: hybrid.qdrant.clone(),
            meili: hybrid.meili.clone(),
            hybrid: Arc::new(hybrid),
            analytics: Arc::new(analytics),
        })
    }
}
