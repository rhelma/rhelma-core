//! Shared service health & dependency-aware readiness for Rhelma services.
//!
//! This crate standardizes the three platform health endpoints:
//!
//! - **`/livez`** and **`/healthz`** — *liveness*. Always `200 OK` while the
//!   process is up. Used by orchestrators to decide whether to restart a pod.
//!   They never touch dependencies, so they cannot be slow or self-DoS.
//! - **`/readyz`** — *readiness*. Aggregates dependency probes (DB, Redis,
//!   Kafka, downstream services, …) and returns `503` only when a **required**
//!   dependency is down. Soft/optional dependency failures report `degraded`
//!   but still return `200` (the service can serve traffic).
//!
//! The pattern is generalized from `ai-orchestrator`'s hand-rolled health
//! module: per-probe timeouts, concurrent evaluation, a short result cache so
//! probes can't be hammered, and low-cardinality Prometheus gauges.
//!
//! ## Usage
//!
//! ```no_run
//! use std::time::Duration;
//! use rhelma_health::{HealthRegistry, CheckOutcome};
//!
//! # async fn build() -> axum::Router {
//! let health = HealthRegistry::builder("my-service")
//!     .probe_timeout(Duration::from_millis(500))
//!     .cache_ttl(Duration::from_secs(3))
//!     // Required: a hard dependency. If it fails, /readyz -> 503.
//!     .required("postgres", || async {
//!         // ... ping the pool ...
//!         CheckOutcome::ok()
//!     })
//!     // Optional: a soft dependency. If it fails, /readyz -> 200 "degraded".
//!     .optional("redis", || async { CheckOutcome::ok() })
//!     .build();
//!
//! // Merge the self-contained health router into the app. It carries its own
//! // state, so it composes with any parent router regardless of its state type.
//! axum::Router::new().merge(rhelma_health::routes(health))
//! # }
//! ```

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::get, Json, Router};
use chrono::{DateTime, Utc};
use futures::future::join_all;
use serde::Serialize;
use tokio::sync::RwLock;

/// Default per-probe timeout. Probes must be cheap and time-bounded so a slow
/// dependency cannot make readiness checks pile up.
pub const DEFAULT_PROBE_TIMEOUT: Duration = Duration::from_millis(500);

/// Default readiness cache TTL. Results are reused for this long to keep load
/// off dependencies when probes are polled frequently.
pub const DEFAULT_CACHE_TTL: Duration = Duration::from_secs(3);

/// Overall readiness status of a service.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadyStatus {
    /// All dependencies healthy.
    Ready,
    /// All *required* dependencies healthy, but one or more *optional* ones are
    /// down. The service can still serve traffic.
    Degraded,
    /// At least one *required* dependency is down. The service should not
    /// receive traffic; `/readyz` returns `503`.
    NotReady,
}

impl ReadyStatus {
    /// Stable string form used in JSON and metrics.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            ReadyStatus::Ready => "ready",
            ReadyStatus::Degraded => "degraded",
            ReadyStatus::NotReady => "not_ready",
        }
    }

    /// HTTP status a readiness probe should return for this state.
    /// `degraded` is still `200` (traffic-serving); only `not_ready` is `503`.
    #[must_use]
    pub fn http_status(self) -> StatusCode {
        match self {
            ReadyStatus::NotReady => StatusCode::SERVICE_UNAVAILABLE,
            ReadyStatus::Ready | ReadyStatus::Degraded => StatusCode::OK,
        }
    }
}

/// Result of running one dependency probe.
#[derive(Debug, Clone)]
pub struct CheckOutcome {
    /// Whether the dependency is reachable/healthy.
    pub ok: bool,
    /// Optional human-readable detail (error message when not ok).
    pub detail: Option<String>,
}

impl CheckOutcome {
    /// A healthy outcome with no detail.
    #[must_use]
    pub fn ok() -> Self {
        Self {
            ok: true,
            detail: None,
        }
    }

    /// A healthy outcome with a detail message (e.g. a version or latency note).
    #[must_use]
    pub fn ok_with(detail: impl Into<String>) -> Self {
        Self {
            ok: true,
            detail: Some(detail.into()),
        }
    }

    /// A failing outcome with a reason.
    #[must_use]
    pub fn fail(detail: impl Into<String>) -> Self {
        Self {
            ok: false,
            detail: Some(detail.into()),
        }
    }

    /// Build an outcome from a `Result`, using the error's `Display` as detail.
    pub fn from_result<T, E: std::fmt::Display>(r: Result<T, E>) -> Self {
        match r {
            Ok(_) => Self::ok(),
            Err(e) => Self::fail(e.to_string()),
        }
    }
}

/// A single check as serialized in the readiness report.
#[derive(Debug, Clone, Serialize)]
pub struct Check {
    /// Dependency name (low cardinality — used as a metric label).
    pub name: String,
    /// Whether this dependency is required for the service to be ready.
    pub required: bool,
    /// Whether the dependency is currently healthy.
    pub ok: bool,
    /// Optional detail (failure reason, version, …).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// How long the probe took, in milliseconds.
    pub latency_ms: u64,
}

