use crate::{
    config::SearchConfig,
    engines::{EnhancedHybridEngine, QueryUnderstandingEngine, RerankingEngine},
    analytics::SearchAnalytics,
    features::SearchFeatures,
};
use rhelma_core::prelude::*;
use rhelma_db::prelude::*;
use rhelma_tracing::prelude::*;
use std::sync::Arc;

/// Enhanced application state برای search service پیشرفته
#[derive(Clone)]
pub struct AppState {
    /// Field `config`.
    pub config: Arc<SearchConfig>,
    /// Field `database`.
    pub database: Database,
    /// Field `qdrant`.
    pub qdrant: crate::engines::QdrantEngine,
    /// Field `meilisearch`.
    pub meilisearch: crate::engines::MeilisearchEngine,
    /// Field `embeddings`.
    pub embeddings: crate::engines::EmbeddingEngine,
    /// Field `hybrid`.
    pub hybrid: crate::engines::HybridEngine,
    
    // ✅ موتورهای جدید
    /// Field `enhanced_hybrid`.
    pub enhanced_hybrid: EnhancedHybridEngine,
    /// Field `query_understanding`.
    pub query_understanding: QueryUnderstandingEngine,
    /// Field `reranker`.
    pub reranker: RerankingEngine,
    /// Field `analytics`.
    pub analytics: SearchAnalytics,
    /// Field `features`.
    pub features: SearchFeatures,
    
    /// Field `http_client`.
    pub http_client: reqwest::Client,
}

impl AppState {
    pub async fn new(config: SearchConfig) -> Result<Self> {
        let config = Arc::new(config);
        
        // Initialize existing engines
        let database = Database::new(&config.database_url.expose_secret())
            .await
            .context("Failed to initialize database")?;
        
        database.run_migrations()
            .await
            .context("Failed to run database migrations")?;

        let qdrant = crate::engines::QdrantEngine::new(config.clone())
            .await
            .context("Failed to initialize Qdrant")?;
            
        let meilisearch = crate::engines::MeilisearchEngine::new(config.clone())
            .await
            .context("Failed to initialize Meilisearch")?;
            
        let embeddings = crate::engines::EmbeddingEngine::new(config.clone())
            .await
            .context("Failed to initialize embedding engine")?;
            
        let hybrid = crate::engines::HybridEngine::new(config.clone())
            .await
            .context("Failed to initialize hybrid engine")?;

        // ✅ Initialize new enhanced engines
        let enhanced_hybrid = EnhancedHybridEngine::new(config.clone())
            .await
            .context("Failed to initialize enhanced hybrid engine")?;
            
        let query_understanding = QueryUnderstandingEngine::new(config.clone())
            .await
            .context("Failed to initialize query understanding engine")?;
            
        let reranker = RerankingEngine::new(config.clone())
            .await
            .context("Failed to initialize reranking engine")?;
            
        let analytics = SearchAnalytics::new(config.clone())
            .await
            .context("Failed to initialize search analytics")?;
            
        let features = SearchFeatures::new(config.clone())
            .await
            .context("Failed to initialize search features")?;

        // Initialize HTTP client
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .context("Failed to create HTTP client")?;

        info!("✅ Enhanced search service state initialized successfully");

        Ok(Self {
            config,
            database,
            qdrant,
            meilisearch,
            embeddings,
            hybrid,
            enhanced_hybrid,
            query_understanding,
            reranker,
            analytics,
            features,
            http_client,
        })
    }

    /// Health check برای همه components
    pub async fn health_check(&self) -> bool {
        let db_healthy = self.database.health_check().await.unwrap_or(false);
        let qdrant_healthy = self.qdrant.health_check().await.unwrap_or(false);
        let meilisearch_healthy = self.meilisearch.health_check().await.unwrap_or(false);
        let embeddings_healthy = self.embeddings.health_check().await.unwrap_or(false);
        
        db_healthy && qdrant_healthy && meilisearch_healthy && embeddings_healthy
    }
}



