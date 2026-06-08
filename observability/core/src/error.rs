use thiserror::Error;

/// Error type for the observability core.
///
/// Notes:
/// - Logger init is always fatal.
/// - Tracing is best-effort by default, but can become fatal when required by policy.
#[derive(Debug, Error)]
pub enum ObservabilityError {
    /// Logger initialization failed (service MUST NOT continue).
    #[error("logger init failed: {0}")]
    /// Variant `Logger`.
    Logger(#[from] rhelma_logger::LoggerError),

    /// Tracing initialization failed (fatal only when required by policy).
    #[error("tracing init failed: {0}")]
    /// Variant `Tracing`.
    Tracing(#[from] rhelma_tracing::TracingConfigError),
}

/// Convenience result type.
pub type ObsResult<T> = Result<T, ObservabilityError>;
