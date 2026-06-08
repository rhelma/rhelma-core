#![forbid(unsafe_code)]

use rhelma_event::{EventRequestContext, EventRequestFlags, EventTraceContext};

#[test]
fn request_context_inherit_fills_missing_ids() {
    let parent = EventRequestContext {
        request_id: None,
        correlation_id: None,
        tenant_id: Some("t1".to_string()),
        user_id: Some("u1".to_string()),
        flags: EventRequestFlags {
            system: true,
            ai_safe: true,
            read_only: false,
        },
    };

    let child = EventRequestContext::inherit_or_generate(&parent);
    assert!(child
        .request_id
        .as_deref()
        .is_some_and(|s| !s.trim().is_empty()));
    assert!(child
        .correlation_id
        .as_deref()
        .is_some_and(|s| !s.trim().is_empty()));
    assert_eq!(child.tenant_id.as_deref(), Some("t1"));
    assert_eq!(child.user_id.as_deref(), Some("u1"));
}

#[test]
fn trace_context_child_preserves_trace_and_sets_parent_span() {
    let parent = EventTraceContext {
        trace_id: Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string()),
        span_id: Some("bbbbbbbbbbbbbbbb".to_string()),
        tracestate: None,
        baggage: None,
        parent_span_id: None,
    };

    let child = EventTraceContext::child_of(&parent);
    assert_eq!(
        child.trace_id.as_deref(),
        Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
    );
    assert_eq!(child.parent_span_id.as_deref(), Some("bbbbbbbbbbbbbbbb"));
    assert!(child.span_id.as_deref().is_some_and(|s| s.len() == 16));
}
