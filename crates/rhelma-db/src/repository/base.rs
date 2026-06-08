use crate::error::{DbError, DbResult};
use crate::metrics::{self, DbOperation, DbOutcome};
use crate::policy::{DatabaseResidencyPolicy, DbPolicy};
use crate::tracing_ext::db_span_ctx;
use crate::Database;

use rhelma_core::RequestContext;
use std::future::Future;
use std::sync::Arc;
use std::time::Instant;
use tracing::Instrument;

#[derive(Clone)]
pub struct BaseRepository {
    db: Database,
    policy: Arc<dyn DbPolicy>,
}

impl BaseRepository {
    pub fn new(db: Database) -> Self {
        Self {
            db,
            policy: Arc::new(DatabaseResidencyPolicy),
        }
    }

    pub fn with_policy(mut self, policy: Arc<dyn DbPolicy>) -> Self {
        self.policy = policy;
        self
    }

    pub fn db(&self) -> &Database {
        &self.db
    }

    pub async fn run_db<T, Fut, F>(
        &self,
        ctx: &RequestContext,
        op: DbOperation,
        table: Option<&str>,
        f: F,
    ) -> DbResult<T>
    where
        F: FnOnce(sqlx::PgPool) -> Fut,
        Fut: Future<Output = Result<T, sqlx::Error>>,
    {
        // Policy check (residency و آینده)
        self.policy.check(ctx, &self.db)?;

        let span = db_span_ctx(ctx, op.as_str(), table);
        let start = Instant::now();

        let pool = self.db.pool().clone();
        let res = f(pool).instrument(span).await;

        let dur = start.elapsed();
        match &res {
            Ok(_) => metrics::record(op, DbOutcome::Success, dur),
            Err(_) => metrics::record(op, DbOutcome::Error, dur),
        }

        res.map_err(DbError::from_sqlx)
    }
}
