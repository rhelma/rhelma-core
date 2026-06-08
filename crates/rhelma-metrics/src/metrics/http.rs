use metrics::{counter, gauge, histogram, SharedString};
use once_cell::sync::Lazy;
use smallvec::SmallVec;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};

/// Rhelma v5.1 HTTP Metrics
///
/// No Box::leak, no allocations on the hot path, all labels must be canonical & 'static.
///
/// Endpoint normalization MUST happen at the routing layer (Axum/Actix/etc.).
/// This module adds an additional *fail-safe* against accidental high-cardinality
/// endpoint labels (e.g. leaking `/users/123` into metrics):
///
/// - Keep track of unique `endpoint` labels observed in-process.
/// - If the unique count exceeds a safe limit, clamp endpoint labels to `"/other"`.
/// - Emit a low-cardinality counter when clamping happens, and a gauge for unique endpoints.
///
/// This is a last line of defense; callers should still normalize endpoints properly.
const CLAMPED_ENDPOINT: &str = "/other";
const DEFAULT_ENDPOINT_LIMIT: usize = 200;

static ENDPOINT_LIMIT: Lazy<usize> = Lazy::new(|| {
    // Allow a limited, safe override for large monorepos or gateway-style services.
    // Note: we intentionally accept only reasonable bounds to prevent misconfig explosions.
    for key in [
        "RHELMA_METRICS__HTTP_ENDPOINT_LIMIT",
        "RHELMA_OBS__HTTP_ENDPOINT_LIMIT",
        "RHELMA_OBSERVABILITY__HTTP_ENDPOINT_LIMIT",
    ] {
        if let Ok(raw) = std::env::var(key) {
            if let Ok(v) = raw.trim().parse::<usize>() {
                if (10..=5000).contains(&v) {
                    return v;
                }
            }
        }
    }
    DEFAULT_ENDPOINT_LIMIT
});

#[derive(Debug, Clone)]
struct EndpointDecision {
    clamped: bool,
    newly_added: bool,
    unique_count: usize,
    /// A metric-safe endpoint label value.
    ///
    /// The `metrics` macros require label values that outlive the call site; using
    /// `SharedString` keeps endpoint labels owned/interned without leaking memory.
    label: SharedString,
}

#[derive(Debug)]
struct EndpointLimiterInner {
    limit: usize,
    // NOTE: read-mostly; write happens only when new endpoints are seen.
    seen: RwLock<HashMap<String, SharedString>>,
    unique: AtomicUsize,
}

impl EndpointLimiterInner {
    fn new(limit: usize) -> Self {
        Self {
            limit,
            seen: RwLock::new(HashMap::new()),
            unique: AtomicUsize::new(0),
        }
    }

    fn decide(&self, endpoint: &str) -> EndpointDecision {
        if endpoint == CLAMPED_ENDPOINT {
            return EndpointDecision {
                clamped: false,
                newly_added: false,
                unique_count: self.unique.load(Ordering::Relaxed),
                label: SharedString::from(CLAMPED_ENDPOINT),
            };
        }

        // Fast-ish path: if already seen, no writes.
        if let Ok(guard) = self.seen.read() {
            if let Some(lbl) = guard.get(endpoint) {
                return EndpointDecision {
                    clamped: false,
                    newly_added: false,
                    unique_count: self.unique.load(Ordering::Relaxed),
                    label: lbl.clone(),
                };
            }
        }

        // If we're already over the limit, clamp without any mutation.
        let uniq = self.unique.load(Ordering::Relaxed);
        if uniq >= self.limit {
            return EndpointDecision {
                clamped: true,
                newly_added: false,
                unique_count: uniq,
                label: SharedString::from(CLAMPED_ENDPOINT),
            };
        }

        // Slow path: try to insert under write lock.
        let mut unique_count = uniq;

        match self.seen.write() {
            Ok(mut guard) => {
                // Another thread might have inserted while we waited.
                if let Some(lbl) = guard.get(endpoint) {
                    unique_count = guard.len();
                    return EndpointDecision {
                        clamped: false,
                        newly_added: false,
                        unique_count,
                        label: lbl.clone(),
                    };
                }

                if guard.len() >= self.limit {
                    return EndpointDecision {
                        clamped: true,
                        newly_added: false,
                        unique_count: guard.len(),
                        label: SharedString::from(CLAMPED_ENDPOINT),
                    };
                }

                // Intern endpoint as an owned, cloneable shared string.
                let lbl: SharedString = endpoint.to_string().into();
                guard.insert(endpoint.to_string(), lbl.clone());
                unique_count = guard.len();
                self.unique.store(unique_count, Ordering::Relaxed);

                EndpointDecision {
                    clamped: false,
                    newly_added: true,
                    unique_count,
                    label: lbl,
                }
            }
            Err(_) => EndpointDecision {
                clamped: true,
                newly_added: false,
                unique_count,
                label: SharedString::from(CLAMPED_ENDPOINT),
            },
        }
    }
}

