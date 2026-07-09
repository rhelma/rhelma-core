#![forbid(unsafe_code)]

//! Internal service-to-service authentication (Stage 13).
//!
//! This module authenticates **service→service** calls to internal endpoints
//! (e.g. `GET /internal/capabilities`, `POST /internal/agent/actions/dry-run`).
//! It deliberately does **not** reuse user JWTs: a user bearer token identifies a
//! human principal, not the calling service, and must never be replayed as a
//! service credential.
//!
//! ## Mechanism
//!
//! The caller signs a canonical string with a shared HMAC-SHA256 secret and sends
//! four project-namespaced headers:
//!
//! ```text
//! X-Rhelma-Service      caller service identity, e.g. "agent-service"
//! X-Rhelma-Request-Id   uuid; also used as correlation id
//! X-Rhelma-Timestamp    unix seconds; a replay window is enforced
//! X-Rhelma-Signature    base64url(HMAC-SHA256(secret, canonical_string))
//! ```
//!
//! The canonical string binds the caller identity, request id, timestamp, and the
//! HTTP method + path so a captured signature cannot be replayed against a
//! different route:
//!
//! ```text
//! v1\n{service}\n{request_id}\n{timestamp}\n{METHOD}\n{path}
//! ```
//!
//! HMAC + constant-time verification reuse the vetted workspace primitives in
//! [`rhelma_core::governance::crypto`] and [`rhelma_core::security`]; no new
//! crypto is introduced.
//!
//! ## Trust posture
//!
//! - Unknown caller service ⇒ rejected.
//! - Missing any required header ⇒ rejected.
//! - Bad signature ⇒ rejected (constant-time compare).
//! - Timestamp outside the allowed skew window ⇒ rejected (replay protection).
//! - Secrets are never logged (only service names and reason codes are).
//! - A verifier with **no** configured identities fails **closed**: every request
//!   is rejected unless dev-insecure mode is explicitly enabled for non-prod.

use std::collections::BTreeMap;

use secrecy::{ExposeSecret, Secret};

use rhelma_core::governance::crypto::{hs256_sign, hs256_verify_b64url};

/// Header names for the internal-auth envelope. Kept as constants so callers and
/// verifiers never drift.
pub mod headers {
    /// Caller service identity, e.g. `agent-service`.
    pub const SERVICE: &str = "x-rhelma-service";
    /// Unique per-request id (uuid); doubles as the correlation id.
    pub const REQUEST_ID: &str = "x-rhelma-request-id";
    /// Unix seconds when the request was signed.
    pub const TIMESTAMP: &str = "x-rhelma-timestamp";
    /// base64url HMAC-SHA256 signature over the canonical string.
    pub const SIGNATURE: &str = "x-rhelma-signature";
}

const CANONICAL_VERSION: &str = "v1";
/// Default allowed clock skew / replay window, in seconds.
pub const DEFAULT_MAX_SKEW_SECONDS: u64 = 300;

/// A named service credential: identity + shared HMAC secret.
///
/// The secret is held in [`Secret`] so it is not accidentally logged or included
/// in `Debug` output.
#[derive(Clone)]
pub struct ServiceIdentity {
    service_name: String,
    secret: Secret<String>,
}

impl std::fmt::Debug for ServiceIdentity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never render the secret.
        f.debug_struct("ServiceIdentity")
            .field("service_name", &self.service_name)
            .field("secret", &"<redacted>")
            .finish()
    }
}

impl ServiceIdentity {
    /// Build an identity from a service name and shared secret.
    pub fn new(service_name: impl Into<String>, secret: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
            secret: Secret::new(secret.into()),
        }
    }

    /// The caller's service name (e.g. `agent-service`).
    pub fn service_name(&self) -> &str {
        &self.service_name
    }

    fn secret_bytes(&self) -> &[u8] {
        self.secret.expose_secret().as_bytes()
    }
}

