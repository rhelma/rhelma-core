#![forbid(unsafe_code)]

use rhelma_core::request_context::{RequestFlags, ResidencyPolicy};
use rhelma_core::RequestContext;

#[test]
fn request_context_parses_canonical_v52_headers() {
    let rid = uuid::Uuid::now_v7().to_string();
    let cid = uuid::Uuid::now_v7().to_string();
    let tp = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01";

    let headers = vec![
        ("x-rhelma-request-id", rid.as_str()),
        ("x-rhelma-correlation-id", cid.as_str()),
        ("x-residency", "GLOBAL"),
        ("x-rhelma-flag-read-only", "true"),
        ("traceparent", tp),
    ];

    let ctx = RequestContext::from_headers(headers).expect("must parse v5.2 headers");

    assert_eq!(ctx.request_id().to_string(), rid);
    assert_eq!(ctx.correlation_id().unwrap(), cid);
    assert_eq!(ctx.residency(), Some(ResidencyPolicy::Global));
    assert_eq!(
        ctx.flags(),
        &RequestFlags {
            read_only: true,
            dry_run: false,
            ai_safe_mode: false,
            debug_mode: false
        }
    );

    let trace_id = ctx.trace().current_trace_id().expect("trace id present");
    assert_eq!(trace_id, "4bf92f3577b34da6a3ce929d0e0e4736");
}

#[test]
fn invalid_request_id_is_hard_fail() {
    let headers = vec![("x-rhelma-request-id", "not-a-uuid")];
    let err = RequestContext::from_headers(headers).expect_err("must error");
    let msg = err.to_string().to_ascii_lowercase();
    assert!(msg.contains("invalid") && msg.contains("request"));
}

#[test]
fn invalid_traceparent_falls_back_to_generated() {
    let rid = uuid::Uuid::now_v7().to_string();

    let ctx = RequestContext::from_headers(vec![
        ("x-rhelma-request-id", rid.as_str()),
        (
            "traceparent",
            "00-00000000000000000000000000000000-0000000000000000-01",
        ),
    ])
    .expect("must parse");

    let trace_id = ctx.trace().current_trace_id().expect("trace id present");
    assert_ne!(trace_id, "00000000000000000000000000000000");
    assert_eq!(trace_id.len(), 32);
}

#[test]
fn orphan_span_id_generates_new_trace() {
    let rid = uuid::Uuid::now_v7().to_string();

    let ctx = RequestContext::from_headers(vec![
        ("x-rhelma-request-id", rid.as_str()),
        ("x-span-id", "00f067aa0ba902b7"),
    ])
    .expect("must parse");

    assert!(ctx.trace().current_trace_id().is_some());
    assert_eq!(ctx.trace().current_trace_id().unwrap().len(), 32);
}
