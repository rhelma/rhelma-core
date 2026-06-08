use crate::{RequestContext, RhelmaError};
use uuid::Uuid;

/// RequestContext validation rules for Rhelma v5.2 (STRICT).
///
/// External entry-points MUST enforce:
/// - x-tenant-id
/// - x-region
/// - x-residency
/// - x-rhelma-request-id: uuidv7
/// - x-rhelma-correlation-id: uuidv7
/// - traceparent: present/valid (at least structurally valid)
///
/// Internal calls remain relaxed.
pub struct RequestContextV52;

impl RequestContextV52 {
    /// STRICT validation for external HTTP APIs (edge / gateway entry points).
    pub fn validate_external(ctx: &RequestContext) -> Result<(), RhelmaError> {
        // Required: tenant
        if ctx.tenant_id().is_none() {
            return Err(RhelmaError::BadRequest(
                "missing required header: x-tenant-id".to_string(),
            ));
        }

        // Required: region
        if ctx.region().is_none() {
            return Err(RhelmaError::BadRequest(
                "missing required header: x-region".to_string(),
            ));
        }

        // Required: residency
        if ctx.residency().is_none() {
            return Err(RhelmaError::BadRequest(
                "missing required header: x-residency".to_string(),
            ));
        }

        // Required: request-id must be uuidv7
        if !is_uuid_v7(&ctx.request_id()) {
            return Err(RhelmaError::BadRequest(
                "x-rhelma-request-id must be uuidv7".to_string(),
            ));
        }

        // Required: correlation-id must exist and be uuidv7
        let cid = ctx.correlation_id().ok_or_else(|| {
            RhelmaError::BadRequest("missing required header: x-rhelma-correlation-id".to_string())
        })?;
        let u = Uuid::parse_str(cid)
            .map_err(|_| RhelmaError::BadRequest("invalid x-rhelma-correlation-id".to_string()))?;
        if !is_uuid_v7(&u) {
            return Err(RhelmaError::BadRequest(
                "x-rhelma-correlation-id must be uuidv7".to_string(),
            ));
        }

        // Required: traceparent (structurally valid)
        //
        // NOTE: Without a "was_present" flag in RequestContext, we can only validate
        // the resulting value. Edge middleware should enforce "presence" at headers
        // if you want strict "must be provided by caller".
        let tp = ctx.trace().to_traceparent().ok_or_else(|| {
            RhelmaError::BadRequest("missing required header: traceparent".to_string())
        })?;
        if !is_valid_traceparent(&tp) {
            return Err(RhelmaError::BadRequest("invalid traceparent".to_string()));
        }

        Ok(())
    }

    /// Relaxed validation for internal/system calls.
    pub fn validate_internal(_ctx: &RequestContext) -> Result<(), RhelmaError> {
        Ok(())
    }
}

fn is_uuid_v7(u: &Uuid) -> bool {
    // version nibble is high 4 bits of byte 6
    let b = u.as_bytes();
    (b[6] >> 4) == 0x7
}

/// Minimal strict validation for W3C traceparent value:
/// - 4 parts separated by '-'
/// - trace-id: 32 hex and not all-zero
/// - span-id: 16 hex and not all-zero
/// - version/flags: 2 hex
fn is_valid_traceparent(tp: &str) -> bool {
    let s = tp.trim();
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 4 {
        return false;
    }

    let version = parts[0];
    let trace_id = parts[1];
    let span_id = parts[2];
    let flags = parts[3];

    if version.len() != 2 || flags.len() != 2 {
        return false;
    }
    if !is_hex(version) || !is_hex(flags) {
        return false;
    }
    if trace_id.len() != 32 || !is_hex(trace_id) || is_all_zeros(trace_id) {
        return false;
    }
    if span_id.len() != 16 || !is_hex(span_id) || is_all_zeros(span_id) {
        return false;
    }
    true
}

fn is_hex(s: &str) -> bool {
    s.chars().all(|c| c.is_ascii_hexdigit())
}

fn is_all_zeros(s: &str) -> bool {
    s.chars().all(|c| c == '0')
}
