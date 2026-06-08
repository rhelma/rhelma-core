use rhelma_core::RequestContext;
use rhelma_db::{Database, DbError};
use sqlx::PgPool;

#[tokio::test(flavor = "current_thread")]
async fn residency_violation_blocks_when_enforced() {
    // connect_lazy does not require a live DB, but sqlx Pool still requires a Tokio runtime.
    let pool = PgPool::connect_lazy("postgres://postgres:postgres@localhost/postgres").unwrap();
    let db = Database::new(pool)
        .with_pool_name("main")
        // RegionId requires min length 3 ([a-z0-9-]{3,}).
        .with_region(rhelma_db::types::RegionId::parse("euw").unwrap(), true);

    let ctx = RequestContext::empty()
        .with_region(rhelma_core::types::ids::RegionId::parse("usa").unwrap());

    let res = db.enforce_residency_or_ok(&ctx);
    assert!(matches!(res, Err(DbError::ResidencyViolation { .. })));
}
