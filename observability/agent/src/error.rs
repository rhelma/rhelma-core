//! AgentError — unified error type for the Rhelma Observability Agent (v5.2).
//!
//! This error model is intentionally minimal compared to `rhelma_core::RhelmaError`,
//! because the agent is a lightweight peripheral runtime, not a full Rhelma service.
//!
//! Supported domains:
//!   - Config validation errors
//!   - HTTP transport & status errors
//!   - JSON serialization/deserialization
//!   - URL parse error
//!   - EventBus publish/consume errors
//!   - Internal logic errors
//!
//! NOTE:
//!   - Keep this enum small and generic. Domain-specific details should be logged,
//!     not encoded as many fine-grained variants.

use rhelma_event::EventBusError;
use thiserror::Error;

/// Unified error type for the Rhelma Observability Agent
#[derive(Debug, Error)]
pub enum AgentError {
    /// Required configuration value is missing.
    #[error("missing required configuration field: {0}")]
    /// Variant `MissingField`.
    MissingField(String),

    /// Invalid configuration (constraints violated, malformed values, etc.).
    #[error("invalid configuration: {0}")]
    /// Variant `InvalidConfig`.
    InvalidConfig(String),

    /// Error performing HTTP request (connection failure, timeout, TLS error, etc.).
    #[error("http transport error: {0}")]
    /// Variant `HttpTransport`.
    HttpTransport(String),

    /// HTTP request succeeded but server returned a non-2xx response.
    #[error("http status error {code}: {message}")]
    /// Variant `HttpStatus`.
    HttpStatus {
        /// HTTP status code
        code: u16,
        /// Error message
        message: String,
    },

    /// JSON decoding failure (e.g. invalid payload from external system).
    #[error("json decode error: {0}")]
    /// Variant `JsonDecode`.
    JsonDecode(String),

    /// JSON serialization or generic serde error.
    #[error("serialization error: {0}")]
    /// Variant `Serialization`.
    Serialization(String),

    /// URL parsing error.
    #[error("url parse error: {0}")]
    /// Variant `Url`.
    Url(String),

    /// Failure inside EventBus backend.
    #[error("eventbus error: {0}")]
    /// Variant `EventBus`.
    EventBus(String),

    /// Internal logic error (should not happen in normal runtime).
    #[error("internal error: {0}")]
    /// Variant `Internal`.
    Internal(String),
}

// ------------------------------------------------------------
// Automatic conversions
// ------------------------------------------------------------

impl From<EventBusError> for AgentError {
    fn from(e: EventBusError) -> Self {
        AgentError::EventBus(e.to_string())
    }
}

impl From<serde_json::Error> for AgentError {
    fn from(e: serde_json::Error) -> Self {
        AgentError::Serialization(e.to_string())
    }
}

impl From<reqwest::Error> for AgentError {
    fn from(e: reqwest::Error) -> Self {
        AgentError::HttpTransport(e.to_string())
    }
}

impl From<url::ParseError> for AgentError {
    fn from(e: url::ParseError) -> Self {
        AgentError::Url(e.to_string())
    }
}

// ------------------------------------------------------------
// Helper constructors
// ------------------------------------------------------------

impl AgentError {
    /// Internal logic or unexpected state.
    ///
    /// # Arguments
    /// * `msg` - Error message
    ///
    /// # Returns
    /// AgentError instance
    pub fn internal<T: Into<String>>(msg: T) -> Self {
        AgentError::Internal(msg.into())
    }

    /// For user-facing or config-level validation errors.
    ///
    /// # Arguments
    /// * `msg` - Error message
    ///
    /// # Returns
    /// AgentError instance
    pub fn invalid<T: Into<String>>(msg: T) -> Self {
        AgentError::InvalidConfig(msg.into())
    }
}

/// Convenient alias for results that use AgentError.
pub type AgentResult<T> = Result<T, AgentError>;
