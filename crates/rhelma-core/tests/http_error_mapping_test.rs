use rhelma_core::prelude::*;

#[test]
fn all_core_errors_map_to_correct_http_status() {
    let ctx = RequestContext::empty();

    let cases = vec![
        (RhelmaError::BadRequest("x".into()), 400),
        (RhelmaError::Auth("x".into()), 401),
        (RhelmaError::Authz("x".into()), 403),
        (RhelmaError::NotFound("x".into()), 404),
        (RhelmaError::Conflict("x".into()), 409),
        (RhelmaError::RateLimited("x".into()), 429),
        (RhelmaError::residency_violation("x"), 451),
        (RhelmaError::Internal, 500),
    ];

    for (err, expected) in cases {
        let (status, _) = err.into_http(&ctx);
        assert_eq!(status.as_u16(), expected);
    }
}