/// Build the canonical string that is HMAC-signed. Path should be the request
/// path only (no query string); method is upper-cased.
fn canonical_string(
    service: &str,
    request_id: &str,
    timestamp: u64,
    method: &str,
    path: &str,
) -> String {
    format!(
        "{CANONICAL_VERSION}\n{service}\n{request_id}\n{timestamp}\n{}\n{}",
        method.to_ascii_uppercase(),
        path
    )
}

/// The four headers a signer produces, ready to attach to an outbound request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedHeaders {
    /// `X-Rhelma-Service`
    pub service: String,
    /// `X-Rhelma-Request-Id`
    pub request_id: String,
    /// `X-Rhelma-Timestamp`
    pub timestamp: String,
    /// `X-Rhelma-Signature`
    pub signature: String,
}

impl SignedHeaders {
    /// Iterate as `(header_name, value)` pairs for attaching to a request.
    pub fn as_pairs(&self) -> [(&'static str, &str); 4] {
        [
            (headers::SERVICE, self.service.as_str()),
            (headers::REQUEST_ID, self.request_id.as_str()),
            (headers::TIMESTAMP, self.timestamp.as_str()),
            (headers::SIGNATURE, self.signature.as_str()),
        ]
    }
}

/// Signs outbound internal requests as a specific service identity.
#[derive(Debug, Clone)]
pub struct InternalRequestSigner {
    identity: ServiceIdentity,
}

impl InternalRequestSigner {
    /// Create a signer for the given identity.
    pub fn new(identity: ServiceIdentity) -> Self {
        Self { identity }
    }

    /// The service name this signer authenticates as.
    pub fn service_name(&self) -> &str {
        self.identity.service_name()
    }

    /// Produce signed headers for a request. `request_id` and `timestamp` are
    /// supplied by the caller so the correlation id can be threaded from the
    /// existing request context (and so signing is deterministic/testable).
    pub fn sign(
        &self,
        request_id: &str,
        timestamp: u64,
        method: &str,
        path: &str,
    ) -> SignedHeaders {
        let canonical = canonical_string(
            self.identity.service_name(),
            request_id,
            timestamp,
            method,
            path,
        );
        // hs256_sign only returns None for an invalid key length; HMAC-SHA256
        // accepts any key length, so this is infallible in practice.
        let sig = hs256_sign(canonical.as_bytes(), self.identity.secret_bytes())
            .map(|bytes| {
                use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
                URL_SAFE_NO_PAD.encode(bytes)
            })
            .unwrap_or_default();
        SignedHeaders {
            service: self.identity.service_name().to_string(),
            request_id: request_id.to_string(),
            timestamp: timestamp.to_string(),
            signature: sig,
        }
    }
}

/// Why an internal request was rejected. Distinct variants let handlers map to
/// safe status codes without leaking which check failed to the caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InternalAuthError {
    /// The verifier has no identities and is not in dev-insecure mode.
    NotConfigured,
    /// A required header was missing or unparseable.
    MissingCredentials,
    /// `X-Rhelma-Service` names a service the verifier does not know.
    UnknownService,
    /// Timestamp was outside the allowed skew window (possible replay).
    ExpiredTimestamp,
    /// Signature did not verify against the caller's secret.
    InvalidSignature,
}

impl InternalAuthError {
    /// Short, non-sensitive reason code for logs/metrics.
    pub fn reason_code(&self) -> &'static str {
        match self {
            Self::NotConfigured => "internal_auth_not_configured",
            Self::MissingCredentials => "missing_internal_credentials",
            Self::UnknownService => "unknown_service",
            Self::ExpiredTimestamp => "expired_timestamp",
            Self::InvalidSignature => "invalid_signature",
        }
    }

    /// `true` when the failure is a server-side misconfiguration (⇒ 503) rather
    /// than a client authentication failure (⇒ 401).
    pub fn is_configuration_error(&self) -> bool {
        matches!(self, Self::NotConfigured)
    }
}

