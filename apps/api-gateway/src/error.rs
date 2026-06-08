#![forbid(unsafe_code)]

use anyhow::{anyhow, Error};
use axum::http::{HeaderName, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use rhelma_core::RhelmaError;

/// Canonical error type header used internally by api-gateway middleware.
///
/// Value is a stable type label such as: `bad_request`, `auth`, `rate_limited`, `dependency`, ...
pub const X_MACH_ERROR_TYPE: HeaderName = HeaderName::from_static("x-rhelma-error-type");

pub type ApiResult<T> = Result<T, GatewayError>;

/// Gateway error wrapper (contract-aligned with rhelma-core).
///
/// - Zero-trust: response bodies are replaced by v5.2 envelope middleware.
/// - Stable: adds `x-rhelma-error-type` for downstream envelope mapping.
#[derive(Debug)]
pub struct GatewayError(pub RhelmaError);

/// Backwards-compatible alias used across handlers.
pub type ApiError = GatewayError;

impl From<RhelmaError> for GatewayError {
    fn from(err: RhelmaError) -> Self {
        Self(err)
    }
}

impl From<sqlx::Error> for GatewayError {
    fn from(err: sqlx::Error) -> Self {
        Self(RhelmaError::Other(Error::new(err)))
    }
}

impl GatewayError {
    pub fn into_mach(self) -> RhelmaError {
        self.0
    }

    // -----------------------------
    // Constructors (typed)
    // -----------------------------
    pub fn bad_request(msg: impl Into<String>) -> Self {
        Self(RhelmaError::BadRequest(msg.into()))
    }

    pub fn validation(msg: impl Into<String>) -> Self {
        Self(RhelmaError::Validation(msg.into()))
    }

    pub fn unauthorized(msg: impl Into<String>) -> Self {
        Self(RhelmaError::Auth(msg.into()))
    }

    pub fn forbidden(msg: impl Into<String>) -> Self {
        Self(RhelmaError::Authz(msg.into()))
    }

    pub fn conflict(msg: impl Into<String>) -> Self {
        Self(RhelmaError::Conflict(msg.into()))
    }

    pub fn rate_limited(msg: impl Into<String>) -> Self {
        Self(RhelmaError::RateLimited(msg.into()))
    }

    /// Upstream dependency failed (search-service, auth-service, redis, etc).
    pub fn bad_gateway(msg: impl Into<String>) -> Self {
        Self(RhelmaError::Dependency(msg.into()))
    }

    pub fn service_unavailable(msg: impl Into<String>) -> Self {
        Self(RhelmaError::Dependency(msg.into()))
    }

    /// Internal error without leaking sensitive details.
    pub fn internal(msg: impl Into<String>) -> Self {
        // Keep response stable (500), but preserve the message for server-side logs.
        Self(RhelmaError::Other(anyhow!(msg.into())))
    }
}

impl IntoResponse for GatewayError {
    fn into_response(self) -> Response {
        let err = self.0;

        let p = err.to_problem();

        // status از problem details میاد
        let status = StatusCode::from_u16(p.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

        // اگر جایی برای x-rhelma-error-type می‌خوای، بهترین معادل همین code هست
        // (مثل BAD_REQUEST / TIMEOUT / RATE_LIMITED و ...)
        let label = p.code.to_string();

        // Zero-trust: never leak internal strings to the client.
        // But we DO log them on the server side.
        if status.is_server_error() {
            tracing::error!(
                rhelma_error_label = %label,
                rhelma_error = %err,
                "api-gateway error"
            );
        } else {
            tracing::info!(
                rhelma_error_label = %label,
                rhelma_error = %err,
                "api-gateway client error"
            );
        }

        // Minimal body; the `error_envelope_middleware` will replace it with the v5.2 envelope.
        let mut resp = (status, "").into_response();

        // Auth responses should be cache-resistant and include the standard challenge header.
        // (RFC 6750 / OAuth2 Bearer token usage)
        if status == StatusCode::UNAUTHORIZED {
            // Avoid leaking any internal details in the challenge.
            resp.headers_mut().insert(
                axum::http::header::WWW_AUTHENTICATE,
                HeaderValue::from_static("Bearer"),
            );
            resp.headers_mut().insert(
                axum::http::header::CACHE_CONTROL,
                HeaderValue::from_static("no-store"),
            );
            resp.headers_mut().insert(
                axum::http::header::PRAGMA,
                HeaderValue::from_static("no-cache"),
            );
        }

        // Attach the canonical error type label for typed envelope mapping.
        if let Ok(hv) = HeaderValue::from_str(&label) {
            resp.headers_mut().insert(X_MACH_ERROR_TYPE, hv);
        }

        resp
    }
}
