use rhelma_core::prelude::*;
use rhelma_core::request_context_v52::RequestContextV52;

const UUIDV7_REQ: &str = "018d3c9f-2f4a-7d26-9d6f-5e6f8f4e1d10";
const UUIDV7_CORR: &str = "018d3ca0-3b2f-7c11-9a5d-2f1a0c9b8d22";

// Valid W3C traceparent
const TRACEPARENT_OK: &str = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01";

fn valid_external_headers() -> Vec<(&'static str, &'static str)> {
    vec![
        ("x-tenant-id", "acme"),
        ("x-region", "eu-west-1"),
        ("x-residency", "REGIONAL_STRICT"),
        ("x-rhelma-request-id", UUIDV7_REQ),
        ("x-rhelma-correlation-id", UUIDV7_CORR),
        ("traceparent", TRACEPARENT_OK),
    ]
}

#[test]
fn external_requires_tenant_and_region() {
    // Missing everything -> must fail
    let ctx = RequestContext::empty();
    let err = RequestContextV52::validate_external(&ctx).unwrap_err();
    assert!(matches!(err, RhelmaError::BadRequest(_)));

    // Missing tenant -> fail
    let ctx2 = RequestContext::from_headers(vec![
        ("x-region", "eu-west-1"),
        ("x-residency", "REGIONAL_STRICT"),
        ("x-rhelma-request-id", UUIDV7_REQ),
        ("x-rhelma-correlation-id", UUIDV7_CORR),
        ("traceparent", TRACEPARENT_OK),
    ])
    .expect("from_headers must succeed");
    assert!(RequestContextV52::validate_external(&ctx2).is_err());

    // Missing region -> fail
    let ctx3 = RequestContext::from_headers(vec![
        ("x-tenant-id", "acme"),
        ("x-residency", "REGIONAL_STRICT"),
        ("x-rhelma-request-id", UUIDV7_REQ),
        ("x-rhelma-correlation-id", UUIDV7_CORR),
        ("traceparent", TRACEPARENT_OK),
    ])
    .expect("from_headers must succeed");
    assert!(RequestContextV52::validate_external(&ctx3).is_err());
}

#[test]
fn internal_allows_missing_tenant() {
    let ctx = RequestContext::empty();
    assert!(RequestContextV52::validate_internal(&ctx).is_ok());
}

#[test]
fn external_with_only_tenant_and_region_is_not_enough() -> Result<(), RhelmaError> {
    // Old behavior test: tenant+region only.
    // In strict v5.2 this must FAIL because residency/correlation are required.
    let ctx = RequestContext::empty()
        .with_tenant(TenantId::parse("acme")?)
        .with_region(RegionId::parse("eu-west-1")?);

    assert!(RequestContextV52::validate_external(&ctx).is_err());
    Ok(())
}

#[test]
fn external_with_tenant_and_region_is_ok() -> Result<(), RhelmaError> {
    // Strict v5.2 compliant: all required headers present
    let ctx =
        RequestContext::from_headers(valid_external_headers()).expect("from_headers must succeed");

    assert!(RequestContextV52::validate_external(&ctx).is_ok());
    Ok(())
}