#[derive(Debug, Clone)]
struct EndpointLimiter {
    inner: Arc<EndpointLimiterInner>,
}

impl EndpointLimiter {
    fn new(limit: usize) -> Self {
        Self {
            inner: Arc::new(EndpointLimiterInner::new(limit)),
        }
    }

    fn decide(&self, endpoint: &str) -> EndpointDecision {
        self.inner.decide(endpoint)
    }
}

#[derive(Debug, Clone)]
pub struct HttpMetrics {
    service_name: &'static str,
    environment: &'static str,
    region: &'static str,
    limiter: EndpointLimiter,
}

impl HttpMetrics {
    /// Create new HttpMetrics instance.
    ///
    /// service_name, environment, region are &'static str because:
    /// - They come from MetricRegistry where they are stabilized once per process.
    /// - They never change and never allocate.
    pub fn new(
        service_name: &'static str,
        environment: &'static str,
        region: Option<&'static str>,
    ) -> Self {
        Self::new_with_endpoint_limit(service_name, environment, region, *ENDPOINT_LIMIT)
    }

    fn new_with_endpoint_limit(
        service_name: &'static str,
        environment: &'static str,
        region: Option<&'static str>,
        endpoint_limit: usize,
    ) -> Self {
        Self {
            service_name,
            environment,
            region: region.unwrap_or("unknown"),
            limiter: EndpointLimiter::new(endpoint_limit),
        }
    }

