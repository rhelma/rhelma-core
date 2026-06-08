//! High-level configuration resolver.

use crate::errors::{ConfigError, ConfigResult};
use crate::merge::deep_merge;
use crate::provider::{AsyncConfigProvider, SyncConfigProvider};
use crate::sources::load_env_overrides;
use crate::{CentralEnv, UnifiedObservabilityConfig};

/// Config resolver entry point.
pub struct ConfigResolver;

impl ConfigResolver {
    /// Resolve a unified observability configuration for a given service
    /// using an async provider.
    pub async fn resolve_async<P>(
        service: &str,
        central: &CentralEnv,
        provider: &P,
    ) -> ConfigResult<UnifiedObservabilityConfig>
    where
        P: AsyncConfigProvider,
    {
        let cfg = UnifiedObservabilityConfig::from_central_env(central, service);

        let mut value = serde_json::to_value(&cfg)
            .map_err(|e| ConfigError::Parse(format!("failed to serialise: {e}")))?;

        if let Some(defaults) = provider.load_defaults().await? {
            value = deep_merge(value, defaults);
        }
        if let Some(region_cfg) = provider.load_region_config(&central.region).await? {
            value = deep_merge(value, region_cfg);
        }
        if let Some(svc_cfg) = provider
            .load_service_config(&central.region, service)
            .await?
        {
            value = deep_merge(value, svc_cfg);
        }

        let env_overrides = load_env_overrides()?;
        value = deep_merge(value, env_overrides);

        let final_cfg: UnifiedObservabilityConfig = serde_json::from_value(value)
            .map_err(|e| ConfigError::Parse(format!("failed to deserialize: {e}")))?;

        Ok(final_cfg)
    }

    /// Resolve configuration using a synchronous provider.
    pub fn resolve_sync<P>(
        service: &str,
        central: &CentralEnv,
        provider: &P,
    ) -> ConfigResult<UnifiedObservabilityConfig>
    where
        P: SyncConfigProvider,
    {
        let cfg = UnifiedObservabilityConfig::from_central_env(central, service);

        let mut value = serde_json::to_value(&cfg)
            .map_err(|e| ConfigError::Parse(format!("failed to serialise: {e}")))?;

        if let Some(defaults) = provider.load_defaults()? {
            value = deep_merge(value, defaults);
        }
        if let Some(region_cfg) = provider.load_region_config(&central.region)? {
            value = deep_merge(value, region_cfg);
        }
        if let Some(svc_cfg) = provider.load_service_config(&central.region, service)? {
            value = deep_merge(value, svc_cfg);
        }

        let env_overrides = load_env_overrides()?;
        value = deep_merge(value, env_overrides);

        let final_cfg: UnifiedObservabilityConfig = serde_json::from_value(value)
            .map_err(|e| ConfigError::Parse(format!("failed to deserialize: {e}")))?;

        Ok(final_cfg)
    }
}
