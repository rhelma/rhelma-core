#![forbid(unsafe_code)]

use rhelma_core::error_v52::{ErrorEnvelopeV52, ErrorSeverity, ErrorV52};
use serde_json::json;

#[test]
fn error_v52_json_snapshot() {
    let env = ErrorEnvelopeV52::new(
        ErrorV52 {
            error_code: "RATE_LIMITED".to_string(),
            http_status: 429,
            message: "Too many requests".to_string(),
            retryable: true,
            severity: ErrorSeverity::Warning,
            retry_after_ms: Some(10_000),

            request_id: "018d3c9f-2f4a-7d26-9d6f-5e6f8f4e1d10".to_string(),
            correlation_id: "018d3c9f-2f4a-7d26-9d6f-5e6f8f4e1d10".to_string(),
            traceparent: Some(
                "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01".to_string(),
            ),

            // v5.2 wire key is "context", but our struct field is `details`
            // to preserve older code paths.
            details: Some(json!({"limit": 100, "window_seconds": 60})),

            // deterministic snapshot
            timestamp: "2025-01-01T00:00:00.000Z".to_string(),
            stack_trace: None,
        }
        .with_now_timestamp(),
    );

    insta::assert_json_snapshot!(env);
}
