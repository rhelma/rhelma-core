//! Tracing helpers for rhelma-auth (Rhelma-aligned).
//!
//! Keep this module tiny: we only expose a stable span name for auth operations.

use tracing::{info_span, Span};

/// Create a span for authentication / authorization work.
pub fn auth_span(op: &'static str) -> Span {
    info_span!("auth", op = op)
}
