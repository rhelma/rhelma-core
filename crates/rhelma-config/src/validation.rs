//! Validation helpers for rhelma-config.

use crate::errors::{ConfigError, ConfigResult};
use crate::{CentralEnv, UnifiedObservabilityConfig};

/// Basic always-on validation for unified observability config.
pub fn validate_base(cfg: &UnifiedObservabilityConfig) -> ConfigResult<()> {
    if cfg.service_name.trim().is_empty() {
        return Err(ConfigError::InvalidValue {
            field: "service_name",
            message: "service_name must not be empty".to_string(),
        });
    }

    if cfg.region.trim().is_empty() {
        return Err(ConfigError::InvalidValue {
            field: "region",
            message: "region must not be empty".to_string(),
        });
    }

    if !(0.0..=1.0).contains(&cfg.sampling_rate) {
        return Err(ConfigError::InvalidValue {
            field: "sampling_rate",
            message: "must be in [0.0, 1.0]".into(),
        });
    }

    // OTEL policy enforcement
    if cfg.otel_required {
        if !cfg.otel_enabled {
            return Err(ConfigError::InvalidValue {
                field: "otel_enabled",
                message: "otel_required=true implies otel_enabled must be true".into(),
            });
        }
        if cfg
            .otel_endpoint
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .is_none()
        {
            return Err(ConfigError::InvalidValue {
                field: "otel_endpoint",
                message: "OTEL is required but no otel_endpoint provided".into(),
            });
        }
    }

    Ok(())
}

#[cfg(feature = "strict-validation")]
use regex::Regex;
#[cfg(feature = "strict-validation")]
use rhelma_core::types::{RegionId, TenantId};
#[cfg(feature = "strict-validation")]
use semver::Version;
#[cfg(feature = "strict-validation")]
use url::Url;

#[cfg(feature = "strict-validation")]
fn validate_strict(cfg: &UnifiedObservabilityConfig, central: &CentralEnv) -> ConfigResult<()> {
    // service_name pattern
    let re_srv = Regex::new(r"^[a-z0-9][a-z0-9-]{1,63}$").unwrap();
    if !re_srv.is_match(&cfg.service_name) {
        return Err(ConfigError::InvalidValue {
            field: "service_name",
            message: "must match ^[a-z0-9][a-z0-9-]{1,63}$".into(),
        });
    }
    // region (rhelma-core Strong-ID)
    RegionId::parse(&cfg.region).map_err(|e| ConfigError::InvalidValue {
        field: "region",
        message: e.to_string(),
    })?;

    // sampling strict (already checked in base, but keep for defence-in-depth)
    if cfg.sampling_rate < 0.0 || cfg.sampling_rate > 1.0 {
        return Err(ConfigError::InvalidValue {
            field: "sampling_rate",
            message: "must be in [0.0, 1.0]".into(),
        });
    }

    // OTEL endpoint URL
    if let Some(ref url) = cfg.otel_endpoint {
        if Url::parse(url).is_err() {
            return Err(ConfigError::InvalidValue {
                field: "otel_endpoint",
                message: "must be a valid URL".into(),
            });
        }
    }

    // prometheus port range
    if !(1024..=65535).contains(&cfg.prometheus_port) {
        return Err(ConfigError::InvalidValue {
            field: "prometheus_port",
            message: "must be in 1024–65535".into(),
        });
    }
    // tenant id format (if present) (rhelma-core Strong-ID)
    if let Some(ref tenant) = central.tenant_id {
        TenantId::parse(tenant).map_err(|e| ConfigError::InvalidValue {
            field: "tenant_id",
            message: e.to_string(),
        })?;
    }

    // environment whitelist
    match central.environment.as_str() {
        "local" | "development" | "staging" | "production" | "test" => {}
        other => {
            return Err(ConfigError::InvalidValue {
                field: "environment",
                message: format!("unsupported RHELMA_ENV value: {other}"),
            });
        }
    }

    // service version semantics (semver)
    if let Err(e) = Version::parse(&central.service_version) {
        return Err(ConfigError::InvalidValue {
            field: "service_version",
            message: format!(
                "RHELMA_SERVICE_VERSION is not valid semver: {} ({})",
                central.service_version, e
            ),
        });
    }

    Ok(())
}

/// Combined validation entry point.
pub fn validate_all(cfg: &UnifiedObservabilityConfig, central: &CentralEnv) -> ConfigResult<()> {
    validate_base(cfg)?;

    #[cfg(not(feature = "strict-validation"))]
    let _ = central;

    #[cfg(feature = "strict-validation")]
    {
        validate_strict(cfg, central)?;
    }

    Ok(())
}

#[cfg(test)]
pub mod examples {

    #[cfg(feature = "strict-validation")]
    #[test]
    fn invalid_tenant_id_rejected() {
        let central = CentralEnv {
            region: "eu-west-1".into(),
            environment: "production".into(),
            service_version: "1.0.0".into(),
            tenant_id: Some("ACME-CORP".into()),
        };

        let cfg = UnifiedObservabilityConfig::baseline("svc".into());

        let res = crate::validation::validate_strict(&cfg, &central);
        assert!(res.is_err());
    }

    #[cfg(feature = "strict-validation")]
    #[test]
    fn invalid_region_rejected() {
        let central = CentralEnv {
            region: "EU-WEST-1".into(), // uppercase invalid
            environment: "production".into(),
            service_version: "1.0.0".into(),
            tenant_id: None,
        };

        let mut cfg = UnifiedObservabilityConfig::baseline("svc".into());
        cfg.region = central.region.clone();

        let res = crate::validation::validate_strict(&cfg, &central);
        assert!(res.is_err());
    }
}
