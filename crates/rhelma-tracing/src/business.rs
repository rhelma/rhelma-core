#![forbid(unsafe_code)]
//! Business-level span helpers.
//!
//! `rhelma-tracing` focuses on transport correlation (trace/request/correlation IDs).
//! This module adds **optional, low-cardinality business fields** that can be
//! recorded on spans for better dashboards and investigations.
//!
//! ## Field names
//! We use underscore-separated identifiers because `tracing` span field names
//! must be valid Rust identifiers in macros.
//!
//! - `rhelma_operation`   (string)
//! - `rhelma_tenant_id`   (string)
//! - `rhelma_user_id`     (string)
//! - `rhelma_subject_id`  (string)
//! - `rhelma_value_amount`(i64)
//!
//! ## Usage
//! ```ignore
//! use rhelma_tracing::business::{business_span, BusinessSpanExt};
//!
//! let span = business_span!("process_order", "checkout");
//! let _guard = span.enter();
//! span.record_tenant_id("acme-corp");
//! span.record_value_amount(9999);
//! ```

use tracing::{field, Span};

use crate::context;

/// Create a span with standard business fields pre-declared.
///
/// The span name **must** be a string literal (e.g., `"my_operation"`).
/// This is a requirement of the `tracing::span!` macro.
///
/// # Example
/// ```ignore
/// let span = business_span!("checkout", "process_payment");
/// let _guard = span.enter();
/// span.record_tenant_id("acme-corp");
/// ```
#[macro_export]
macro_rules! business_span {
    ($name:literal, $operation:expr) => {{
        $crate::context::set_baggage_item("rhelma.operation", $operation);
        tracing::span!(
            tracing::Level::INFO,
            $name,
            rhelma_operation = %$operation,
            rhelma_tenant_id = tracing::field::Empty,
            rhelma_user_id = tracing::field::Empty,
            rhelma_subject_id = tracing::field::Empty,
            rhelma_value_amount = tracing::field::Empty,
        )
    }};
}

/// Create a business span with a specific tracing level.
///
/// # Example
/// ```ignore
/// let span = business_span_with_level!(tracing::Level::DEBUG, "debug_op", "inspect");
/// ```
#[macro_export]
macro_rules! business_span_with_level {
    ($level:expr, $name:literal, $operation:expr) => {{
        $crate::context::set_baggage_item("rhelma.operation", $operation);
        tracing::span!(
            $level,
            $name,
            rhelma_operation = %$operation,
            rhelma_tenant_id = tracing::field::Empty,
            rhelma_user_id = tracing::field::Empty,
            rhelma_subject_id = tracing::field::Empty,
            rhelma_value_amount = tracing::field::Empty,
        )
    }};
}

/// Extension helpers to record business fields.
///
/// Note: recording only works if the span declares these fields
/// (i.e., spans created via `business_span!` or `business_span_with_level!`).
pub trait BusinessSpanExt {
    /// Record the tenant ID on the span.
    fn record_tenant_id(&self, tenant_id: &str);
    /// Record the user ID on the span.
    fn record_user_id(&self, user_id: &str);
    /// Record the subject ID on the span.
    fn record_subject_id(&self, subject_id: &str);
    /// Record a value amount on the span.
    fn record_value_amount(&self, amount: i64);
}

impl BusinessSpanExt for Span {
    fn record_tenant_id(&self, tenant_id: &str) {
        context::set_baggage_item("rhelma.tenant", tenant_id);
        self.record("rhelma_tenant_id", field::display(tenant_id));
    }

    fn record_user_id(&self, user_id: &str) {
        self.record("rhelma_user_id", field::display(user_id));
    }

    fn record_subject_id(&self, subject_id: &str) {
        context::set_baggage_item("rhelma.subject", subject_id);
        self.record("rhelma_subject_id", field::display(subject_id));
    }

    fn record_value_amount(&self, amount: i64) {
        context::set_baggage_item("rhelma.value.amount", &amount.to_string());
        self.record("rhelma_value_amount", field::display(amount));
    }
}

// Re-export macros for use as `rhelma_tracing::business::business_span!`
pub use business_span;
pub use business_span_with_level;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn business_span_macro_compiles() {
        // Note: span name MUST be a string literal
        let span = business_span!("test_operation", "test_op");
        let _guard = span.enter();
        span.record_tenant_id("test-tenant");
        span.record_user_id("user-123");
        span.record_subject_id("subject-456");
        span.record_value_amount(1000);
    }

    #[test]
    fn business_span_with_level_macro_compiles() {
        let span = business_span_with_level!(tracing::Level::DEBUG, "debug_operation", "debug_op");
        let _guard = span.enter();
    }

    #[test]
    fn can_use_variable_for_operation() {
        let op = "dynamic_operation".to_string();
        // Name is literal, but operation can be dynamic
        let span = business_span!("my_span", &op);
        let _guard = span.enter();
    }
}