/// The verified caller identity returned on success.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedCaller {
    /// The authenticated caller service name.
    pub service_name: String,
    /// The caller-supplied request id (usable as a correlation id).
    pub request_id: String,
}

/// Verifies inbound internal requests. Holds the set of known caller identities
/// (keyed by service name) plus policy (skew window, dev-insecure).
#[derive(Clone)]
pub struct InternalRequestVerifier {
    identities: BTreeMap<String, ServiceIdentity>,
    max_skew_seconds: u64,
    /// Non-production escape hatch: when `true` and no identities are configured,
    /// requests are allowed through as an anonymous dev caller. Never set this in
    /// production.
    dev_insecure: bool,
}

impl std::fmt::Debug for InternalRequestVerifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InternalRequestVerifier")
            .field("identities", &self.identities.keys().collect::<Vec<_>>())
            .field("max_skew_seconds", &self.max_skew_seconds)
            .field("dev_insecure", &self.dev_insecure)
            .finish()
    }
}

impl InternalRequestVerifier {
    /// Build a verifier from a set of allowed caller identities.
    pub fn new(identities: impl IntoIterator<Item = ServiceIdentity>) -> Self {
        let identities = identities
            .into_iter()
            .map(|identity| (identity.service_name().to_string(), identity))
            .collect();
        Self {
            identities,
            max_skew_seconds: DEFAULT_MAX_SKEW_SECONDS,
            dev_insecure: false,
        }
    }

    /// Override the allowed clock-skew / replay window (seconds).
    pub fn with_max_skew_seconds(mut self, seconds: u64) -> Self {
        if seconds > 0 {
            self.max_skew_seconds = seconds;
        }
        self
    }

    /// Enable the non-production dev-insecure escape hatch. Only takes effect when
    /// no identities are configured; a verifier that *has* identities always
    /// enforces them regardless of this flag.
    pub fn with_dev_insecure(mut self, dev_insecure: bool) -> Self {
        self.dev_insecure = dev_insecure;
        self
    }

    /// `true` when this verifier can authenticate at least one caller.
    pub fn is_enforcing(&self) -> bool {
        !self.identities.is_empty()
    }

    /// Number of known caller identities.
    pub fn identity_count(&self) -> usize {
        self.identities.len()
    }

    /// Verify a request from raw header lookups. `now` is unix seconds (passed in
    /// so verification is testable and deterministic). `header` returns the value
    /// for a lowercase header name.
    pub fn verify(
        &self,
        method: &str,
        path: &str,
        now: u64,
        header: impl Fn(&str) -> Option<String>,
    ) -> Result<VerifiedCaller, InternalAuthError> {
        // Fail closed unless explicitly configured. Dev-insecure only applies when
        // no identities exist (so it can never *weaken* an enforcing verifier).
        if !self.is_enforcing() {
            if self.dev_insecure {
                return Ok(VerifiedCaller {
                    service_name: "dev-insecure".to_string(),
                    request_id: header(headers::REQUEST_ID).unwrap_or_default(),
                });
            }
            return Err(InternalAuthError::NotConfigured);
        }

        let service = header(headers::SERVICE).ok_or(InternalAuthError::MissingCredentials)?;
        let request_id =
            header(headers::REQUEST_ID).ok_or(InternalAuthError::MissingCredentials)?;
        let timestamp_raw =
            header(headers::TIMESTAMP).ok_or(InternalAuthError::MissingCredentials)?;
        let signature = header(headers::SIGNATURE).ok_or(InternalAuthError::MissingCredentials)?;

        if service.is_empty()
            || request_id.is_empty()
            || timestamp_raw.is_empty()
            || signature.is_empty()
        {
            return Err(InternalAuthError::MissingCredentials);
        }

        let timestamp: u64 = timestamp_raw
            .trim()
            .parse()
            .map_err(|_| InternalAuthError::MissingCredentials)?;

        // Unknown service is rejected before any secret work.
        let identity = self
            .identities
            .get(&service)
            .ok_or(InternalAuthError::UnknownService)?;

        // Replay window: reject timestamps too far in the past or future.
        let skew = now.abs_diff(timestamp);
        if skew > self.max_skew_seconds {
            return Err(InternalAuthError::ExpiredTimestamp);
        }

        let canonical = canonical_string(&service, &request_id, timestamp, method, path);
        if !hs256_verify_b64url(canonical.as_bytes(), &signature, identity.secret_bytes()) {
            return Err(InternalAuthError::InvalidSignature);
        }

        Ok(VerifiedCaller {
            service_name: service,
            request_id,
        })
    }
}

