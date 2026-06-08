use rhelma_core::prelude::*;
use uuid::Uuid;

#[test]
fn missing_request_id_generates_uuid() {
    let headers = vec![("x-correlation-id", "abc")];

    // MUST pass ownership, not &headers
    let ctx = RequestContext::from_headers(headers).unwrap();

    let id = ctx.request_id();

    // RequestContext ALWAYS has a UUID, even if missing
    assert_ne!(id, Uuid::nil());
}

//
// TEST 2 — Invalid region must be ignored (not error)
//
#[test]
fn invalid_region_is_ignored() {
    let headers = vec![
        ("x-request-id", "123e4567-e89b-12d3-a456-426614174000"),
        ("x-region", "INVALID_REGION"),
    ];

    let ctx = RequestContext::from_headers(headers).unwrap();

    // invalid → silently ignored
    assert!(ctx.region().is_none());
}

//
// TEST 3 — Invalid UUID in x-request-id must produce BadRequest
//
#[test]
fn invalid_request_id_is_rejected() {
    let headers = vec![("x-request-id", "NOT-A-UUID"), ("x-correlation-id", "abc")];

    // MUST pass headers, NOT &headers
    let err = RequestContext::from_headers(headers).unwrap_err();

    assert!(matches!(err, RhelmaError::BadRequest(_)));

    let msg = err.to_string();
    assert!(msg.contains("invalid request_id"));
}
