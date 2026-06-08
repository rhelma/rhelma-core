use serde::Serialize;

/// Very lightweight query understanding placeholder.
///
/// The previous implementation had a fairly advanced engine; here we preserve
/// the structure and leave room for future AI-based intent detection.
#[derive(Debug, Clone, Serialize)]
pub struct QueryInsights {
    /// Field `intent`.
    pub intent: String,
    /// Field `complexity`.
    pub complexity: String,
}

impl QueryInsights {
    pub fn analyze(query: &str) -> Self {
        let intent = if query.ends_with("?") {
            "question"
        } else if query.len() < 20 {
            "short_query"
        } else {
            "long_query"
        };

        let complexity = if query.split_whitespace().count() > 8 {
            "complex"
        } else {
            "simple"
        };

        Self {
            intent: intent.to_string(),
            complexity: complexity.to_string(),
        }
    }
}