/// Current unix time in seconds. Isolated so tests can pass a fixed `now`.
pub fn now_unix_seconds() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Verify an inbound request expressed with `http` crate types.
///
/// This is the transport-level bridge used by per-service axum middleware: axum
/// re-exports `http::HeaderMap`/`http::Method`, so a service guard can call this
/// directly with the request's method, path, and headers. `now` defaults to the
/// system clock via [`now_unix_seconds`].
pub fn verify_http_request(
    verifier: &InternalRequestVerifier,
    method: &http::Method,
    path: &str,
    headers: &http::HeaderMap,
) -> Result<VerifiedCaller, InternalAuthError> {
    verifier.verify(method.as_str(), path, now_unix_seconds(), |name| {
        headers
            .get(name)
            .and_then(|value| value.to_str().ok())
            .map(str::to_string)
    })
}

/// Environment-driven construction of verifiers and signers, so every service
/// wires internal auth identically.
///
/// Shared secret env vars (checked in order):
/// - `RHELMA_INTERNAL_AUTH_SECRET` — a single shared secret used for the
///   default allowed caller (`agent-service`). This is the common case.
/// - `RHELMA_INTERNAL_AUTH_SECRETS` — optional `name=secret` pairs separated by
///   commas or whitespace, to authorize additional callers (e.g.
///   `agent-service=abc,control-service=def`).
///
/// Policy env vars:
/// - `RHELMA_INTERNAL_AUTH_MAX_SKEW_SECONDS` — replay window (default 300).
/// - `RHELMA_INTERNAL_AUTH_DEV_INSECURE` — when truthy **and** not production,
///   allow unauthenticated internal calls (dev/test only).
pub mod env {
    use super::{InternalRequestSigner, InternalRequestVerifier, ServiceIdentity};

    /// The caller identity assumed for a bare single-secret configuration.
    pub const DEFAULT_ALLOWED_CALLER: &str = "agent-service";

    fn env_var(name: &str) -> Option<String> {
        std::env::var(name).ok().and_then(|v| {
            let v = v.trim().to_string();
            if v.is_empty() {
                None
            } else {
                Some(v)
            }
        })
    }

    fn truthy(value: &str) -> bool {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    }

    /// Parse `name=secret` pairs separated by commas/whitespace/newlines.
    fn parse_pairs(raw: &str) -> Vec<ServiceIdentity> {
        raw.split([',', '\n', '\r', ' ', '\t'])
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .filter_map(|entry| {
                let (name, secret) = entry.split_once('=')?;
                let name = name.trim();
                let secret = secret.trim();
                if name.is_empty() || secret.is_empty() {
                    return None;
                }
                Some(ServiceIdentity::new(name, secret))
            })
            .collect()
    }

    /// Collect configured caller identities from the environment.
    pub fn identities_from_env() -> Vec<ServiceIdentity> {
        let mut identities = Vec::new();
        if let Some(secret) = env_var("RHELMA_INTERNAL_AUTH_SECRET") {
            identities.push(ServiceIdentity::new(DEFAULT_ALLOWED_CALLER, secret));
        }
        if let Some(raw) = env_var("RHELMA_INTERNAL_AUTH_SECRETS") {
            identities.extend(parse_pairs(&raw));
        }
        identities
    }

