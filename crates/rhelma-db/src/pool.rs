#![forbid(unsafe_code)]

use crate::error::{DbError, DbResult};
use crate::metrics;
use crate::types::RegionId;

use rhelma_core::RequestContext;
use sqlx::{Pool, Postgres};

/// Thin wrapper around a `sqlx::Pool<Postgres>` with Rhelma-specific policy hooks.
///
/// Design goals (Rhelma v5.2):
/// - **Zero-trust**: never trust caller-provided region/tenant; validate/compare only.
/// - **Residency-aware**: optionally enforce DB region == request region.
/// - **Observability-first**: expose pool gauges (size/idle) for rhelma-metrics.
#[derive(Clone)]
pub struct Database {
    pool: Pool<Postgres>,
    pool_name: &'static str,
    region: Option<RegionId>,
    enforce_residency: bool,
}

impl Database {
    /// Create a database wrapper from an existing `sqlx` pool.
    ///
    /// Defaults:
    /// - `pool_name = "db"`
    /// - residency enforcement disabled
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self {
            pool,
            pool_name: "db",
            region: None,
            enforce_residency: false,
        }
    }

    /// Human-friendly pool name used for metrics labels.
    pub fn with_pool_name(mut self, pool_name: &'static str) -> Self {
        self.pool_name = pool_name;
        self
    }

    /// Configure DB region and optionally enforce residency policy.
    ///
    /// If `enforce = true`, requests without a region will be rejected with
    /// `DbError::ResidencyViolation`.
    pub fn with_region(mut self, region: RegionId, enforce: bool) -> Self {
        self.region = Some(region);
        self.enforce_residency = enforce;
        self
    }

    /// Borrow the underlying pool.
    pub fn pool(&self) -> &Pool<Postgres> {
        &self.pool
    }

    /// Configured DB region (if any).
    pub fn region(&self) -> Option<&RegionId> {
        self.region.as_ref()
    }

    /// Record pool gauges via `rhelma-metrics`.
    ///
    /// This is intentionally a synchronous snapshot to keep it cheap.
    pub fn record_pool_gauges(&self) {
        let size_u32 = self.pool.size();
        let idle_usize = self.pool.num_idle();
        let idle_u32: u32 = idle_usize.min(u32::MAX as usize) as u32;
        metrics::set_pool_gauges(self.pool_name, size_u32, idle_u32);
    }

    /// Enforce residency if enabled; otherwise returns `Ok(())`.
    ///
    /// Rules:
    /// - If enforcement is disabled → OK
    /// - If DB region is not configured → OK (can't compare)
    /// - If enforcement enabled and request has no region → violation
    /// - If request region != db region → violation
    pub fn enforce_residency_or_ok(&self, ctx: &RequestContext) -> DbResult<()> {
        if !self.enforce_residency {
            return Ok(());
        }

        let Some(db_region) = self.region.as_ref() else {
            // Not configured, cannot enforce.
            return Ok(());
        };

        let tenant = ctx.tenant_id().map(|t| t.as_str().to_string());
        let requested_region = ctx.region().map(|r| r.as_str().to_string());
        let db_region_str = Some(db_region.as_str().to_string());

        match ctx.region() {
            Some(r) if r.as_str() == db_region.as_str() => Ok(()),
            Some(_) | None => Err(DbError::ResidencyViolation {
                tenant,
                requested_region,
                db_region: db_region_str,
            }),
        }
    }
}
