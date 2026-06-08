#![forbid(unsafe_code)]

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use rhelma_core::RhelmaError;
use serde::Serialize;
use thiserror::Error;
use tracing::error;

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("config error: {0}")]
    /// Variant `Config`.
    Config(String),

    #[error("bad request: {0}")]
    /// Variant `BadRequest`.
    BadRequest(String),

    #[error("conflict: {0}")]
    /// Variant `Conflict`.
    Conflict(String),

    #[error("internal error: {0}")]
    /// Variant `Internal`.
    Internal(String),

    #[error("http {0}: {1}")]
    /// Variant `Http`.
    Http(StatusCode, String),
}

impl RegistryError {
    pub fn too_many_requests(msg: &str) -> Self {
        Self::Http(StatusCode::TOO_MANY_REQUESTS, msg.to_string())
    }

    pub fn not_found(msg: &str) -> Self {
        Self::Http(StatusCode::NOT_FOUND, msg.to_string())
    }

    pub fn unauthorized(msg: impl Into<String>) -> Self {
        Self::Http(StatusCode::UNAUTHORIZED, msg.into())
    }

    pub fn config(msg: impl Into<String>) -> Self {
        Self::Config(msg.into())
    }
    pub fn bad_request(msg: impl Into<String>) -> Self {
        Self::BadRequest(msg.into())
    }
    pub fn conflict(msg: impl Into<String>) -> Self {
        Self::Conflict(msg.into())
    }
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal(msg.into())
    }
}

impl From<RhelmaError> for RegistryError {
    fn from(e: RhelmaError) -> Self {
        RegistryError::Config(e.to_string())
    }
}

#[derive(Debug, Serialize)]
pub struct ApiErrorBody {
    /// Field `code`.
    pub code: String,
    /// Field `message`.
    pub message: String,
}

impl IntoResponse for RegistryError {
    fn into_response(self) -> Response {
        let (status, code) = match &self {
            RegistryError::Config(_) => (StatusCode::INTERNAL_SERVER_ERROR, "CONFIG_ERROR"),
            RegistryError::BadRequest(_) => (StatusCode::BAD_REQUEST, "BAD_REQUEST"),
            RegistryError::Conflict(_) => (StatusCode::CONFLICT, "CONFLICT"),
            RegistryError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL"),
            RegistryError::Http(status, _) => (*status, "HTTP"),
        };

        if status.is_server_error() {
            error!("{self}");
        }

        let body = ApiErrorBody {
            code: code.to_string(),
            message: self.to_string(),
        };

        (status, Json(body)).into_response()
    }
}
