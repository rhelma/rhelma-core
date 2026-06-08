use rhelma_core::HttpErrorMapping;
use rhelma_core::RequestContext;
use rhelma_core::{ErrorExt, RhelmaError, RhelmaResult};

fn fail() -> RhelmaResult<()> {
    Err(RhelmaError::Validation("missing field".into()))
}

#[test]
fn rhelma_context_adds_human_readable_context() {
    let err = fail()
        .rhelma_context("while doing something important")
        .unwrap_err();
    let msg = err.to_string();

    assert!(msg.contains("missing field"));
    assert!(msg.contains("while doing something important"));
}

#[test]
fn http_error_mapping_for_validation_error() {
    let err = RhelmaError::Validation("missing field".into());

    let ctx = RequestContext::empty();
    let (status, body) = err.into_http(&ctx);

    assert_eq!(status.as_u16(), 400);
    assert_eq!(body.type_label, "validation");
    assert_eq!(body.code, 400);
}
