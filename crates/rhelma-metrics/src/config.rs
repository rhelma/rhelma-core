use serde::{Deserialize, Serialize};
use thiserror::Error;

// Needed for Lazy + Regex validation
use once_cell::sync::Lazy;
use regex::Regex;

/// Configuration for rhelma-metrics (v5.1 semantic-only config).
///
/// This config contains ONLY metadata required for deterministic naming
/// and stable labeling.  No exporter/recorder backend lives here.
///
/// Backends (Prometheus, OTEL) are configured inside rhelma-observability-core.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MetricsConfig {
    /// Logical service name (must be non-empty)
    pub service_name: String,

    /// Deployment environment: development / staging / production / etc.
    pub environment: String,

    /// Optional region code: `eu-west-1`
    pub region: Option<String>,

    /// Optional zone: `az1`
    pub deployment_zone: Option<String>,

    /// Optional cluster identifier
    pub cluster_id: Option<String>,

    /// Optional version string (git SHA or semantic version)
    pub service_version: Option<String>,

    /// Optional instance identifier
    pub instance_id: Option<String>,

    /// Namespace prefix for all metrics (default: "rhelma")
    pub namespace: String,

    /// Default labels added to every metric
    pub default_labels: Vec<(String, String)>,
    /// Optional histogram buckets override.
    /// MUST be strictly increasing.
    pub histogram_buckets: Option<Vec<f64>>,
}

impl MetricsConfig {
    pub fn new(service_name: &str) -> Self {
        Self {
            service_name: service_name.to_owned(),
            environment: "development".into(),
            region: None,
            deployment_zone: None,
            cluster_id: None,
            service_version: None,
            instance_id: None,
            namespace: "rhelma".into(),
            default_labels: Vec::new(),
            // REQUIRED fix
            histogram_buckets: None,
        }
    }

    pub fn with_namespace(mut self, ns: &str) -> Self {
        self.namespace = ns.to_owned();
        self
    }

    pub fn with_label(mut self, key: &str, value: &str) -> Self {
        self.default_labels.push((key.to_owned(), value.to_owned()));
        self
    }

    pub fn with_env(mut self, env: &str) -> Self {
        self.environment = env.to_owned();
        self
    }

    pub fn with_region(mut self, region: &str) -> Self {
        self.region = Some(region.to_owned());
        self
    }

    pub fn with_version(mut self, v: &str) -> Self {
        self.service_version = Some(v.to_owned());
        self
    }

    pub fn with_buckets(mut self, buckets: &[f64]) -> Self {
        self.histogram_buckets = Some(buckets.to_vec());
        self
    }

    /// Merge semantic-only configuration (other overrides self)
    pub fn merge(mut self, other: &Self) -> Self {
        if !other.service_name.is_empty() {
            self.service_name = other.service_name.clone();
        }
        if !other.environment.is_empty() {
            self.environment = other.environment.clone();
        }
        if other.region.is_some() {
            self.region = other.region.clone();
        }
        if other.cluster_id.is_some() {
            self.cluster_id = other.cluster_id.clone();
        }
        if other.deployment_zone.is_some() {
            self.deployment_zone = other.deployment_zone.clone();
        }
        if other.service_version.is_some() {
            self.service_version = other.service_version.clone();
        }
        if other.instance_id.is_some() {
            self.instance_id = other.instance_id.clone();
        }
        if !other.namespace.is_empty() {
            self.namespace = other.namespace.clone();
        }

        if other.histogram_buckets.is_some() {
            self.histogram_buckets = other.histogram_buckets.clone();
        }

        self.default_labels.extend(other.default_labels.clone());
        self
    }

    // ---------------------------
    // VALIDATION
    // ---------------------------

    pub fn validate(&self) -> Result<(), MetricsConfigError> {
        // 1. service_name
        if self.service_name.trim().is_empty() {
            return Err(MetricsConfigError::Invalid(
                "service_name cannot be empty".into(),
            ));
        }

        // 2. Namespace validation
        static RE_NS: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"^[a-zA-Z][a-zA-Z0-9_:]*$").expect("regex compile failure"));

        if !RE_NS.is_match(&self.namespace) {
            return Err(MetricsConfigError::Invalid(format!(
                "invalid namespace '{}'",
                self.namespace
            )));
        }

        // 3. environment must not be empty
        if self.environment.trim().is_empty() {
            return Err(MetricsConfigError::Invalid(
                "environment cannot be empty".into(),
            ));
        }

        // 4. region format (lowercase, dash separated)
        static RE_REGION: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"^[a-z]{2,}(-[a-z0-9]+)+$").expect("regex compile failure"));

        if let Some(region) = &self.region {
            if !RE_REGION.is_match(region) {
                return Err(MetricsConfigError::Invalid(format!(
                    "invalid region '{}'",
                    region
                )));
            }
        }

        // 5. Default labels cardinality limit
        if self.default_labels.len() > 20 {
            return Err(MetricsConfigError::Invalid(
                "too many default labels (>20), high cardinality risk".into(),
            ));
        }

        // 6. Validate size of keys/values
        for (k, v) in &self.default_labels {
            if k.len() > 64 {
                return Err(MetricsConfigError::Invalid(format!(
                    "label key '{}' too long (max 64 chars)",
                    k
                )));
            }
            if v.len() > 256 {
                return Err(MetricsConfigError::Invalid(format!(
                    "label value for '{}' too long (max 256 chars)",
                    k
                )));
            }
        }

        // histogram buckets
        if let Some(b) = &self.histogram_buckets {
            if b.len() < 2 {
                return Err(MetricsConfigError::Invalid(
                    "histogram_buckets must contain at least 2 values".into(),
                ));
            }
            if !b.windows(2).all(|w| w[0] < w[1]) {
                return Err(MetricsConfigError::Invalid(
                    "histogram_buckets must be strictly increasing".into(),
                ));
            }
        }

        Ok(())
    }

    /// Build from UnifiedObservabilityConfig (if enabled)
    #[cfg(feature = "with-config")]
    pub fn from_unified(
        service_name: &str,
        obs: &rhelma_config::UnifiedObservabilityConfig,
    ) -> Self {
        let mut cfg = Self::new(service_name);

        // Environment enum -> lowercase string
        cfg.environment = format!("{:?}", obs.environment).to_lowercase();

        // Region is a concrete string in rhelma-config
        if !obs.region.trim().is_empty() {
            cfg.region = Some(obs.region.clone());
        }

        // Service version is a concrete string in rhelma-config
        if !obs.service_version.trim().is_empty() {
            cfg.service_version = Some(obs.service_version.clone());
        }

        // Optional fields not currently provided by rhelma-config's unified model
        cfg.deployment_zone = None;
        cfg.cluster_id = None;
        cfg.instance_id = None;

        // Canonical labels
        cfg.default_labels
            .push(("service".into(), service_name.into()));
        cfg.default_labels
            .push(("env".into(), cfg.environment.clone()));
        if let Some(region) = &cfg.region {
            cfg.default_labels.push(("region".into(), region.clone()));
        }

        cfg
    }
}

#[derive(Debug, Error)]
pub enum MetricsConfigError {
    #[error("invalid metrics configuration: {0}")]
    /// Variant `Invalid`.
    Invalid(String),
}
