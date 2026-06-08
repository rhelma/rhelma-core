/// Create a span with common attributes for server-style operations.
///
/// This macro wraps `tracing::info_span` and sets standard attributes like `otel.kind`.
#[macro_export]
macro_rules! instrument_span {
    ($name:expr) => {
        tracing::info_span!(
            $name,
            "otel.kind" = "server",
            "otel.status_code" = tracing::field::Empty,
        )
    };
    ($name:expr, $($key:expr => $value:expr),+ $(,)?) => {
        tracing::info_span!(
            $name,
            "otel.kind" = "server",
            "otel.status_code" = tracing::field::Empty,
            $($key = $value),+
        )
    };
}
