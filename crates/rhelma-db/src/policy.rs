use crate::{Database, DbResult};
use rhelma_core::RequestContext;

pub trait DbPolicy: Send + Sync {
    /// fn `check`.
    fn check(&self, ctx: &RequestContext, db: &Database) -> DbResult<()>;
}

/// Default: enforce residency فقط اگر Database خودش enforce_residency فعال کرده باشد.
#[derive(Default)]
pub struct DatabaseResidencyPolicy;

impl DbPolicy for DatabaseResidencyPolicy {
    fn check(&self, ctx: &RequestContext, db: &Database) -> DbResult<()> {
        db.enforce_residency_or_ok(ctx)
    }
}

/// No-op policy (اگر خواستی strict رو خاموش کنی)
#[derive(Default)]
pub struct NoopDbPolicy;

impl DbPolicy for NoopDbPolicy {
    fn check(&self, _ctx: &RequestContext, _db: &Database) -> DbResult<()> {
        Ok(())
    }
}
