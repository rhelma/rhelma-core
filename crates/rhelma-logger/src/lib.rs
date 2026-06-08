#![forbid(unsafe_code)]

//! rhelma-logger v0.17.0

pub mod builder;
pub mod config;
pub mod dispatchers;
pub mod event;
pub mod extensions;
pub mod performance;
pub mod pii;
pub mod state;

mod event_time_ms;

// -------------------------
// Re-exports for macro API
// -------------------------
pub use crate::builder::LogBuilder;
pub use crate::event::{LogEvent, LogLevel};

pub use crate::config::{
    BackpressureStrategy, DispatchMode, Environment, LogFormat, LoggerConfig, LoggerError,
    PerformanceProfile,
};

pub use crate::extensions::{DispatchError, LogDispatcher};
pub use crate::pii::{DefaultPiiRedactor, PiiRedactor};
pub use crate::state::{
    set_dispatcher, set_internal_error_handler, set_pii_violation_handler, set_redactor,
};

// -------------------------
// Macros (MUST be after exports!)
// -------------------------

#[macro_export]
macro_rules! log_info {
    ($msg:expr $(, $k:expr => $v:expr )* $(,)?) => {{
        let mut b = $crate::LogBuilder::new($crate::LogLevel::Info, $msg);
        $( b = b.field($k, $v); )*
        b.emit();
    }};
}

#[macro_export]
macro_rules! log_debug {
    ($msg:expr $(, $k:expr => $v:expr )* $(,)?) => {{
        let mut b = $crate::LogBuilder::new($crate::LogLevel::Debug, $msg);
        $( b = b.field($k, $v); )*
        b.emit();
    }};
}

#[macro_export]
macro_rules! log_warn {
    ($msg:expr $(, $k:expr => $v:expr )* $(,)?) => {{
        let mut b = $crate::LogBuilder::new($crate::LogLevel::Warn, $msg);
        $( b = b.field($k, $v); )*
        b.emit();
    }};
}

#[macro_export]
macro_rules! log_error {
    ($msg:expr $(, $k:expr => $v:expr )* $(,)?) => {{
        let mut b = $crate::LogBuilder::new($crate::LogLevel::Error, $msg);
        $( b = b.field($k, $v); )*
        b.emit();
    }};
}

#[macro_export]
macro_rules! log_audit {
    ($msg:expr, $actor_type:expr, $operation:expr, $resource_type:expr, $resource_id:expr $(, $k:expr => $v:expr )* $(,)?) => {{
        let mut b = $crate::LogBuilder::new($crate::LogLevel::Info, $msg)
            .audit($actor_type, $operation, $resource_type, $resource_id)
            .tag("audit");
        $( b = b.field($k, $v); )*
        b.emit();
    }};
}

#[macro_export]
macro_rules! log_heartbeat {
    ($msg:expr $(, $k:expr => $v:expr )* $(,)?) => {{
        let mut b = $crate::LogBuilder::new($crate::LogLevel::Info, $msg)
            .heartbeat()
            .tag("heartbeat");
        $( b = b.field($k, $v); )*
        b.emit();
    }};
}

#[macro_export]
macro_rules! log_trace {
    ($msg:expr $(, $k:expr => $v:expr )* $(,)?) => {{
        let mut b = $crate::LogBuilder::new($crate::LogLevel::Trace, $msg);
        $( b = b.field($k, $v); )*
        b.emit();
    }};
}

#[macro_export]
macro_rules! log_critical {
    ($msg:expr $(, $k:expr => $v:expr )* $(,)?) => {{
        let mut b = $crate::LogBuilder::new($crate::LogLevel::Critical, $msg);
        $( b = b.field($k, $v); )*
        b.emit();
    }};
}

// -------------------------
// RhelmaLogger facade (API compatibility with rhelma-observability)
// -------------------------
pub struct RhelmaLogger;

impl RhelmaLogger {
    /// Initialize rhelma-logger using an explicit LoggerConfig.
    #[deprecated(note = "init_with_config is deprecated; prefer init_from_unified")]
    pub fn init_with_config(cfg: &LoggerConfig) -> Result<(), LoggerError> {
        crate::state::install_globals(cfg)
    }

    /// Initialize rhelma-logger from the unified observability config (rhelma-config v5.2).
    #[cfg(feature = "with-config")]
    pub fn init_from_unified(
        unified: &rhelma_config::UnifiedObservabilityConfig,
        core: Option<&rhelma_config::CoreConfig>,
    ) -> Result<(), LoggerError> {
        let cfg = LoggerConfig::from_unified(unified, core);
        crate::state::install_globals(&cfg)
    }

    /// Flush async queue (if enabled) and stop the worker.
    pub fn flush_and_shutdown() {
        crate::state::flush_and_shutdown();
    }

    /// Read a debug snapshot of current global logger state.
    pub fn snapshot() -> crate::state::LoggerStateSnapshot {
        crate::state::get_state_snapshot()
    }
}
