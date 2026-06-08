#![forbid(unsafe_code)]

use crate::models::query::{EnhancedSearchRequest, SearchHit};

#[cfg(feature = "query-understanding")]
use crate::query_understanding::QueryInsights;

// ✅ وقتی feature خاموش است: stub
#[cfg(not(feature = "query-understanding"))]
struct QueryInsights;

#[cfg(not(feature = "query-understanding"))]
impl QueryInsights {
    #[inline]
    fn analyze(_query: &str) -> Self {
        Self
    }
}

use crate::state::AppState;
use metrics::histogram;
use std::time::Instant;

pub async fn enhanced_search(
    state: &AppState,
    req: EnhancedSearchRequest,
) -> anyhow::Result<Vec<SearchHit>> {
    let start = Instant::now();

    // keep for future use; doesn't break build
    let _insights = QueryInsights::analyze(&req.query);

    let hits = state.hybrid.search(&req.query, req.limit).await?;

    let elapsed = start.elapsed();
    histogram!("search_enhanced_duration_seconds").record(elapsed.as_secs_f64());

    Ok(hits)
}
