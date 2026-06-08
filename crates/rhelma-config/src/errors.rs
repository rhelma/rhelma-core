//! Error types for rhelma-config.

use thiserror::Error;

/// Result alias for config operations.
pub type ConfigResult<T> = Result<T, ConfigError>;

/// Error type for configuration and resolution.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// Errors originating from an external provider implementation.
    #[error("provider error: {0}")]
    /// Variant `Provider`.
    Provider(String),

    /// Errors originating from a config source (e.g. file, database).
    #[error("source error: {0}")]
    /// Variant `Source`.
    Source(String),

    /// Generic parse / deserialisation error.
    #[error("parse error: {0}")]
    /// Variant `Parse`.
    Parse(String),

    /// A required field was missing.
    #[error("missing required field `{0}`")]
    /// Variant `MissingField`.
    MissingField(&'static str),

    /// Invalid value for a particular field.
    #[error("invalid value for `{field}`: {message}")]
    /// Variant `InvalidValue`.
    InvalidValue {
        field: &'static str,
        message: String,
    },
}

impl From<serde_json::Error> for ConfigError {
    fn from(e: serde_json::Error) -> Self {
        ConfigError::Parse(e.to_string())
    }
}

impl From<config::ConfigError> for ConfigError {
    fn from(e: config::ConfigError) -> Self {
        ConfigError::Source(e.to_string())
    }
}
