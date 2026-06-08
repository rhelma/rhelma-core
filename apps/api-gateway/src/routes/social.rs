#![forbid(unsafe_code)]

use axum::{
    body::{to_bytes, Body},
    http::{header, Request, Response, StatusCode},
    response::IntoResponse,
    routing::any,
    Extension, Router,
};
use std::sync::Arc;
use std::time::Duration;

use redis::AsyncCommands;
use rhelma_core::constants;
use rhelma_http_observability::reqwest::ReqwestRequestBuilderExt;
use serde::Deserialize;
use std::collections::HashMap;

use crate::state::AppState;

/// Minimal reverse-proxy to `social-service`.
///
/// This keeps api-gateway's router tidy while allowing the social domain to
/// evolve independently in its own service.
pub fn router() -> Router {
    Router::new().route("/*path", any(proxy))
}

async fn proxy(
    Extension(state): Extension<Arc<AppState>>,
    req: Request<Body>,
) -> impl IntoResponse {
    let tenant = req
        .headers()
        .get(constants::HEADER_TENANT_ID)
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("central");

    let base = resolve_upstream(&state, tenant, "social-service")
        .await
        .unwrap_or_else(|| state.config.services.social_service_url.clone());
    let base = base.trim_end_matches('/');

    // In nested routers, axum strips the prefix. So this URI is relative to `/social`.
    let path_and_query = req
        .uri()
        .path_and_query()
        .map(|pq| pq.as_str())
        .unwrap_or(req.uri().path());

    let url = format!("{}{}", base, path_and_query);

    let (parts, body) = req.into_parts();
    let method = parts.method;

    // Copy headers (avoid hop-by-hop headers; let reqwest set content-length).
    let mut headers = parts.headers;
    headers.remove(header::HOST);
    headers.remove(header::CONTENT_LENGTH);
    headers.remove(header::TRANSFER_ENCODING);
    headers.remove(header::CONNECTION);

    let bytes = match to_bytes(body, 10 * 1024 * 1024).await {
        Ok(b) => b,
        Err(_) => {
            return (StatusCode::BAD_REQUEST, "request body too large or invalid").into_response();
        }
    };

    let res = state
        .http
        .request(method, url)
        .headers(headers)
        .body(bytes)
        .with_rhelma_observability()
        .send()
        .await;

    let res = match res {
        Ok(r) => r,
        Err(_) => {
            return (StatusCode::BAD_GATEWAY, "social-service unavailable").into_response();
        }
    };

    let status = res.status();
    let mut out = Response::builder().status(status);

    // Copy response headers (strip hop-by-hop)
    for (k, v) in res.headers().iter() {
        if k == header::TRANSFER_ENCODING || k == header::CONNECTION {
            continue;
        }
        out = out.header(k, v);
    }

    let body_bytes = match res.bytes().await {
        Ok(b) => b,
        Err(_) => {
            return (
                StatusCode::BAD_GATEWAY,
                "invalid response from social-service",
            )
                .into_response();
        }
    };

    out.body(Body::from(body_bytes)).unwrap_or_else(|_| {
        Response::builder()
            .status(StatusCode::BAD_GATEWAY)
            .body(Body::empty())
            .unwrap()
    })
}

#[derive(Debug, Deserialize)]
struct DiscoveryResponse {
    services: HashMap<String, DiscoveryService>,
}

#[derive(Debug, Deserialize)]
struct DiscoveryService {
    base_url: String,
}

async fn resolve_upstream(state: &Arc<AppState>, realm: &str, service: &str) -> Option<String> {
    let control = state.config.services.control_service_url.as_deref()?;
    let ttl: u64 = clamp_discovery_ttl_secs(state.config.discovery_cache_ttl);

    let key = discovery_cache_key(realm, service);
    let mut con = state.redis.clone();

    if let Ok(Some(cached)) = con.get::<_, Option<String>>(&key).await {
        let c = cached.trim().to_string();
        if !c.is_empty() {
            return Some(c);
        }
    }

    let mut url = reqwest::Url::parse(control).ok()?;
    url.set_path("/v1/discovery");
    url.query_pairs_mut().append_pair("realm", realm);

    let resp = state
        .http
        .get(url)
        .with_rhelma_observability()
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }

    let body: DiscoveryResponse = resp.json().await.ok()?;
    let svc = body.services.get(service)?;
    let base_url = svc.base_url.trim().to_string();
    if base_url.is_empty() {
        return None;
    }

    let _ = con.set_ex::<_, _, ()>(key, base_url.clone(), ttl).await;
    Some(base_url)
}

fn discovery_cache_key(realm: &str, service: &str) -> String {
    format!("rhelma:discovery:{realm}:{service}")
}

fn clamp_discovery_ttl_secs(ttl: Duration) -> u64 {
    ttl.as_secs().clamp(5, 300)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovery_cache_key_is_stable() {
        assert_eq!(
            discovery_cache_key("central", "social-service"),
            "rhelma:discovery:central:social-service"
        );
    }

    #[test]
    fn clamp_discovery_ttl_is_bounded() {
        assert_eq!(clamp_discovery_ttl_secs(Duration::from_secs(1)), 5);
        assert_eq!(clamp_discovery_ttl_secs(Duration::from_secs(5)), 5);
        assert_eq!(clamp_discovery_ttl_secs(Duration::from_secs(30)), 30);
        assert_eq!(clamp_discovery_ttl_secs(Duration::from_secs(9999)), 300);
    }
}
