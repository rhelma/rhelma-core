#![forbid(unsafe_code)]

use axum::{
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use chrono::Utc;
use rhelma_core::prelude::RequestContext;
use serde::Serialize;
use serde_json::{json, Value};

#[derive(Debug, Clone, Copy)]
pub enum Severity {
    /// Variant `Low`.
    Low,
    /// Variant `Medium`.
    Medium,
    /// Variant `High`.
    High,
    /// Variant `Critical`.
    Critical,
}

impl Severity {
    fn as_str(self) -> &'static str {
        match self {
            Severity::Low => "LOW",
            Severity::Medium => "MEDIUM",
            Severity::High => "HIGH",
            Severity::Critical => "CRITICAL",
        }
    }
}

#[derive(Debug, Clone)]
pub struct RhelmaErrorResponse {
    /// Field `status`.
    pub status: StatusCode,
    /// Field `body`.
    pub body: ErrorEnvelope,
}

impl IntoResponse for RhelmaErrorResponse {
    fn into_response(self) -> Response {
        (self.status, Json(self.body)).into_response()
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct ErrorEnvelope {
    /// Field `error`.
    pub error: ErrorV52,
}

#[derive(Debug, Serialize, Clone)]
pub struct ErrorV52 {
    /// Field `error_code`.
    pub error_code: String,
    /// Field `http_status`.
    pub http_status: u16,
    /// Field `message`.
    pub message: String,

    /// Field `retryable`.
    pub retryable: bool,
    /// Field `severity`.
    pub severity: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Field `retry_after_ms`.
    pub retry_after_ms: Option<u64>,

    /// Field `context`.
    pub context: Value,
    /// Field `request_id`.
    pub request_id: String,
    /// Field `correlation_id`.
    pub correlation_id: String,

    /// Field `timestamp`.
    pub timestamp: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Field `stack_trace`.
    pub stack_trace: Option<String>,
}

#[allow(clippy::too_many_arguments)]
pub fn v52_error(
    ctx: &RequestContext,
    status: StatusCode,
    error_code: &str,
    message: impl Into<String>,
    retryable: bool,
    severity: Severity,
    retry_after_ms: Option<u64>,
    context: Value,
) -> RhelmaErrorResponse {
    let request_id = ctx.request_id().to_string();
    let correlation_id = ctx
        .correlation_id()
        .map(|s| s.to_string())
        .unwrap_or_else(|| request_id.clone());

    RhelmaErrorResponse {
        status,
        body: ErrorEnvelope {
            error: ErrorV52 {
                error_code: error_code.to_string(),
                http_status: status.as_u16(),
                message: message.into(),
                retryable,
                severity: severity.as_str().to_string(),
                retry_after_ms,
                context,
                request_id,
                correlation_id,
                timestamp: Utc::now().to_rfc3339(),
                stack_trace: None,
            },
        },
    }
}

/// Convenience helpers
pub fn unauthorized(ctx: &RequestContext, message: &str) -> RhelmaErrorResponse {
    v52_error(
        ctx,
        StatusCode::UNAUTHORIZED,
        "UNAUTHORIZED",
        message,
        false,
        Severity::High,
        None,
        json!({}),
    )
}

pub fn forbidden(ctx: &RequestContext, message: &str) -> RhelmaErrorResponse {
    v52_error(
        ctx,
        StatusCode::FORBIDDEN,
        "FORBIDDEN",
        message,
        false,
        Severity::High,
        None,
        json!({}),
    )
}

pub fn validation_error(
    ctx: &RequestContext,
    message: &str,
    context: Value,
) -> RhelmaErrorResponse {
    v52_error(
        ctx,
        StatusCode::BAD_REQUEST,
        "VALIDATION_ERROR",
        message,
        false,
        Severity::Low,
        None,
        context,
    )
}

pub fn rate_limit(ctx: &RequestContext, retry_after_ms: u64) -> RhelmaErrorResponse {
    v52_error(
        ctx,
        StatusCode::TOO_MANY_REQUESTS,
        "RATE_LIMIT",
        "rate limit exceeded",
        true,
        Severity::Medium,
        Some(retry_after_ms),
        json!({ "retry_after_ms": retry_after_ms }),
    )
}