    /// Canonical HTTP method.
    #[inline]
    fn method_to_static(method: &str) -> &'static str {
        match method {
            "GET" => "GET",
            "POST" => "POST",
            "PUT" => "PUT",
            "DELETE" => "DELETE",
            "PATCH" => "PATCH",
            "HEAD" => "HEAD",
            "OPTIONS" => "OPTIONS",
            _ => "OTHER",
        }
    }

    /// Classify status code into stable metric-compatible groups.
    ///
    /// Prevents massive cardinality by grouping raw status numbers.
    #[inline]
    fn status_class(status: u16) -> &'static str {
        match status {
            100..=199 => "1xx",
            200..=299 => "2xx",
            300..=399 => "3xx",
            400..=499 => "4xx",
            500..=599 => "5xx",
            _ => "other",
        }
    }

    #[inline]
    fn on_endpoint_decision(&self, d: &EndpointDecision) {
        // Clamp counter is low-cardinality and helps alert on misconfiguration / regressions.
        if d.clamped {
            counter!(
                "rhelma_http_endpoint_cardinality_clamped_total",
                "service" => self.service_name,
                "environment" => self.environment,
                "region" => self.region,
                "reason" => "limit_exceeded",
            )
            .increment(1);
        }

        // Gauge updates only on new endpoints; keeps overhead minimal.
        if d.newly_added {
            gauge!(
                "rhelma_http_endpoint_unique",
                "service" => self.service_name,
                "environment" => self.environment,
                "region" => self.region,
            )
            .set(d.unique_count as f64);
        }
    }

    /// Core HTTP request recorder.
    pub fn record(&self, method: &str, endpoint: &str, status: u16, duration_secs: f64) {
        let m = Self::method_to_static(method);
        let s = Self::status_class(status);

        let d = self.limiter.decide(endpoint);
        self.on_endpoint_decision(&d);

        let ep = d.label.clone();

        counter!(
            "rhelma_http_requests_total",
            "service" => self.service_name,
            "environment" => self.environment,
            "region" => self.region,
            "method" => m,
            "endpoint" => ep.clone(),
            "status" => s,
        )
        .increment(1);

        histogram!(
            "rhelma_http_request_duration_seconds",
            "service" => self.service_name,
            "environment" => self.environment,
            "region" => self.region,
            "method" => m,
            "endpoint" => ep,
            "status" => s,
        )
        .record(duration_secs);
    }

    /// Record HTTP request including request/response sizes.
    pub fn record_with_bytes(
        &self,
        method: &str,
        endpoint: &str,
        status: u16,
        duration_secs: f64,
        request_bytes: u64,
        response_bytes: u64,
    ) {
        let m = Self::method_to_static(method);
        let s = Self::status_class(status);

        let d = self.limiter.decide(endpoint);
        self.on_endpoint_decision(&d);

        let ep = d.label.clone();

        // Base metrics
        counter!(
            "rhelma_http_requests_total",
            "service" => self.service_name,
            "environment" => self.environment,
            "region" => self.region,
            "method" => m,
            "endpoint" => ep.clone(),
            "status" => s,
        )
        .increment(1);

        histogram!(
            "rhelma_http_request_duration_seconds",
            "service" => self.service_name,
            "environment" => self.environment,
            "region" => self.region,
            "method" => m,
            "endpoint" => ep.clone(),
            "status" => s,
        )
        .record(duration_secs);

        // Byte counters
        counter!(
            "rhelma_http_request_bytes_total",
            "direction" => "in",
            "service" => self.service_name,
            "environment" => self.environment,
            "region" => self.region,
            "method" => m,
            "endpoint" => ep.clone(),
        )
        .increment(request_bytes);

        counter!(
            "rhelma_http_response_bytes_total",
            "direction" => "out",
            "service" => self.service_name,
            "environment" => self.environment,
            "region" => self.region,
            "method" => m,
            "endpoint" => ep,
        )
        .increment(response_bytes);
    }

    /// Extended recorder that allows limited extra labels.
    ///
    /// NOTE: ONLY use for low-cardinality fixed labels!
    pub fn record_with_labels(
        &self,
        method: &str,
        endpoint: &'static str,
        status: u16,
        duration_secs: f64,
        extra: &[(&'static str, &'static str)],
    ) {
        let m = Self::method_to_static(method);
        let s = Self::status_class(status);

        let d = self.limiter.decide(endpoint);
        self.on_endpoint_decision(&d);

        let ep: &'static str = if d.clamped {
            CLAMPED_ENDPOINT
        } else {
            endpoint
        };

        // NOTE: metrics macros support a labels slice; this keeps the callsite flexible.
        let mut labels: SmallVec<[(&'static str, &'static str); 32]> = SmallVec::new();
        labels.push(("service", self.service_name));
        labels.push(("environment", self.environment));
        labels.push(("region", self.region));
        labels.push(("method", m));
        labels.push(("endpoint", ep));
        labels.push(("status", s));
        for &(k, v) in extra {
            labels.push((k, v));
        }

        counter!("rhelma_http_requests_total", labels.as_slice()).increment(1);
        histogram!("rhelma_http_request_duration_seconds", labels.as_slice()).record(duration_secs);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_limiter_clamps_after_limit() {
        let http = HttpMetrics::new_with_endpoint_limit("svc", "dev", Some("eu"), 2);

        // First 2 are accepted.
        let d1 = http.limiter.decide("/a");
        assert!(!d1.clamped);
        assert!(d1.newly_added);

        let d2 = http.limiter.decide("/b");
        assert!(!d2.clamped);
        assert!(d2.newly_added);

        // Third unique endpoint should be clamped.
        let d3 = http.limiter.decide("/c");
        assert!(d3.clamped);
    }

    #[test]
    fn http_metrics_basic() {
        let http = HttpMetrics::new_with_endpoint_limit("svc", "dev", Some("eu"), 100);
        http.record("GET", "/health", 200, 0.01);
        http.record("POST", "/items/{id}", 201, 0.02);
    }

    #[test]
    fn http_metrics_bytes() {
        let http = HttpMetrics::new_with_endpoint_limit("svc", "prod", Some("us-west"), 100);
        http.record_with_bytes("POST", "/upload", 201, 0.5, 1024, 2048);
    }
}
