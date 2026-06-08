#![forbid(unsafe_code)]
use rhelma_http_observability::reqwest::ReqwestClientExt;

use std::sync::{
    atomic::{AtomicU32, AtomicU64, Ordering},
    Arc,
};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::config::GatewayConfig;
use crate::error::GatewayError;
use crate::eventing::GatewayEventPublisher;
use crate::region_routing::RegionRoutingHandle;
use rhelma_core::request_context::ResidencyPolicy as RequestResidencyPolicy;
use rhelma_core::tenancy::ResidencyPolicy as TenancyResidencyPolicy;
use rhelma_core::{RequestContext, RhelmaError};
use rhelma_db::metrics::{DbOperation, DbOutcome};
use tracing::debug;

#[derive(Clone)]
pub struct BaseRepo {
    pool: sqlx::PgPool,
}

impl BaseRepo {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }

    pub async fn run_db<F, Fut, T>(
        &self,
        _ctx: &RequestContext,
        op: DbOperation,
        _table: Option<&'static str>,
        f: F,
    ) -> Result<T, sqlx::Error>
    where
        F: FnOnce(sqlx::PgPool) -> Fut,
        Fut: std::future::Future<Output = Result<T, sqlx::Error>>,
    {
        let start = Instant::now();
        let out = f(self.pool.clone()).await;

        let dur = start.elapsed();
        match out.as_ref() {
            Ok(_) => rhelma_db::metrics::record(op, DbOutcome::Success, dur),
            Err(_) => rhelma_db::metrics::record(op, DbOutcome::Error, dur),
        }

        out
    }
}

#[derive(Clone)]
pub struct SearchService {
    cfg: Arc<GatewayConfig>,
    http: reqwest::Client,

    /// Optional multi-region router for selecting the upstream `search-service` endpoint.
    region_router: Option<Arc<RegionRoutingHandle>>,

    // simple circuit breaker (no extra deps)
    fail_count: Arc<AtomicU32>,
    open_until_ms: Arc<AtomicU64>,

    /// Best-effort publisher for region failover events.
    event_publisher: Arc<GatewayEventPublisher>,
}

fn map_request_residency(r: RequestResidencyPolicy) -> TenancyResidencyPolicy {
    match r {
        RequestResidencyPolicy::Global => TenancyResidencyPolicy::GlobalPreferred,
        RequestResidencyPolicy::RegionalPreferred => TenancyResidencyPolicy::RegionalPreferred,
        RequestResidencyPolicy::RegionalStrict => TenancyResidencyPolicy::RegionalRequired,
    }
}

impl SearchService {
    pub fn new(
        cfg: Arc<GatewayConfig>,
        http: reqwest::Client,
        region_router: Option<Arc<RegionRoutingHandle>>,
        event_publisher: Arc<GatewayEventPublisher>,
    ) -> Self {
        Self {
            cfg,
            http,
            region_router,
            fail_count: Arc::new(AtomicU32::new(0)),
            open_until_ms: Arc::new(AtomicU64::new(0)),
            event_publisher,
        }
    }

    fn select_search_upstream(&self, ctx: &RequestContext) -> SelectedUpstream {
        let fallback = self.cfg.services.search_service_url.clone();

        let Some(router) = self.region_router.as_ref() else {
            return SelectedUpstream {
                region_id: None,
                base_url: fallback,
            };
        };

        let residency = ctx
            .residency()
            .map(map_request_residency)
            .unwrap_or(TenancyResidencyPolicy::GlobalPreferred);
        let requested_region = ctx.region().map(|r| r.as_str());

        match router.route_for_upstream("search-service", residency, requested_region) {
            Ok(decision) => match decision {
                rhelma_core::multi_region::RouteDecision::Direct(region) => {
                    let base_url = pick_endpoint(&region, ctx.request_id());
                    debug!(
                        request_id = %ctx.request_id(),
                        region = %region.region_id,
                        endpoint = %base_url,
                        "selected region upstream for search-service"
                    );
                    SelectedUpstream {
                        region_id: Some(region.region_id),
                        base_url,
                    }
                }
            },
            Err(e) => {
                debug!(
                    request_id = %ctx.request_id(),
                    error = %e,
                    "region routing failed; falling back to configured search_service_url"
                );
                metrics::counter!("rhelma_gateway_region_routing_fallback_total", "service" => "search-service", "reason" => "route_error").increment(1);
                SelectedUpstream {
                    region_id: None,
                    base_url: fallback,
                }
            }
        }
    }

