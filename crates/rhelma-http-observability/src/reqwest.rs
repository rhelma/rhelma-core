#![forbid(unsafe_code)]

//! Reqwest helpers for Rhelma Contract v6.0 outbound propagation.
//!
//! Goals:
//! - Propagate W3C tracing (`traceparent`/`tracestate`) when available.
//! - Propagate canonical Rhelma request/correlation IDs.
//! - Propagate tenancy/location (`x-tenant-id`, `x-region`) when present in local context.
//! - Propagate residency policy.
//! - Ensure the minimum Contract v6.0 header set exists even outside a scoped request context.
//!
//! Design notes:
//! - **Fail-open:** this must never bring a service down.
//! - **Low-cardinality:** we only inject a small allow-list of headers.
//! - **Best-effort OTEL:** when the `otel` feature is enabled, we inject using the current span's
//!   OpenTelemetry context; otherwise we fall back to `rhelma_tracing::context`.

use http::HeaderMap;
use reqwest::RequestBuilder;

/// Extension trait for `reqwest::RequestBuilder`.
pub trait ReqwestRequestBuilderExt {
    /// Inject Rhelma v6.0 observability headers into an outbound request.
    ///
    /// Injects:
    /// - `traceparent` (+ optional `tracestate`)
    /// - `x-rhelma-request-id`
    /// - `x-rhelma-correlation-id`
    /// - `x-tenant-id`
    /// - `x-region`
    /// - `x-residency`
    ///
    /// Then calls [`crate::ensure_contract_v60`] to guarantee required headers exist / are normalized.
    fn with_rhelma_observability(self) -> Self;
}

impl ReqwestRequestBuilderExt for RequestBuilder {
    fn with_rhelma_observability(self) -> Self {
        let mut h = HeaderMap::new();

        // 1) Try OTEL injection (when compiled in). If nothing gets injected, fall back.
        #[cfg(feature = "otel")]
        {
            if crate::otel::try_inject_otel_headers(&mut h) {
                // OTEL injected at least one header.
            } else {
                crate::fallback::inject_trace_headers_from_context(&mut h);
            }
        }

        #[cfg(not(feature = "otel"))]
        {
            crate::fallback::inject_trace_headers_from_context(&mut h);
        }

        // 2) Propagate request/correlation/tenant/region/residency from local context.
        crate::insert_request_correlation_from_context(&mut h);
        crate::insert_tenant_region_from_context(&mut h);
        crate::insert_residency_from_context(&mut h);

        // 3) Ensure minimum Contract v6.0 headers exist.
        crate::ensure_contract_v60(&mut h);

        // 4) Merge into builder without clobbering existing explicit headers.
        let mut b = self;
        for (k, v) in h.iter() {
            b = b.header(k, v);
        }
        b
    }
}

/// Convenience extension trait for `reqwest::Client`.
///
/// This reduces the chance of forgetting `.with_rhelma_observability()` on new outbound calls.
pub trait ReqwestClientExt {
    /// Like `Client::get`, but injects Rhelma context immediately.
    fn rhelma_get(&self, url: impl reqwest::IntoUrl) -> reqwest::RequestBuilder;

    /// Like `Client::post`, but injects Rhelma context immediately.
    fn rhelma_post(&self, url: impl reqwest::IntoUrl) -> reqwest::RequestBuilder;

    /// Like `Client::put`, but injects Rhelma context immediately.
    fn rhelma_put(&self, url: impl reqwest::IntoUrl) -> reqwest::RequestBuilder;

    /// Like `Client::delete`, but injects Rhelma context immediately.
    fn rhelma_delete(&self, url: impl reqwest::IntoUrl) -> reqwest::RequestBuilder;

    /// Like `Client::patch`, but injects Rhelma context immediately.
    fn rhelma_patch(&self, url: impl reqwest::IntoUrl) -> reqwest::RequestBuilder;
}

impl ReqwestClientExt for reqwest::Client {
    fn rhelma_get(&self, url: impl reqwest::IntoUrl) -> reqwest::RequestBuilder {
        self.get(url).with_rhelma_observability()
    }

    fn rhelma_post(&self, url: impl reqwest::IntoUrl) -> reqwest::RequestBuilder {
        self.post(url).with_rhelma_observability()
    }

    fn rhelma_put(&self, url: impl reqwest::IntoUrl) -> reqwest::RequestBuilder {
        self.put(url).with_rhelma_observability()
    }

    fn rhelma_delete(&self, url: impl reqwest::IntoUrl) -> reqwest::RequestBuilder {
        self.delete(url).with_rhelma_observability()
    }

    fn rhelma_patch(&self, url: impl reqwest::IntoUrl) -> reqwest::RequestBuilder {
        self.patch(url).with_rhelma_observability()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn reqwest_builder_injects_contract_headers_without_network() {
        // No scope => should still generate request/correlation + residency + traceparent.
        let c = reqwest::Client::new();
        let req = c
            .get("http://example.invalid/test")
            .with_rhelma_observability()
            .build()
            .expect("build reqwest request");

        let rid = req
            .headers()
            .get("x-rhelma-request-id")
            .unwrap()
            .to_str()
            .unwrap();
        assert!(Uuid::parse_str(rid).is_ok());

        let cid = req
            .headers()
            .get("x-rhelma-correlation-id")
            .unwrap()
            .to_str()
            .unwrap();
        assert!(Uuid::parse_str(cid).is_ok());

        assert!(req.headers().get("x-residency").is_some());
        assert!(req.headers().get("traceparent").is_some());
    }
}
