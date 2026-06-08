//! High-level configuration loader for Rhelma services.

use serde::de::DeserializeOwned;

use crate::errors::ConfigResult;
use crate::validation::validate_all;
use crate::{CentralEnv, CoreConfig, UnifiedObservabilityConfig};

/// Grouped configuration for a service.
#[derive(Debug)]
pub struct GenericServiceConfig<T> {
    /// Field `central`.
    pub central: CentralEnv,
    /// Field `core`.
    pub core: CoreConfig,
    /// Field `service`.
    pub service: T,
}

/// Load service configuration using the RHELMA_* env model and a typed prefix.
pub fn load_with_prefix<T>(prefix: &str) -> ConfigResult<GenericServiceConfig<T>>
where
    T: Default + DeserializeOwned,
{
    let central = CentralEnv::from_env();
    let core = CoreConfig::from_env(&central)?;

    let mut builder = config::Config::builder();
    builder = builder.add_source(config::Environment::with_prefix(prefix).separator("__"));

    let service: T = match builder.build() {
        Ok(cfg) if is_config_effectively_empty(&cfg) => T::default(),
        Ok(cfg) => cfg.try_deserialize().map_err(|e| {
            crate::errors::ConfigError::Parse(format!("failed to deserialize {prefix} config: {e}"))
        })?,
        Err(e) if is_empty_config_error(&e) => T::default(),
        Err(e) => return Err(e.into()),
    };

    Ok(GenericServiceConfig {
        central,
        core,
        service,
    })
}

/// Load service configuration using the RHELMA_* env model and a typed prefix (fail-closed on deserialisation).
///
/// Use this in production services if you want config typos to surface immediately.
pub fn load_with_prefix_required<T>(prefix: &str) -> ConfigResult<GenericServiceConfig<T>>
where
    T: DeserializeOwned,
{
    let central = CentralEnv::from_env();
    let core = CoreConfig::from_env(&central)?;

    let cfg = config::Config::builder()
        .add_source(config::Environment::with_prefix(prefix).separator("__"))
        .build()?;

    let service: T = cfg.try_deserialize()?;

    Ok(GenericServiceConfig {
        central,
        core,
        service,
    })
}

/// Load service configuration using the RHELMA_* env model and a typed prefix (strict CentralEnv).
///
/// This is the recommended entrypoint for production services.
pub fn load_with_prefix_strict<T>(prefix: &str) -> ConfigResult<GenericServiceConfig<T>>
where
    T: Default + DeserializeOwned,
{
    let central = CentralEnv::from_env_strict()?;
    let core = CoreConfig::from_env(&central)?;

    let mut builder = config::Config::builder();
    builder = builder.add_source(config::Environment::with_prefix(prefix).separator("__"));

    let service: T = match builder.build() {
        Ok(cfg) if is_config_effectively_empty(&cfg) => T::default(),
        Ok(cfg) => cfg.try_deserialize().map_err(|e| {
            crate::errors::ConfigError::Parse(format!("failed to deserialize {prefix} config: {e}"))
        })?,
        Err(e) if is_empty_config_error(&e) => T::default(),
        Err(e) => return Err(e.into()),
    };

    Ok(GenericServiceConfig {
        central,
        core,
        service,
    })
}

fn is_empty_config_error(e: &config::ConfigError) -> bool {
    // `config` historically reports an empty builder as "configuration is empty".
    // We only default when the configuration is actually absent. Any other error
    // should surface.
    let msg = e.to_string().to_lowercase();
    msg.contains("configuration is empty")
        || msg.contains("config is empty")
        || msg.contains("empty configuration")
}

fn is_config_effectively_empty(cfg: &config::Config) -> bool {
    // `config` may successfully build an empty configuration (especially when only
    // environment sources are used). In that case, deserialising into a typed struct
    // with required fields will fail. We treat "no keys at all" as "absent".
    //
    // Micro-note: this does a `clone()` + deserialize into `serde_json::Value`, which
    // is slightly more work than inspecting an internal map. The `config` crate does
    // not expose a stable "is empty" API, and this happens only during startup, so
    // we prefer correctness and stable behaviour.
    match cfg.clone().try_deserialize::<serde_json::Value>() {
        Ok(serde_json::Value::Null) => true,
        Ok(serde_json::Value::Object(map)) => map.is_empty(),
        Ok(_) => false,
        Err(_) => false,
    }
}

/// Load service configuration using the RHELMA_* env model and a typed prefix (strict CentralEnv, fail-closed on deserialisation).
pub fn load_with_prefix_strict_required<T>(prefix: &str) -> ConfigResult<GenericServiceConfig<T>>
where
    T: DeserializeOwned,
{
    let central = CentralEnv::from_env_strict()?;
    let core = CoreConfig::from_env(&central)?;

    let cfg = config::Config::builder()
        .add_source(config::Environment::with_prefix(prefix).separator("__"))
        .build()?;

    let service: T = cfg.try_deserialize()?;

    Ok(GenericServiceConfig {
        central,
        core,
        service,
    })
}

/// Load + validate configuration in a single, contract-compliant call.
pub fn load_and_validate_with_prefix<T>(
    prefix: &str,
    service_name: &str,
) -> ConfigResult<GenericServiceConfig<T>>
where
    T: Default + DeserializeOwned,
{
    let cfg = load_with_prefix::<T>(prefix)?;
    let unified = UnifiedObservabilityConfig::from_central_env(&cfg.central, service_name);
    validate_all(&unified, &cfg.central)?;
    Ok(cfg)
}

/// Load + validate configuration in a single, contract-compliant call (strict CentralEnv).
pub fn load_and_validate_with_prefix_strict<T>(
    prefix: &str,
    service_name: &str,
) -> ConfigResult<GenericServiceConfig<T>>
where
    T: Default + DeserializeOwned,
{
    let cfg = load_with_prefix_strict::<T>(prefix)?;
    let unified = UnifiedObservabilityConfig::from_central_env(&cfg.central, service_name);
    validate_all(&unified, &cfg.central)?;
    Ok(cfg)
}

/// Load + validate configuration in a single, contract-compliant call (strict CentralEnv),
/// reading `RHELMA_SERVICE_NAME` automatically from the environment.
///
/// This reduces boilerplate in services that already export `RHELMA_SERVICE_NAME` at runtime.
pub fn load_and_validate_with_prefix_strict_auto<T>(
    prefix: &str,
) -> ConfigResult<GenericServiceConfig<T>>
where
    T: DeserializeOwned,
{
    let service_name = std::env::var("RHELMA_SERVICE_NAME")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown-service".to_string());

    let cfg = load_with_prefix_strict_required::<T>(prefix)?;
    let unified = UnifiedObservabilityConfig::from_central_env(&cfg.central, &service_name);
    validate_all(&unified, &cfg.central)?;
    Ok(cfg)
}
