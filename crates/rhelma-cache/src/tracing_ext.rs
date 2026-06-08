// rhelma-cache v0.2.0-enterprise-pro: tracing helpers for cache operations.

use tracing::Span;

/// Create a standard cache span for operations like get/set/delete.
///
/// `backend`: memory / redis / layered / other
/// `key_space`: logical grouping or prefix (e.g. "session", "config", ...).
pub fn cache_span(operation: &str, backend: &str, key_space: Option<&str>) -> Span {
    tracing::info_span!(
        "cache.op",
        cache.operation = operation,
        cache.backend = backend,
        cache.key_space = key_space.unwrap_or("default"),
    )
}
