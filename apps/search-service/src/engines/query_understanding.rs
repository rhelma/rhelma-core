use crate::{config::SearchConfig, error::SearchError};
use rhelma_core::prelude::*;
use rhelma_tracing::prelude::*;
use std::sync::Arc;
use whatlang::detect;

/// Query understanding engine برای تحلیل queryهای کاربر
#[derive(Clone)]
pub struct QueryUnderstandingEngine {
    config: Arc<SearchConfig>,
}

impl QueryUnderstandingEngine {
    pub async fn new(config: Arc<SearchConfig>) -> Result<Self, SearchError> {
        Ok(Self { config })
    }

    /// Analyze query برای extract کردن intent و features
    #[instrument(skip(self))]
    pub async fn analyze_query(&self, query: &str) -> Result<QueryAnalysis> {
        let mut analysis = QueryAnalysis {
            original_query: query.to_string(),
            ..Default::default()
        };

        // Language detection
        analysis.language = self.detect_language(query);
        
        // Intent recognition
        analysis.intent = self.detect_intent(query).await?;
        
        // Query complexity
        analysis.complexity = self.assess_complexity(query);
        
        // Entity extraction (basic)
        analysis.entities = self.extract_entities_basic(query).await?;

        debug!(
            query = query,
            intent = ?analysis.intent,
            language = ?analysis.language,
            complexity = ?analysis.complexity,
            "Query analysis completed"
        );

        Ok(analysis)
    }

    /// Detect language of query
    fn detect_language(&self, query: &str) -> Option<String> {
        detect(query).map(|info| info.lang().to_string())
    }

    /// Detect user intent از query
    async fn detect_intent(&self, query: &str) -> Result<SearchIntent> {
        let query_lower = query.to_lowercase();
        
        // Rule-based intent detection - در آینده با ML تکمیل می‌شه
        if query_lower.contains("how to") || 
           query_lower.contains("tutorial") || 
           query_lower.contains("guide") {
            Ok(SearchIntent::Tutorial)
        } else if query_lower.contains("best") || 
                  query_lower.contains("vs") || 
                  query_lower.contains("comparison") {
            Ok(SearchIntent::Comparison)
        } else if query_lower.contains("latest") || 
                  query_lower.contains("news") || 
                  query_lower.contains("recent") {
            Ok(SearchIntent::News)
        } else if query_lower.contains("price") || 
                  query_lower.contains("buy") || 
                  query_lower.contains("cost") {
            Ok(SearchIntent::Commercial)
        } else if self.is_question_query(query) {
            Ok(SearchIntent::Factual)
        } else {
            Ok(SearchIntent::General)
        }
    }

    /// Check if query is a question
    fn is_question_query(&self, query: &str) -> bool {
        let question_words = ["what", "why", "how", "when", "where", "which", "who"];
        let query_lower = query.to_lowercase();
        
        question_words.iter().any(|&word| query_lower.starts_with(word))
    }

    /// Assess query complexity
    fn assess_complexity(&self, query: &str) -> QueryComplexity {
        let word_count = query.split_whitespace().count();
        let char_count = query.chars().count();
        
        match word_count {
            0..=2 => QueryComplexity::Simple,
            3..=5 => QueryComplexity::Medium,
            _ => QueryComplexity::Complex,
        }
    }

    /// Basic entity extraction
    async fn extract_entities_basic(&self, query: &str) -> Result<Vec<QueryEntity>> {
        let mut entities = Vec::new();
        
        // Basic entity extraction based on patterns
        // در آینده با NER model تکمیل می‌شه
        
        let words: Vec<&str> = query.split_whitespace().collect();
        
        // Detect potential technical terms (words with specific patterns)
        for word in words {
            if word.len() > 4 {
                // Simple heuristic برای technical terms
                if word.chars().any(|c| c.is_uppercase()) || 
                   word.contains('_') || 
                   self.looks_like_technical_term(word) {
                    entities.push(QueryEntity {
                        text: word.to_string(),
                        entity_type: "technical_term".to_string(),
                        confidence: 0.7,
                    });
                }
            }
        }

        Ok(entities)
    }

    fn looks_like_technical_term(&self, word: &str) -> bool {
        // Simple heuristic برای technical terms
        let technical_indicators = ["api", "sdk", "lib", "framework", "database", "server", "client"];
        technical_indicators.iter().any(|&indicator| 
            word.to_lowercase().contains(indicator)
        )
    }

    /// Generate search suggestions بر اساس query analysis
    pub async fn generate_suggestions(&self, analysis: &QueryAnalysis) -> Result<Vec<SearchSuggestion>> {
        let mut suggestions = Vec::new();
        
        match &analysis.intent {
            SearchIntent::Tutorial => {
                suggestions.push(SearchSuggestion {
                    text: format!("{} step by step", analysis.original_query),
                    reason: "Step-by-step tutorial".to_string(),
                });
            }
            SearchIntent::Comparison => {
                suggestions.push(SearchSuggestion {
                    text: format!("{} alternatives", analysis.original_query),
                    reason: "Alternative options".to_string(),
                });
            }
            _ => {}
        }

        // Add general suggestions
        if analysis.original_query.len() < 5 {
            suggestions.push(SearchSuggestion {
                text: format!("{} examples", analysis.original_query),
                reason: "Practical examples".to_string(),
            });
        }

        Ok(suggestions)
    }
}

#[derive(Debug, Clone, Default)]
pub struct QueryAnalysis {
    /// Field `original_query`.
    pub original_query: String,
    /// Field `language`.
    pub language: Option<String>,
    /// Field `intent`.
    pub intent: SearchIntent,
    /// Field `complexity`.
    pub complexity: QueryComplexity,
    /// Field `entities`.
    pub entities: Vec<QueryEntity>,
    /// Field `confidence`.
    pub confidence: f32,
}

#[derive(Debug, Clone)]
pub enum SearchIntent {
    /// Variant `General`.
    General,
    /// Variant `Tutorial`.
    Tutorial,
    /// Variant `Comparison`.
    Comparison,
    /// Variant `News`.
    News,
    /// Variant `Commercial`.
    Commercial,
    /// Variant `Factual`.
    Factual,
}

impl Default for SearchIntent {
    fn default() -> Self {
        Self::General
    }
}

#[derive(Debug, Clone)]
pub enum QueryComplexity {
    /// Variant `Simple`.
    Simple,
    /// Variant `Medium`.
    Medium,
    /// Variant `Complex`.
    Complex,
}

impl Default for QueryComplexity {
    fn default() -> Self {
        Self::Medium
    }
}

#[derive(Debug, Clone)]
pub struct QueryEntity {
    /// Field `text`.
    pub text: String,
    /// Field `entity_type`.
    pub entity_type: String,
    /// Field `confidence`.
    pub confidence: f32,
}

#[derive(Debug, Clone)]
pub struct SearchSuggestion {
    /// Field `text`.
    pub text: String,
    /// Field `reason`.
    pub reason: String,
}



