#![forbid(unsafe_code)]

use std::collections::HashMap;

use rhelma_event::{transport_headers, EventRequestContext, EventTraceContext, Residency};

#[test]
fn nats_like_headers_roundtrip_preserves_context() {
    let request = EventRequestContext {
        request_id: Some("018d2e33-9c90-7b16-bc49-0e1b2d87c9f1".to_string()),
        correlation_id: Some("018d2e33-9c90-7b16-bc49-0e1b2d87c9f2".to_string()),
        tenant_id: None,
        user_id: None,
        flags: Default::default(),
    };
    let trace = EventTraceContext {
        trace_id: Some("4bf92f3577b34da6a3ce929d0e0e4736".to_string()),
        span_id: Some("00f067aa0ba902b7".to_string()),
        tracestate: None,
        baggage: None,
        parent_span_id: None,
    };

    let headers =
        transport_headers::headers_from_context(&request, &trace, Residency::RegionalOnly);

    assert!(headers.contains_key("x-rhelma-request-id"));
    assert!(headers.contains_key("x-rhelma-correlation-id"));
    assert!(headers.contains_key("x-residency"));
    assert!(headers.contains_key("traceparent"));

    let (req2, trace2, res2) = transport_headers::extract_context_from_headers(&headers);

    assert_eq!(req2.request_id, request.request_id);
    assert_eq!(req2.correlation_id, request.correlation_id);
    assert_eq!(trace2.trace_id, trace.trace_id);
    assert_eq!(trace2.span_id, trace.span_id);
    assert!(matches!(res2, Residency::RegionalOnly));
}

#[test]
fn extraction_accepts_legacy_correlation_key() {
    let mut headers = HashMap::new();
    headers.insert("x-request-id".to_string(), "req-1".to_string());
    headers.insert("x-correlation-id".to_string(), "corr-1".to_string());
    headers.insert("x-residency".to_string(), "GLOBAL".to_string());
    headers.insert(
        "traceparent".to_string(),
        "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01".to_string(),
    );

    let (req, trace, res) = transport_headers::extract_context_from_headers(&headers);
    assert_eq!(req.request_id.as_deref(), Some("req-1"));
    assert_eq!(req.correlation_id.as_deref(), Some("corr-1"));
    assert_eq!(
        trace.trace_id.as_deref(),
        Some("4bf92f3577b34da6a3ce929d0e0e4736")
    );
    assert_eq!(trace.span_id.as_deref(), Some("00f067aa0ba902b7"));
    assert!(matches!(res, Residency::Global));
}
