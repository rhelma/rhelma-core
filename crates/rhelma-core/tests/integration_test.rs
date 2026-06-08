use rhelma_core::prelude::*;

#[test]
fn request_context_from_headers() {
    let headers = vec![
        ("x-correlation-id", "corr-1"),
        // MUST be a valid UUID
        ("x-request-id", "123e4567-e89b-12d3-a456-426614174000"),
        ("x-tenant-id", "tenant-1"),
        ("x-region", "eu-west-1"),
    ];

    let ctx = RequestContext::from_headers(headers).expect("valid headers");

    assert_eq!(ctx.correlation_id(), Some("corr-1"));
    assert!(ctx.request_id().to_string().starts_with("123e4567"));
    assert!(ctx.has_tenant());
    assert!(ctx.has_region());
}