    /// Whether the non-production dev-insecure escape hatch is requested.
    pub fn dev_insecure_requested() -> bool {
        env_var("RHELMA_INTERNAL_AUTH_DEV_INSECURE")
            .map(|v| truthy(&v))
            .unwrap_or(false)
    }

    fn max_skew_from_env() -> Option<u64> {
        env_var("RHELMA_INTERNAL_AUTH_MAX_SKEW_SECONDS").and_then(|v| v.parse::<u64>().ok())
    }

    /// Build a verifier for an inbound-protecting service.
    ///
    /// `is_production` gates the dev-insecure hatch: it is honored only when the
    /// environment is non-production, so a stray env var can never open an
    /// internal surface in prod.
    pub fn verifier_from_env(is_production: bool) -> InternalRequestVerifier {
        let identities = identities_from_env();
        let dev_insecure = !is_production && dev_insecure_requested();
        let mut verifier = InternalRequestVerifier::new(identities).with_dev_insecure(dev_insecure);
        if let Some(skew) = max_skew_from_env() {
            verifier = verifier.with_max_skew_seconds(skew);
        }
        verifier
    }

    /// Build a signer for an outbound service (e.g. agent-service), if a secret
    /// is configured for it. Looks up the caller's own name in the pair list
    /// first, then falls back to the single shared secret.
    pub fn signer_from_env(service_name: &str) -> Option<InternalRequestSigner> {
        // Prefer an explicit pair for this service.
        if let Some(raw) = env_var("RHELMA_INTERNAL_AUTH_SECRETS") {
            if let Some(identity) = parse_pairs(&raw)
                .into_iter()
                .find(|id| id.service_name() == service_name)
            {
                return Some(InternalRequestSigner::new(identity));
            }
        }
        // Fall back to the shared single secret, signing as this service.
        env_var("RHELMA_INTERNAL_AUTH_SECRET")
            .map(|secret| InternalRequestSigner::new(ServiceIdentity::new(service_name, secret)))
    }

    #[cfg(test)]
    mod env_tests {
        use super::*;

        #[test]
        fn parse_pairs_reads_multiple_callers() {
            let ids = parse_pairs("agent-service=aaa, control-service=bbb");
            let names: Vec<_> = ids.iter().map(|i| i.service_name().to_string()).collect();
            assert_eq!(names, vec!["agent-service", "control-service"]);
        }

        #[test]
        fn parse_pairs_skips_malformed_entries() {
            let ids = parse_pairs("agent-service=aaa,,broken,=nosecret,name=");
            assert_eq!(ids.len(), 1);
            assert_eq!(ids[0].service_name(), "agent-service");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn signer() -> InternalRequestSigner {
        InternalRequestSigner::new(ServiceIdentity::new("agent-service", "super-secret-key"))
    }

    fn verifier() -> InternalRequestVerifier {
        InternalRequestVerifier::new([ServiceIdentity::new("agent-service", "super-secret-key")])
    }

    /// Turn signed headers into a lookup closure, optionally with overrides.
    fn lookup(h: &SignedHeaders) -> impl Fn(&str) -> Option<String> + '_ {
        move |name: &str| match name {
            headers::SERVICE => Some(h.service.clone()),
            headers::REQUEST_ID => Some(h.request_id.clone()),
            headers::TIMESTAMP => Some(h.timestamp.clone()),
            headers::SIGNATURE => Some(h.signature.clone()),
            _ => None,
        }
    }

    #[test]
    fn valid_signature_accepted() {
        let signed = signer().sign("req-1", 1_000, "GET", "/internal/capabilities");
        let caller = verifier()
            .verify("GET", "/internal/capabilities", 1_010, lookup(&signed))
            .unwrap();
        assert_eq!(caller.service_name, "agent-service");
        assert_eq!(caller.request_id, "req-1");
    }

    #[test]
    fn missing_credentials_rejected() {
        let err = verifier()
            .verify("GET", "/internal/capabilities", 1_000, |_| None)
            .unwrap_err();
        assert_eq!(err, InternalAuthError::MissingCredentials);
    }