/// The full readiness report serialized at `/readyz`.
#[derive(Debug, Clone, Serialize)]
pub struct ReadinessReport {
    /// Aggregate status.
    pub status: ReadyStatus,
    /// Service name.
    pub service: String,
    /// Per-dependency check results.
    pub checks: Vec<Check>,
    /// When the report was computed.
    pub checked_at: DateTime<Utc>,
    /// Whether this report was served from the short-lived cache.
    pub cached: bool,
}

type ProbeFn = Arc<dyn Fn() -> Pin<Box<dyn Future<Output = CheckOutcome> + Send>> + Send + Sync>;

struct ProbeDef {
    name: String,
    required: bool,
    run: ProbeFn,
}

/// Builder for a [`HealthRegistry`].
pub struct HealthRegistryBuilder {
    service: String,
    probes: Vec<ProbeDef>,
    timeout: Duration,
    ttl: Duration,
}

impl HealthRegistryBuilder {
    fn push<F, Fut>(mut self, name: impl Into<String>, required: bool, f: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = CheckOutcome> + Send + 'static,
    {
        let run: ProbeFn = Arc::new(move || Box::pin(f()));
        self.probes.push(ProbeDef {
            name: name.into(),
            required,
            run,
        });
        self
    }

    /// Register a **required** dependency probe. If it fails, `/readyz` -> `503`.
    #[must_use]
    pub fn required<F, Fut>(self, name: impl Into<String>, f: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = CheckOutcome> + Send + 'static,
    {
        self.push(name, true, f)
    }

    /// Register an **optional** dependency probe. If it fails, `/readyz` stays
    /// `200` but the status becomes `degraded`.
    #[must_use]
    pub fn optional<F, Fut>(self, name: impl Into<String>, f: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = CheckOutcome> + Send + 'static,
    {
        self.push(name, false, f)
    }

    /// Register a probe, choosing required/optional explicitly.
    #[must_use]
    pub fn probe<F, Fut>(self, name: impl Into<String>, required: bool, f: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = CheckOutcome> + Send + 'static,
    {
        self.push(name, required, f)
    }

    /// Override the per-probe timeout (default [`DEFAULT_PROBE_TIMEOUT`]).
    #[must_use]
    pub fn probe_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout.max(Duration::from_millis(10));
        self
    }

    /// Override the readiness cache TTL (default [`DEFAULT_CACHE_TTL`]).
    #[must_use]
    pub fn cache_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }

    /// Finalize into a shareable [`HealthRegistry`].
    #[must_use]
    pub fn build(self) -> Arc<HealthRegistry> {
        Arc::new(HealthRegistry {
            service: self.service,
            probes: self.probes,
            timeout: self.timeout,
            ttl: self.ttl,
            cache: RwLock::new(None),
        })
    }
}

/// A service's set of dependency probes plus a short readiness-result cache.
pub struct HealthRegistry {
    service: String,
    probes: Vec<ProbeDef>,
    timeout: Duration,
    ttl: Duration,
    cache: RwLock<Option<(Instant, ReadinessReport)>>,
}

impl HealthRegistry {
    /// Start building a registry for `service`.
    #[must_use]
    pub fn builder(service: impl Into<String>) -> HealthRegistryBuilder {
        HealthRegistryBuilder {
            service: service.into(),
            probes: Vec::new(),
            timeout: DEFAULT_PROBE_TIMEOUT,
            ttl: DEFAULT_CACHE_TTL,
        }
    }

    /// The service name.
    #[must_use]
    pub fn service(&self) -> &str {
        &self.service
    }

    /// Evaluate readiness, serving a cached report when one is still fresh.
    pub async fn evaluate(&self) -> ReadinessReport {
        if self.ttl > Duration::ZERO {
            let guard = self.cache.read().await;
            if let Some((computed_at, report)) = guard.as_ref() {
                if computed_at.elapsed() < self.ttl {
                    let mut cached = report.clone();
                    cached.cached = true;
                    record_metrics(&cached);
                    return cached;
                }
            }
        }

        let report = self.compute().await;

        if self.ttl > Duration::ZERO {
            let mut guard = self.cache.write().await;
            *guard = Some((Instant::now(), report.clone()));
        }

        record_metrics(&report);
        report
    }

    async fn compute(&self) -> ReadinessReport {
        // Run all probes concurrently, each bounded by the probe timeout.
        let futs = self.probes.iter().map(|p| {
            let timeout = self.timeout;
            async move {
                let started = Instant::now();
                let outcome = match tokio::time::timeout(timeout, (p.run)()).await {
                    Ok(o) => o,
                    Err(_) => CheckOutcome::fail(format!(
                        "probe timed out after {}ms",
                        timeout.as_millis()
                    )),
                };
                Check {
                    name: p.name.clone(),
                    required: p.required,
                    ok: outcome.ok,
                    detail: outcome.detail,
                    latency_ms: started.elapsed().as_millis() as u64,
                }
            }
        });

        let checks: Vec<Check> = join_all(futs).await;

        let any_required_down = checks.iter().any(|c| c.required && !c.ok);
        let any_optional_down = checks.iter().any(|c| !c.required && !c.ok);
        let status = if any_required_down {
            ReadyStatus::NotReady
        } else if any_optional_down {
            ReadyStatus::Degraded
        } else {
            ReadyStatus::Ready
        };

        ReadinessReport {
            status,
            service: self.service.clone(),
            checks,
            checked_at: Utc::now(),
            cached: false,
        }
    }
}

/// Emit low-cardinality readiness gauges. Names are uniform across all services
/// (the `service` label distinguishes them) so one Prometheus rule/dashboard
/// covers the whole fleet.
fn record_metrics(report: &ReadinessReport) {
    let ready = if report.status == ReadyStatus::NotReady {
        0.0
    } else {
        1.0
    };
    metrics::gauge!("rhelma_service_ready", "service" => report.service.clone()).set(ready);
    for c in &report.checks {
        metrics::gauge!(
            "rhelma_dependency_up",
            "service" => report.service.clone(),
            "dependency" => c.name.clone(),
            "required" => if c.required { "true" } else { "false" },
        )
        .set(if c.ok { 1.0 } else { 0.0 });
    }
}

#[derive(Serialize)]
struct LiveBody {
    status: &'static str,
}

async fn livez() -> impl IntoResponse {
    (StatusCode::OK, Json(LiveBody { status: "alive" }))
}

async fn readyz(State(reg): State<Arc<HealthRegistry>>) -> impl IntoResponse {
    let report = reg.evaluate().await;
    (report.status.http_status(), Json(report))
}

/// Build a self-contained router exposing `/livez`, `/healthz`, and `/readyz`.
///
/// The router carries its own state ([`HealthRegistry`]), so it can be
/// `.merge()`d into any application router regardless of that router's state
/// type. Liveness paths are aliased so both `/livez` and `/healthz` work.
pub fn routes(registry: Arc<HealthRegistry>) -> Router {
    Router::new()
        .route("/livez", get(livez))
        .route("/healthz", get(livez))
        .route("/readyz", get(readyz))
        .with_state(registry)
}

/// Probe a downstream HTTP service's conventional health endpoints.
///
/// Tries `/readyz`, `/ready`, `/healthz`, `/health` in order and returns `ok`
/// on the first success. Used for "is my downstream up" readiness checks.
#[cfg(feature = "http")]
pub async fn http_probe(base_url: &str, timeout: Duration) -> CheckOutcome {
    use rhelma_http_observability::reqwest::ReqwestRequestBuilderExt;

    let base = base_url.trim_end_matches('/');
    let client = match reqwest::Client::builder().timeout(timeout).build() {
        Ok(c) => c,
        Err(e) => return CheckOutcome::fail(format!("client build failed: {e}")),
    };

    let mut last: Option<String> = None;
    for path in ["/readyz", "/ready", "/healthz", "/health"] {
        let url = format!("{base}{path}");
        match client.get(&url).with_rhelma_observability().send().await {
            Ok(resp) if resp.status().is_success() || resp.status().is_redirection() => {
                return CheckOutcome::ok_with(format!("{path} -> {}", resp.status().as_u16()));
            }
            Ok(resp) => last = Some(format!("{path} -> {}", resp.status().as_u16())),
            Err(e) => last = Some(format!("{path}: {e}")),
        }
    }
    CheckOutcome::fail(last.unwrap_or_else(|| "no health endpoint reachable".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn all_ok_is_ready() {
        let reg = HealthRegistry::builder("svc")
            .required("db", || async { CheckOutcome::ok() })
            .optional("redis", || async { CheckOutcome::ok() })
            .cache_ttl(Duration::ZERO)
            .build();
        let r = reg.evaluate().await;
        assert_eq!(r.status, ReadyStatus::Ready);
        assert_eq!(r.checks.len(), 2);
    }

    #[tokio::test]
    async fn optional_down_is_degraded_but_200() {
        let reg = HealthRegistry::builder("svc")
            .required("db", || async { CheckOutcome::ok() })
            .optional("redis", || async { CheckOutcome::fail("conn refused") })
            .cache_ttl(Duration::ZERO)
            .build();
        let r = reg.evaluate().await;
        assert_eq!(r.status, ReadyStatus::Degraded);
        assert_eq!(r.status.http_status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn required_down_is_not_ready_503() {
        let reg = HealthRegistry::builder("svc")
            .required("db", || async { CheckOutcome::fail("conn refused") })
            .cache_ttl(Duration::ZERO)
            .build();
        let r = reg.evaluate().await;
        assert_eq!(r.status, ReadyStatus::NotReady);
        assert_eq!(r.status.http_status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn slow_probe_times_out_as_failure() {
        let reg = HealthRegistry::builder("svc")
            .required("slow", || async {
                tokio::time::sleep(Duration::from_secs(10)).await;
                CheckOutcome::ok()
            })
            .probe_timeout(Duration::from_millis(50))
            .cache_ttl(Duration::ZERO)
            .build();
        let r = reg.evaluate().await;
        assert_eq!(r.status, ReadyStatus::NotReady);
        assert!(r.checks[0].detail.as_deref().unwrap().contains("timed out"));
    }
}