    /// Execute a search request against `search-service`.
    ///
    /// Communication standard:
    /// - Always propagates RequestContext v5.2 headers (request/correlation/residency + W3C trace), as required by Contract v6.0.
    /// - Retries on 5xx with bounded exponential backoff.
    /// - Uses a tiny in-process circuit-breaker to avoid thundering herds.
    ///
    /// Notes:
    /// - `ctx` is used only for logging/telemetry. Header propagation is bound by
    ///   `request_guard_middleware` via `rhelma_tracing::context::scope_with_headers`.
    pub async fn search(
        &self,
        ctx: &RequestContext,
        query: &str,
        limit: u32,
    ) -> Result<Vec<serde_json::Value>, GatewayError> {
        if self.breaker_open() {
            return Err(GatewayError::from(RhelmaError::CircuitOpen(
                "search-service circuit open".to_string(),
            )));
        }

        // Retry policy: 3 attempts with exponential backoff + small jitter.
        let max_attempts: u32 = 3;

        let mut last_err: Option<String> = None;

        let mut prev_region: Option<String> = None;
        let mut last_fail_reason: Option<&'static str> = None;

        for attempt in 1..=max_attempts {
            let upstream = self.select_search_upstream(ctx);
            // Emit failover metric when the selected region changes between attempts.
            if attempt > 1 && upstream.region_id.as_deref() != prev_region.as_deref() {
                let from_region = prev_region
                    .clone()
                    .unwrap_or_else(|| "fallback".to_string());
                let to_region = upstream
                    .region_id
                    .clone()
                    .unwrap_or_else(|| "fallback".to_string());
                let reason = last_fail_reason.unwrap_or("unknown");
                metrics::counter!("rhelma_gateway_region_failover_total", "service" => "search-service", "from_region" => from_region.clone(), "to_region" => to_region.clone(), "reason" => reason).increment(1);
                tracing::info!(
                    request_id = %ctx.request_id(),
                    from_region = %from_region,
                    to_region = %to_region,
                    reason,
                    "search-service region failover"
                );

                // Best-effort event for cross-service awareness.
                let publisher = self.event_publisher.clone();
                let request_id = ctx.request_id().to_string();
                let correlation_id = ctx.correlation_id().map(|s| s.to_string());
                tokio::spawn(async move {
                    publisher
                        .publish_failover(
                            "search-service",
                            &request_id,
                            correlation_id.as_deref(),
                            &from_region,
                            &to_region,
                            reason,
                        )
                        .await;
                });
            }
            metrics::counter!("rhelma_gateway_search_upstream_attempt_total", "service" => "search-service", "region" => upstream.region_id.clone().unwrap_or_else(|| "fallback".to_string()), "attempt" => attempt.to_string()).increment(1);
            let url = format!("{}/search", upstream.base_url.trim_end_matches('/'));

            let resp = self
                .http
                .rhelma_post(&url)
                .timeout(self.cfg.timeouts.upstream)
                .json(&serde_json::json!({ "query": query, "limit": limit }))
                .send()
                .await;

            match resp {
                Ok(resp) => {
                    let st = resp.status();

                    if st.is_success() {
                        self.on_success();

                        let v: serde_json::Value = resp
                            .json()
                            .await
                            .map_err(|e| GatewayError::bad_gateway(format!("search parse: {e}")))?;

                        // Accept both formats:
                        // - legacy: array
                        // - current: { total, hits: [...] }
                        if let Some(arr) = v.as_array() {
                            return Ok(arr.clone());
                        }
                        if let Some(hits) = v.get("hits").and_then(|x| x.as_array()) {
                            return Ok(hits.clone());
                        }
                        return Ok(vec![v]);
                    }

                    // Read a small error hint (best-effort) for server-side logs.
                    let err_hint = resp
                        .text()
                        .await
                        .ok()
                        .map(|s| {
                            let s = s.trim().replace(['\n', '\r', '\t'], " ");
                            if s.len() > 256 {
                                format!("{}…", &s[..256])
                            } else {
                                s
                            }
                        })
                        .unwrap_or_else(|| "<no-body>".to_string());

                    tracing::warn!(
                        request_id = %ctx.request_id(),
                        correlation_id = ctx.correlation_id().unwrap_or(""),
                        status = %st,
                        attempt,
                        err_hint = %err_hint,
                        "search-service non-success response"
                    );

                    // 4xx (except 429): don't retry
                    if st.is_client_error() && st != reqwest::StatusCode::TOO_MANY_REQUESTS {
                        last_err = Some(format!("search-service status={st}"));
                        self.on_failure();
                        break;
                    }

                    // 5xx: retryable. If multi-region is enabled, mark the chosen region
                    // unhealthy to allow the next attempt to fail over.
                    if st.is_server_error() {
                        if let Some(region_id) = upstream.region_id.as_deref() {
                            if let Some(router) = self.region_router.as_ref() {
                                router.mark_health(region_id, false, u32::MAX);
                            }
                        }
                    }

                    prev_region = upstream.region_id.clone();
                    last_fail_reason = Some(if st.is_server_error() {
                        "5xx"
                    } else if st == reqwest::StatusCode::TOO_MANY_REQUESTS {
                        "429"
                    } else {
                        "non_success"
                    });
                    last_err = Some(format!("search-service status={st} (attempt {attempt})"));
                    self.on_failure();
                }
                Err(e) => {
                    let is_timeout = e.is_timeout();
                    let is_connect = e.is_connect();
                    let label = if is_timeout {
                        "timeout"
                    } else if is_connect {
                        "connect"
                    } else {
                        "request"
                    };

                    tracing::warn!(
                        request_id = %ctx.request_id(),
                        correlation_id = ctx.correlation_id().unwrap_or(""),
                        attempt,
                        label,
                        error = %e,
                        "search-service request failed"
                    );

                    // Mark region unhealthy to force failover on next attempt.
                    if let Some(region_id) = upstream.region_id.as_deref() {
                        if let Some(router) = self.region_router.as_ref() {
                            router.mark_health(region_id, false, u32::MAX);
                        }
                    }

                    prev_region = upstream.region_id.clone();
                    last_fail_reason = Some(label);
                    last_err = Some(format!("search-service {label} failed (attempt {attempt})"));
                    self.on_failure();
                }
            }

            if attempt < max_attempts {
                tokio::time::sleep(backoff_with_jitter(attempt)).await;
            }
        }

        Err(GatewayError::from(RhelmaError::Dependency(
            last_err.unwrap_or_else(|| "search request failed".to_string()),
        )))
    }

