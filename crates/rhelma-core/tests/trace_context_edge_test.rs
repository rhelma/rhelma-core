use rhelma_core::TraceContext;

#[test]
fn invalid_traceparent_falls_back_to_generated() {
    let headers = vec![
        // invalid format (not 4 parts)
        ("traceparent", "00-not-a-traceparent"),
    ];

    let ctx = TraceContext::extract_from_headers(headers);

    // Must always generate a safe, valid context
    assert!(ctx.current_trace_id().is_some());
    assert!(ctx.to_traceparent().is_some());
}

#[test]
fn orphan_span_id_generates_new_trace() {
    let spoofed_span = "00f067aa0ba902b7";
    let headers = vec![("x-span-id", spoofed_span)];

    let ctx = TraceContext::extract_from_headers(headers);

    // A span-id without a trace-id is untrusted: we generate a fresh context.
    assert!(ctx.current_trace_id().is_some());
    let tp = ctx.to_traceparent().expect("traceparent must be buildable");

    // And we must not accept the spoofed span-id as-is.
    assert!(!tp.contains(spoofed_span));
}
