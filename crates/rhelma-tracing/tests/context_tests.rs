use std::collections::HashMap;

use rhelma_tracing::context;
use uuid::Uuid;

#[test]
fn context_roundtrip_legacy_headers() {
    context::clear_current_ids();

    let mut headers = HashMap::new();
    context::inject_current_context(&mut headers);

    // legacy compatibility headers are injected
    assert!(headers.contains_key("x-trace-id"));
    assert!(headers.contains_key("x-span-id"));

    let headers_clone = headers.clone();
    context::clear_current_ids();
    context::extract_current_context(&headers_clone);

    let current_trace = context::current_trace_id();
    let current_span = context::current_span_id();

    assert_eq!(current_trace, headers.get("x-trace-id").cloned());
    assert_eq!(current_span, headers.get("x-span-id").cloned());
}

#[test]
fn context_roundtrip_traceparent() {
    context::clear_current_ids();
    let mut headers = HashMap::new();
    context::inject_traceparent(&mut headers);

    assert!(headers.contains_key("traceparent"));

    let headers_clone = headers.clone();
    context::clear_current_ids();
    context::extract_traceparent(&headers_clone);

    assert!(context::current_trace_id().is_some());
    assert!(context::current_span_id().is_some());
}

#[test]
fn sampled_flag_roundtrip_via_traceparent() {
    context::clear_current_ids();
    let mut headers = HashMap::new();

    context::set_sampled(false);
    context::inject_traceparent(&mut headers);

    let headers_clone = headers.clone();
    context::clear_current_ids();
    context::extract_traceparent(&headers_clone);

    assert_eq!(context::is_sampled(), Some(false));
}

#[test]
fn invalid_traceparent_does_not_panic() {
    context::clear_current_ids();

    let mut headers = HashMap::new();
    headers.insert("traceparent".into(), "invalid-value".into());

    // Only requirement: MUST NOT panic.
    context::extract_traceparent(&headers);

    // No crash = success.
}

#[test]
fn correlation_id_header_roundtrip() {
    context::clear_current_ids();
    context::set_correlation_id("corr-123");

    let mut headers = HashMap::new();
    context::inject_current_context(&mut headers);

    assert_eq!(
        headers.get("x-correlation-id"),
        Some(&"corr-123".to_string())
    );

    let cloned = headers.clone();
    context::clear_current_ids();
    context::extract_current_context(&cloned);

    assert_eq!(
        context::current_correlation_id(),
        Some("corr-123".to_string())
    );
}

#[test]
fn request_id_header_roundtrip() {
    context::clear_current_ids();
    let rid = Uuid::now_v7().to_string();
    context::set_request_id(&rid);

    let mut headers = HashMap::new();
    context::inject_current_context(&mut headers);

    assert_eq!(headers.get("x-rhelma-request-id"), Some(&rid));
    assert_eq!(headers.get("x-request-id"), Some(&rid));

    let cloned = headers.clone();
    context::clear_current_ids();
    context::extract_current_context(&cloned);

    assert_eq!(context::current_request_id(), Some(rid));
}

#[test]
fn rhelma_correlation_id_preferred_over_legacy() {
    context::clear_current_ids();

    let corr = Uuid::now_v7().to_string();
    let mut headers = HashMap::new();
    headers.insert("x-rhelma-correlation-id".into(), corr.clone());
    headers.insert("x-correlation-id".into(), "legacy-should-not-win".into());

    context::extract_current_context(&headers);

    assert_eq!(context::current_correlation_id(), Some(corr));
}