    fn breaker_open(&self) -> bool {
        Self::now_ms() < self.open_until_ms.load(Ordering::Relaxed)
    }

    fn on_success(&self) {
        self.fail_count.store(0, Ordering::Relaxed);
        self.open_until_ms.store(0, Ordering::Relaxed);
    }

    fn on_failure(&self) {
        let n = self.fail_count.fetch_add(1, Ordering::Relaxed) + 1;

        // After 5 consecutive failures => open circuit for 10s
        if n >= 5 {
            self.open_until_ms
                .store(Self::now_ms().saturating_add(10_000), Ordering::Relaxed);
        }
    }

    fn now_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }
}

#[derive(Clone, Debug)]
struct SelectedUpstream {
    region_id: Option<String>,
    base_url: String,
}

fn pick_endpoint(
    region: &rhelma_core::multi_region::RegionEndpoint,
    request_id: uuid::Uuid,
) -> String {
    if region.endpoints.is_empty() {
        return String::new();
    }

    // Stable selection based on the first byte of UUID.
    let idx = (request_id.as_bytes()[0] as usize) % region.endpoints.len();
    region.endpoints[idx].clone()
}

fn backoff_with_jitter(attempt: u32) -> Duration {
    // base: 100ms, 200ms, 400ms...
    let base_ms = 100u64.saturating_mul(2u64.saturating_pow(attempt.saturating_sub(1)));

    // jitter: 0..50ms (no rand dependency)
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    let jitter_ms = nanos % 50;

    Duration::from_millis(base_ms.saturating_add(jitter_ms))
}
