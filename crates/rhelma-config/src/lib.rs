//! rhelma-config v5.2.0
//!
//! Enterprise configuration and unified observability model for the Rhelma platform.

#![forbid(unsafe_code)]

pub mod builder;
pub mod central_env;
pub mod core_config;
pub mod deprecation;
pub mod errors;
pub mod governance;
pub mod loader;
pub mod merge;
pub mod models;
pub mod provider;
pub mod resolver;
pub mod runtime;
pub mod sources;
pub mod validation;

pub use crate::builder::ConfigBuilder;
pub use crate::central_env::{is_env_model_v1_enabled, CentralEnv, CentralEnvTyped};
pub use crate::core_config::{CoreConfig, FileBackend};
pub use crate::deprecation::{set_deprecation_handler, DeprecationHandler};
pub use crate::errors::{ConfigError, ConfigResult};
pub use crate::governance::GovernanceRuntimeConfig;
pub use crate::models::{
    Environment, LogFormat, LoggerConfig, MetricsConfig, PerformanceProfile, TracingConfig,
    UnifiedObservabilityConfig,
};
pub use crate::provider::{AsyncConfigProvider, CachedProvider, SyncConfigProvider};
pub use crate::resolver::ConfigResolver;
pub use crate::runtime::CentralRuntime;

/// Convenient prelude for downstream crates.
pub mod prelude {
    pub use crate::loader::{
        load_and_validate_with_prefix, load_and_validate_with_prefix_strict,
        load_and_validate_with_prefix_strict_auto, load_with_prefix, load_with_prefix_required,
        load_with_prefix_strict, load_with_prefix_strict_required, GenericServiceConfig,
    };
    pub use crate::merge::{deep_merge, flattened_to_nested, insert_nested};
    pub use crate::sources::MemoryConfig;
    pub use crate::validation::{validate_all, validate_base};
    pub use crate::{
        set_deprecation_handler, AsyncConfigProvider, CachedProvider, CentralEnv, ConfigBuilder,
        ConfigError, ConfigResolver, ConfigResult, CoreConfig, Environment, FileBackend,
        GovernanceRuntimeConfig, LoggerConfig, MetricsConfig, SyncConfigProvider, TracingConfig,
        UnifiedObservabilityConfig,
    };
}
