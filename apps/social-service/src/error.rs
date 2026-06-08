#![forbid(unsafe_code)]

use axum::{
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use chrono::Utc;
use rhelma_core::RequestContext;
use serde::Serialize;
use serde_json::{json, Value};

pub type ApiResult<T> = Result<T, RhelmaErrorResponse>;

#[derive(Debug, Clone, Copy)]
pub enum Severity {
    Low,
    Medium,
    High,
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
    pub status: StatusCode,
    pub body: ErrorEnvelope,
}

impl IntoResponse for RhelmaErrorResponse {
    fn into_response(self) -> Response {
        (self.status, Json(self.body)).into_response()
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct ErrorEnvelope {
    pub error: ErrorV52,
}

#[derive(Debug, Serialize, Clone)]
pub struct ErrorV52 {
    pub error_code: String,
    pub http_status: u16,
    pub message: String,

    pub retryable: bool,
    pub severity: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after_ms: Option<u64>,

    pub context: Value,
    pub request_id: String,
    pub correlation_id: String,

    pub timestamp: String,

    #[serde(skip_serializing_if = "Option::is_none")]
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

pub fn bad_request(ctx: &RequestContext, message: &str, context: Value) -> RhelmaErrorResponse {
    v52_error(
        ctx,
        StatusCode::BAD_REQUEST,
        "BAD_REQUEST",
        message,
        false,
        Severity::Low,
        None,
        context,
    )
}

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

pub fn not_found(ctx: &RequestContext, message: &str) -> RhelmaErrorResponse {
    v52_error(
        ctx,
        StatusCode::NOT_FOUND,
        "NOT_FOUND",
        message,
        false,
        Severity::Low,
        None,
        json!({}),
    )
}

pub fn internal(ctx: &RequestContext, message: &str) -> RhelmaErrorResponse {
    v52_error(
        ctx,
        StatusCode::INTERNAL_SERVER_ERROR,
        "INTERNAL",
        message,
        true,
        Severity::Critical,
        None,
        json!({}),
    )
}
