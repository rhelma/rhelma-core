#![forbid(unsafe_code)]

use rhelma_core::RequestContext;
use tracing::Span;
use uuid::Uuid;

/// db.query span (semantic-conventions aligned).
///
/// Attributes:
/// - db.system, db.operation, db.table
pub fn db_span(op: &str, table: Option<&str>) -> Span {
    tracing::info_span!(
        "db.query",
        db.system = "postgres",
        db.operation = op,
        db.table = table.unwrap_or("unknown"),
    )
}

/// db.query span enriched with Rhelma `RequestContext`.
///
/// We attach both:
/// - **Rhelma fields** (request_id/correlation_id/trace_id/span_id) for compatibility
/// - **v5.2 / OTEL-ish fields** (request.id, correlation.id, tenant.id, user.id, region)
pub fn db_span_ctx(ctx: &RequestContext, op: &str, table: Option<&str>) -> Span {
    // Avoid per-query heap allocations: keep IDs as &str / Uuid where possible.
    let tenant = ctx.tenant_id().map(|t| t.as_str()).unwrap_or("");
    let region = ctx.region().map(|r| r.as_str()).unwrap_or("");
    let user_uuid = ctx.user_id().map(|u| u.as_uuid()).unwrap_or_else(Uuid::nil);

    tracing::info_span!(
        "db.query",
        // OTEL-ish / Rhelma v5.2
        request.id = %ctx.request_id(),
        correlation.id = ctx.correlation_id().unwrap_or(""),
        tenant.id = tenant,
        user.id = %user_uuid,
        region = region,
        // Trace context (best effort)
        trace.id = ctx.trace().trace_id.as_deref().unwrap_or(""),
        span.id = ctx.trace().span_id.as_deref().unwrap_or(""),
        // DB
        db.system = "postgres",
        db.operation = op,
        db.table = table.unwrap_or("unknown"),
        // Back-compat names (kept; low cardinality)
        request_id = %ctx.request_id(),
        correlation_id = ctx.correlation_id(),
        trace_id = ctx.trace().trace_id.as_deref(),
        span_id = ctx.trace().span_id.as_deref(),
    )
}