    #[test]
    fn unknown_service_rejected() {
        let other = InternalRequestSigner::new(ServiceIdentity::new("evil-service", "whatever"));
        let signed = other.sign("req-1", 1_000, "GET", "/internal/capabilities");
        let err = verifier()
            .verify("GET", "/internal/capabilities", 1_000, lookup(&signed))
            .unwrap_err();
        assert_eq!(err, InternalAuthError::UnknownService);
    }

    #[test]
    fn wrong_secret_rejected() {
        // Same service name, different secret than the verifier expects.
        let imposter =
            InternalRequestSigner::new(ServiceIdentity::new("agent-service", "wrong-secret"));
        let signed = imposter.sign("req-1", 1_000, "GET", "/internal/capabilities");
        let err = verifier()
            .verify("GET", "/internal/capabilities", 1_000, lookup(&signed))
            .unwrap_err();
        assert_eq!(err, InternalAuthError::InvalidSignature);
    }

    #[test]
    fn expired_timestamp_rejected() {
        let signed = signer().sign("req-1", 1_000, "GET", "/internal/capabilities");
        // now is far past the signed timestamp + default window.
        let err = verifier()
            .verify(
                "GET",
                "/internal/capabilities",
                1_000 + 10_000,
                lookup(&signed),
            )
            .unwrap_err();
        assert_eq!(err, InternalAuthError::ExpiredTimestamp);
    }

    #[test]
    fn future_timestamp_rejected() {
        let signed = signer().sign("req-1", 1_000_000, "GET", "/internal/capabilities");
        let err = verifier()
            .verify("GET", "/internal/capabilities", 1_000, lookup(&signed))
            .unwrap_err();
        assert_eq!(err, InternalAuthError::ExpiredTimestamp);
    }

    #[test]
    fn tampered_path_rejected() {
        // Signature bound to method+path; replaying on another route fails.
        let signed = signer().sign("req-1", 1_000, "POST", "/internal/agent/actions/dry-run");
        let err = verifier()
            .verify("GET", "/internal/capabilities", 1_010, lookup(&signed))
            .unwrap_err();
        assert_eq!(err, InternalAuthError::InvalidSignature);
    }

    #[test]
    fn empty_verifier_fails_closed() {
        let v = InternalRequestVerifier::new(std::iter::empty());
        let err = v
            .verify("GET", "/internal/capabilities", 1_000, |_| None)
            .unwrap_err();
        assert_eq!(err, InternalAuthError::NotConfigured);
        assert!(err.is_configuration_error());
    }

    #[test]
    fn dev_insecure_only_when_no_identities() {
        // No identities + dev_insecure ⇒ allowed as dev caller.
        let v = InternalRequestVerifier::new(std::iter::empty()).with_dev_insecure(true);
        let caller = v
            .verify("GET", "/internal/capabilities", 1_000, |_| None)
            .unwrap();
        assert_eq!(caller.service_name, "dev-insecure");

        // dev_insecure does NOT weaken an enforcing verifier: unsigned request
        // still rejected.
        let v2 = verifier().with_dev_insecure(true);
        let err = v2
            .verify("GET", "/internal/capabilities", 1_000, |_| None)
            .unwrap_err();
        assert_eq!(err, InternalAuthError::MissingCredentials);
    }

    #[test]
    fn signer_produces_four_pairs() {
        let signed = signer().sign("req-1", 1_000, "GET", "/internal/capabilities");
        let pairs = signed.as_pairs();
        assert_eq!(pairs.len(), 4);
        assert_eq!(pairs[0].0, headers::SERVICE);
        assert_eq!(pairs[0].1, "agent-service");
    }

    #[test]
    fn debug_never_leaks_secret() {
        let identity = ServiceIdentity::new("agent-service", "top-secret-value");
        let rendered = format!("{identity:?}");
        assert!(!rendered.contains("top-secret-value"));
        assert!(rendered.contains("redacted"));
    }
}
