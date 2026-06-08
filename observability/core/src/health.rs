//! Small health model for core observability components.

use std::sync::{Mutex, MutexGuard};

use serde::{Deserialize, Serialize};

/// Health status of a subsystem.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum HealthStatus {
    /// Fully operational.
    #[default]
    /// Variant `Healthy`.
    Healthy,
    /// Operational but degraded.
    Degraded,
    /// Disabled by configuration.
    Disabled,
    /// Not operational.
    Down,
}

/// Health snapshot for core observability subsystems.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreHealthSnapshot {
    /// Logger health.
    pub logger: HealthStatus,
    /// Tracing health.
    pub tracing: HealthStatus,
    /// Metrics health.
    pub metrics: HealthStatus,
}

impl CoreHealthSnapshot {
    /// Compute overall status using a conservative aggregation.
    ///
    /// `Down` has the highest priority, then `Degraded`, otherwise `Healthy`.
    #[must_use]
    pub fn overall(&self) -> HealthStatus {
        use HealthStatus::{Degraded, Down, Healthy};

        if self.logger == Down || self.tracing == Down || self.metrics == Down {
            return Down;
        }
        if self.logger == Degraded || self.tracing == Degraded || self.metrics == Degraded {
            return Degraded;
        }
        Healthy
    }
}

/// Core metadata for the health center.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HealthMetadata {
    /// Optional service name.
    pub service_name: Option<String>,
    /// Optional environment.
    pub environment: Option<String>,
    /// Optional region.
    pub region: Option<String>,
    /// Optional service version.
    pub service_version: Option<String>,
}

#[derive(Debug, Default, Clone)]
struct HealthState {
    logger: HealthStatus,
    tracing: HealthStatus,
    metrics: HealthStatus,
    metadata: HealthMetadata,
}

/// In-memory tracker for subsystem health.
///
/// This is intentionally lightweight and has no external dependencies.
#[derive(Debug, Default)]
pub struct HealthCenter {
    inner: Mutex<HealthState>,
}

impl HealthCenter {
    fn lock_inner(&self) -> MutexGuard<'_, HealthState> {
        self.inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Set logger health.
    pub fn set_logger(&self, status: HealthStatus) {
        self.lock_inner().logger = status;
    }

    /// Set tracing health.
    pub fn set_tracing(&self, status: HealthStatus) {
        self.lock_inner().tracing = status;
    }

    /// Set metrics health.
    pub fn set_metrics(&self, status: HealthStatus) {
        self.lock_inner().metrics = status;
    }

    /// A new `HealthCenter` instance with provided metadata.
    #[must_use]
    pub fn new_with_metadata(metadata: HealthMetadata) -> Self {
        Self {
            inner: Mutex::new(HealthState {
                logger: HealthStatus::Healthy,
                tracing: HealthStatus::Healthy,
                metrics: HealthStatus::Healthy,
                metadata,
            }),
        }
    }

    /// A new `HealthCenter` instance with default metadata.
    #[must_use]
    pub fn new() -> Self {
        Self::new_with_metadata(HealthMetadata::default())
    }

    /// Snapshot current health state.
    #[must_use]
    pub fn snapshot(&self) -> CoreHealthSnapshot {
        let g = self.lock_inner();
        CoreHealthSnapshot {
            logger: g.logger,
            tracing: g.tracing,
            metrics: g.metrics,
        }
    }

    /// Service name if set.
    #[must_use]
    pub fn service_name(&self) -> Option<String> {
        self.lock_inner().metadata.service_name.clone()
    }

    /// Environment if set.
    #[must_use]
    pub fn environment(&self) -> Option<String> {
        self.lock_inner().metadata.environment.clone()
    }

    /// Region if set.
    #[must_use]
    pub fn region(&self) -> Option<String> {
        self.lock_inner().metadata.region.clone()
    }

    /// Service version if set.
    #[must_use]
    pub fn service_version(&self) -> Option<String> {
        self.lock_inner().metadata.service_version.clone()
    }
}

impl Clone for HealthCenter {
    fn clone(&self) -> Self {
        let snapshot = self.lock_inner().clone();
        Self {
            inner: Mutex::new(snapshot),
        }
    }
}
