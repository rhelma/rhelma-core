//! Rhelma Contract v5.2 error envelope (canonical JSON shape).
//!
//! This is intentionally **additive** to the existing v5.1 RFC7807 `ProblemDetails` model.
//! Gateway / edge should expose this envelope to external callers.

#![forbid(unsafe_code)]

use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::{RequestContext, RhelmaError};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum ErrorSeverity {
    // Keep old API variants, but serialize to v5.2 wire values.
    #[serde(rename = "LOW", alias = "INFO", alias = "Info")]
    /// Variant `Info`.
    Info,

    #[serde(rename = "MEDIUM", alias = "WARNING", alias = "Warning")]
    /// Variant `Warning`.
    Warning,

    #[serde(rename = "HIGH", alias = "ERROR", alias = "Error")]
    #[default]
    /// Variant `Error`.
    Error,

    #[serde(rename = "CRITICAL", alias = "Critical")]
    /// Variant `Critical`.
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceErrorSeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl ServiceErrorSeverity {
    fn into_error_severity(self) -> ErrorSeverity {
        match self {
            ServiceErrorSeverity::Low => ErrorSeverity::Info,
            ServiceErrorSeverity::Medium => ErrorSeverity::Warning,
            ServiceErrorSeverity::High => ErrorSeverity::Error,
            ServiceErrorSeverity::Critical => ErrorSeverity::Critical,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ErrorEnvelopeV52 {
    /// Field `error`.
    pub error: ErrorV52,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ErrorV52 {
    /// Field `error_code`.
    pub error_code: String,
    /// Field `http_status`.
    pub http_status: u16,
    /// Field `message`.
    pub message: String,

    /// Field `retryable`.
    pub retryable: bool,

    #[serde(default)]
    /// Field `severity`.
    pub severity: ErrorSeverity,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Field `retry_after_ms`.
    pub retry_after_ms: Option<u64>,

    /// Field `request_id`.
    pub request_id: String,
    /// Field `correlation_id`.
    pub correlation_id: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Field `traceparent`.
    pub traceparent: Option<String>,

    // v5.2 wire name: "context"
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "context", alias = "details")]
    /// Field `details`.
    pub details: Option<Value>,

    /// Field `timestamp`.
    pub timestamp: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Field `stack_trace`.
    pub stack_trace: Option<String>,
}

impl ErrorEnvelopeV52 {
    pub fn new(error: ErrorV52) -> Self {
        Self { error }
    }
}

impl ErrorV52 {
    pub fn now_timestamp() -> String {
        Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
    }

    /// Keep older call-site ergonomics:
    /// if timestamp is empty, fill with "now".
    pub fn with_now_timestamp(mut self) -> Self {
        if self.timestamp.trim().is_empty() {
            self.timestamp = Self::now_timestamp();
        }
        self
    }
}

pub fn envelope_from_rhelma_error(
    err: &RhelmaError,
    ctx: Option<&RequestContext>,
) -> ErrorEnvelopeV52 {
    let (request_id, correlation_id, traceparent) = ctx_to_ids(ctx);

    let p = err.to_problem();
    let http_status = p.status;

    let (severity, retryable, retry_after_ms) = classify(http_status, err);

    let details = p.detail.as_ref().map(|s| Value::String(s.clone()));

    ErrorEnvelopeV52::new(
        ErrorV52 {
            error_code: p.code.to_string(),
            http_status,
            message: err.to_string(),
            retryable,
            severity,
            retry_after_ms,
            request_id,
            correlation_id,
            traceparent,
            details,
            timestamp: ErrorV52::now_timestamp(),
            stack_trace: None,
        }
        .with_now_timestamp(),
    )
}

pub fn envelope_from_status(
    http_status: u16,
    message: impl Into<String>,
    ctx: Option<&RequestContext>,
) -> ErrorEnvelopeV52 {
    let (request_id, correlation_id, traceparent) = ctx_to_ids(ctx);
    let message = message.into();

    let error_code = status_to_code(http_status).to_string();
    let (severity, retryable, retry_after_ms) = classify(http_status, &RhelmaError::Internal);

    ErrorEnvelopeV52::new(
        ErrorV52 {
            error_code,
            http_status,
            message,
            retryable,
            severity,
            retry_after_ms,
            request_id,
            correlation_id,
            traceparent,
            details: None,
            timestamp: ErrorV52::now_timestamp(),
            stack_trace: None,
        }
        .with_now_timestamp(),
    )
}

#[allow(clippy::too_many_arguments)]
pub fn service_error_envelope(
    ctx: &RequestContext,
    http_status: u16,
    error_code: impl Into<String>,
    message: impl Into<String>,
    retryable: bool,
    severity: ServiceErrorSeverity,
    retry_after_ms: Option<u64>,
    context: Value,
) -> ErrorEnvelopeV52 {
    let (request_id, correlation_id, traceparent) = ctx_to_ids(Some(ctx));

    ErrorEnvelopeV52::new(
        ErrorV52 {
            error_code: error_code.into(),
            http_status,
            message: message.into(),
            retryable,
            severity: severity.into_error_severity(),
            retry_after_ms,
            request_id,
            correlation_id,
            traceparent,
            details: Some(context),
            timestamp: ErrorV52::now_timestamp(),
            stack_trace: None,
        }
        .with_now_timestamp(),
    )
}

fn ctx_to_ids(ctx: Option<&RequestContext>) -> (String, String, Option<String>) {
    match ctx {
        Some(c) => {
            let rid = c.request_id().to_string();
            let cid = c
                .correlation_id()
                .map(|s| s.to_string())
                .unwrap_or_else(|| rid.clone());
            let tp = c.trace().to_traceparent();
            (rid, cid, tp)
        }
        None => {
            let rid = Uuid::now_v7().to_string();
            let cid = Uuid::now_v7().to_string();
            (rid, cid, None)
        }
    }
}

fn status_to_code(status: u16) -> &'static str {
    match status {
        400 => "BAD_REQUEST",
        401 => "UNAUTHORIZED",
        403 => "FORBIDDEN",
        404 => "NOT_FOUND",
        409 => "CONFLICT",
        429 => "RATE_LIMITED",
        451 => "RESIDENCY_VIOLATION",
        503 => "SERVICE_UNAVAILABLE",
        504 => "TIMEOUT",
        500 => "INTERNAL_ERROR",
        _ => "HTTP_ERROR",
    }
}

fn classify(status: u16, err: &RhelmaError) -> (ErrorSeverity, bool, Option<u64>) {
    let retryable = matches!(status, 429 | 503 | 504)
        || matches!(
            err,
            RhelmaError::RateLimited(_) | RhelmaError::Dependency(_) | RhelmaError::CircuitOpen(_)
        );

    let severity = match status {
        451 => ErrorSeverity::Critical,
        500..=599 => ErrorSeverity::Error,
        400..=499 => ErrorSeverity::Warning,
        _ => ErrorSeverity::Info,
    };

    (severity, retryable, None)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn service_error_envelope_uses_v52_wire_values() {
        let ctx = RequestContext::empty();

        let envelope = service_error_envelope(
            &ctx,
            403,
            "FORBIDDEN",
            "tenant mismatch",
            false,
            ServiceErrorSeverity::High,
            None,
            json!({"tenant": "tenant-b"}),
        );

        let value = serde_json::to_value(envelope).expect("serialize envelope");
        assert_eq!(value["error"]["error_code"], "FORBIDDEN");
        assert_eq!(value["error"]["http_status"], 403);
        assert_eq!(value["error"]["severity"], "HIGH");
        assert_eq!(value["error"]["retryable"], false);
        assert_eq!(value["error"]["context"]["tenant"], "tenant-b");
        assert_eq!(value["error"]["request_id"], ctx.request_id().to_string());
        assert_eq!(
            value["error"]["correlation_id"],
            ctx.request_id().to_string()
        );
    }
}
