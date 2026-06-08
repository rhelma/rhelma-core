//! Unified HTTP error mapping for Rhelma Platform v5.1.1
//!
//! Canonical error body fields:
//! - code            → HTTP status code
//! - type_label      → stable logical error type (machine-friendly)
//! - message         → non-sensitive message (Zero-Trust)
//! - request_id      → UUID from RequestContext
//! - correlation_id  → optional correlation chain ID
//! - trace_id        → OpenTelemetry trace identifier
//!
//! Contract notes:
//! - rhelma-core MUST NOT implement HTTP-framework responses (e.g. axum IntoResponse).
//! - rhelma-core provides a stable mapping to (StatusCode, HttpErrorBody).
//! - Gateways / HTTP adapters serialize this payload.

use http::StatusCode;
use serde::Serialize;

use crate::{RequestContext, RhelmaError};

/// Standard Rhelma error response payload.
/// This shape is guaranteed stable for all external/public APIs.
#[derive(Debug, Clone, Serialize)]
pub struct HttpErrorBody {
    /// HTTP status code that the server returns.
    pub code: u16,

    /// Logical, stable error type for metrics/logging dashboards.
    ///
    /// NOTE: For residency violations this MUST be "residency_violation".
    pub type_label: String,

    /// Human-readable, non-sensitive message.
    pub message: String,

    /// Request-level correlation uuid.
    pub request_id: String,

    /// Caller-level correlation id, if any.
    pub correlation_id: Option<String>,

    /// Distributed tracing identifier (traceparent → trace_id).
    pub trace_id: Option<String>,
}

/// Error-to-HTTP converter trait.
///
/// Services should return an `(StatusCode, HttpErrorBody)` tuple that the HTTP layer
/// serializes into a final JSON response.
pub trait HttpErrorMapping {
    /// fn `into_http`.
    fn into_http(self, ctx: &RequestContext) -> (StatusCode, HttpErrorBody);
}

impl HttpErrorMapping for RhelmaError {
    fn into_http(self, ctx: &RequestContext) -> (StatusCode, HttpErrorBody) {
        use RhelmaError::*;

        // ---------------------------------------------------------------------
        // SAFETY MESSAGE MAPPING (Zero-Trust Rule)
        //
        // ALL internal details (DB errors, config values, stack traces)
        // MUST NOT be exposed to users. Only safe, generic messages are allowed.
        // ---------------------------------------------------------------------
        let (status, safe_message) = match &self {
            Validation(_) => (StatusCode::BAD_REQUEST, "Validation failed"),
            BadRequest(_) => (StatusCode::BAD_REQUEST, "Bad request"),
            NotFound(_) => (StatusCode::NOT_FOUND, "Resource not found"),
            Conflict(_) => (StatusCode::CONFLICT, "Resource conflict"),

            Auth(_) => (StatusCode::UNAUTHORIZED, "Authentication failed"),
            Authz(_) => (StatusCode::FORBIDDEN, "Access forbidden"),

            RateLimited(_) => (StatusCode::TOO_MANY_REQUESTS, "Rate limit exceeded"),

            SecurityPolicy(_) => {
                if self.is_residency_violation() {
                    (
                        StatusCode::UNAVAILABLE_FOR_LEGAL_REASONS,
                        "Residency violation",
                    )
                } else {
                    (StatusCode::FORBIDDEN, "Security policy violation")
                }
            }

            Dependency(_) => (StatusCode::BAD_GATEWAY, "Upstream dependency error"),
            CircuitOpen(_) => (StatusCode::SERVICE_UNAVAILABLE, "Circuit breaker open"),
            DistributedTx(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Distributed transaction failed",
            ),

            Cache(_) | Config(_) | Database(_) | Internal | Other(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")
            }
        };

        // type_label must be stable and policy-aware.
        // For residency violations we expose a dedicated label regardless of RhelmaError::as_str().
        let type_label = if self.is_residency_violation() {
            RhelmaError::RESIDENCY_VIOLATION_CODE.to_string()
        } else {
            self.as_str().to_string()
        };

        // Extract trace_id from TraceContext (best-effort).
        // NOTE: If ctx.trace() or current_trace_id() isn't available in your RequestContext,
        // adjust this line to your actual trace accessor.
        let trace_id = ctx.trace().current_trace_id().map(|s| s.to_string());

        // Compose error body
        let body = HttpErrorBody {
            code: status.as_u16(),
            type_label,
            message: safe_message.to_string(),

            request_id: ctx.request_id().to_string(),
            correlation_id: ctx.correlation_id().map(|s| s.to_string()),
            trace_id,
        };

        (status, body)
    }
}
