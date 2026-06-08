#![forbid(unsafe_code)]

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use rhelma_core::prelude::*;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FileStorageError {
    #[error("database error: {0}")]
    /// Variant `Database`.
    Database(String),

    #[error("storage backend error: {0}")]
    /// Variant `Storage`.
    Storage(String),

    #[error("file too large: {0} bytes")]
    /// Variant `FileTooLarge`.
    FileTooLarge(u64),

    #[error("unsupported mime type: {0}")]
    /// Variant `UnsupportedMime`.
    UnsupportedMime(String),

    #[error("mime type not allowed by policy: {0}")]
    /// Variant `MimeNotAllowed`.
    MimeNotAllowed(String),

    #[error("antivirus scan failed: {0}")]
    /// Variant `Antivirus`.
    Antivirus(String),
}

pub type FileStorageResult<T> = Result<T, FileStorageError>;

impl From<sqlx::Error> for FileStorageError {
    fn from(e: sqlx::Error) -> Self {
        Self::Database(e.to_string())
    }
}

impl From<FileStorageError> for RhelmaError {
    fn from(e: FileStorageError) -> Self {
        match e {
            FileStorageError::FileTooLarge(_) => RhelmaError::BadRequest(e.to_string()),
            FileStorageError::UnsupportedMime(_) => RhelmaError::BadRequest(e.to_string()),
            FileStorageError::MimeNotAllowed(_) => RhelmaError::BadRequest(e.to_string()),
            FileStorageError::Antivirus(_) => RhelmaError::BadRequest(e.to_string()),
            FileStorageError::Database(_) => RhelmaError::Dependency(e.to_string()),
            FileStorageError::Storage(_) => RhelmaError::Dependency(e.to_string()),
        }
    }
}

#[derive(Debug)]
pub struct ApiError {
    /// Field `err`.
    pub err: RhelmaError,
    /// Field `ctx`.
    pub ctx: Option<RequestContext>,
}

impl ApiError {
    pub fn with_ctx(err: RhelmaError, ctx: &RequestContext) -> Self {
        Self {
            err,
            ctx: Some(ctx.clone()),
        }
    }

    pub fn without_ctx(err: RhelmaError) -> Self {
        Self { err, ctx: None }
    }
}

pub type ApiResult<T> = Result<T, ApiError>;

impl From<RhelmaError> for ApiError {
    fn from(err: RhelmaError) -> Self {
        Self::without_ctx(err)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status_u16 = self.err.to_problem().status;
        let status = StatusCode::from_u16(status_u16).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

        let envelope =
            rhelma_core::error_v52::envelope_from_rhelma_error(&self.err, self.ctx.as_ref());

        let mut resp = (status, Json(envelope)).into_response();
        resp.headers_mut().insert(
            "x-rhelma-error-envelope",
            axum::http::HeaderValue::from_static("v5.2"),
        );
        resp
    }
}
