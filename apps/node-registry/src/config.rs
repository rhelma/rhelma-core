#![forbid(unsafe_code)]

use std::env;
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;
use std::time::Duration;

use rhelma_config::{CentralEnv, CoreConfig};
use rhelma_core::RhelmaError;

use crate::error::RegistryError;

#[derive(Debug, Clone)]
pub struct NodeRegistryTuning {
    /// Field `node_ttl`.
    pub node_ttl: Duration,
    /// Field `prune_interval`.
    pub prune_interval: Duration,
    /// Field `max_nodes`.
    pub max_nodes: usize,
}

#[derive(Debug, Clone)]
pub struct NodeRegistryPolicy {
    /// Default minimum reputation required when discover() does not specify a filter.
    pub default_min_reputation: i32,
    /// Default attestation requirement when discover() does not specify a filter.
    pub default_require_attested: bool,

    /// Require manifest signature_hex at registration time.
    pub require_manifest_signature: bool,
    /// Require attestation evidence when attestation.kind != "none".
    pub require_attestation_evidence: bool,

    /// Require cryptographic/remote verification for attestation evidence.
    ///
    /// When enabled:
    /// - `software` attestation must include a verifiable signature (or pass the external verifier)
    /// - `tpm`/`sgx` attestation must pass the external verifier (or a compiled-in verifier when available)
    pub require_attestation_verification: bool,

    /// Optional external verifier command for attestation evidence.
    ///
    /// When set, node-registry will invoke this command (best-effort) to verify attestation
    /// evidence.
    ///
    /// Invocation contract:
    /// - argv: `<cmd> verify <kind> <node_id>`
    /// - stdin: raw evidence string (UTF-8)
    /// - exit code 0 => verified; non-zero => rejected (when `require_attestation_verification=true`)
    pub attestation_verify_cmd: Option<String>,

    /// Optional allowlist for TPM PCR values.
    ///
    /// Format: JSON object mapping PCR index (number or string) -> expected SHA-256 hex.
    /// Example: {"0": "abc...", "7": "def..."}
    pub tpm_pcr_allowlist_json: Option<String>,

    /// Optional allowlist for SGX MRENCLAVE values.
    ///
    /// Format: comma-separated hex strings or a JSON array of hex strings.
    pub sgx_mrenclave_allowlist: Option<String>,

    /// Reputation threshold to promote a node into `active` scheduling status.
    pub promote_threshold: i32,
    /// Reputation threshold below which the node is suspended.
    pub suspend_threshold: i32,
    /// Suspension duration in seconds.
    pub suspend_seconds: u64,

    /// Reputation deltas.
    pub delta_ok: i32,
    /// Field `delta_fail`.
    pub delta_fail: i32,
    /// Field `delta_timeout`.
    pub delta_timeout: i32,
    /// Field `delta_bad_result`.
    pub delta_bad_result: i32,

    /// Clamp bounds.
    pub min_reputation: i32,
    /// Field `max_reputation`.
    pub max_reputation: i32,
}

#[derive(Debug, Clone)]
pub struct NodeRegistryAdmission {
    /// Field `pow_enabled`.
    pub pow_enabled: bool,
    /// Field `pow_difficulty_bits`.
    pub pow_difficulty_bits: u8,
    /// Field `pow_challenge_ttl`.
    pub pow_challenge_ttl: Duration,

    /// Field `register_rate_limit_max`.
    pub register_rate_limit_max: u32,
    /// Field `register_rate_limit_ttl`.
    pub register_rate_limit_ttl: Duration,

    /// Optional Redis URL to store PoW challenges and rate-limit counters.
    /// If unset, admission state is kept in memory and is not shared between instances.
    pub redis_url: Option<String>,

    /// Prefix used for Redis keys when `redis_url` is set.
    pub redis_prefix: String,
}

#[derive(Clone)]
pub struct NodeRegistryConfig {
    /// Field `central`.
    pub central: CentralEnv,
    /// Field `core`.
    pub core: CoreConfig,

    /// Field `service_name`.
    pub service_name: String,
    /// Field `bind_host`.
    pub bind_host: IpAddr,
    /// Field `bind_port`.
    pub bind_port: u16,

    /// Optional internal bind address for `/v1/internal/*` endpoints.
    ///
    /// When set, internal/admin routes can be hosted on a separate listener to
    /// improve isolation (e.g. `127.0.0.1:9090`).
    pub internal_bind: Option<SocketAddr>,

    /// Field `tuning`.
    pub tuning: NodeRegistryTuning,

    /// Phase 4 trust-gating configuration.
    pub policy: NodeRegistryPolicy,

    /// Phase 46 admission controls for permissionless onboarding.
    pub admission: NodeRegistryAdmission,

    /// Optional admin token required for internal report endpoints.
    /// If unset, internal endpoints are disabled.
    pub admin_token: Option<String>,
}

impl NodeRegistryConfig {
    pub fn load() -> Result<Self, RegistryError> {
        Self::from_env()
    }

    pub fn bind_addr(&self) -> Result<SocketAddr, RegistryError> {
        Ok(SocketAddr::new(self.bind_host, self.bind_port))
    }

    #[must_use]
    pub fn internal_bind_addr(&self) -> Option<SocketAddr> {
        self.internal_bind
    }

    pub fn is_prod(&self) -> bool {
        self.central.environment.eq_ignore_ascii_case("production")
    }

    pub fn from_env() -> Result<Self, RegistryError> {
        // CentralEnv is strict & contract-aligned:
        // - RHELMA_ENV or RHELMA_ENVIRONMENT (required)
        // - RHELMA_REGION (required)
        // - RHELMA_SERVICE_VERSION (required)
        let central = CentralEnv::from_env_strict()
            .map_err(|e| RegistryError::from(RhelmaError::Config(e.to_string())))?;
        let core = CoreConfig::from_env(&central)
            .map_err(|e| RegistryError::from(RhelmaError::Config(e.to_string())))?;

        let service_name =
            env::var("RHELMA_SERVICE_NAME").unwrap_or_else(|_| "node-registry".into());
        let bind_host = env::var("RHELMA_BIND_HOST").unwrap_or_else(|_| "0.0.0.0".into());
        let bind_port = env::var("RHELMA_BIND_PORT")
            .ok()
            .and_then(|v| v.parse::<u16>().ok())
            .unwrap_or(8090);

        let internal_bind = env::var("RHELMA_NODE_REGISTRY__INTERNAL_BIND")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .map(|s| {
                s.parse::<SocketAddr>().map_err(|e| {
                    RegistryError::config(format!(
                        "RHELMA_NODE_REGISTRY__INTERNAL_BIND invalid: {e}"
                    ))
                })
            })
            .transpose()?;

        let node_ttl_secs = env::var("RHELMA_NODE_REGISTRY__NODE_TTL_SECONDS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(120);

        let prune_interval_secs = env::var("RHELMA_NODE_REGISTRY__PRUNE_INTERVAL_SECONDS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(30);

        let max_nodes = env::var("RHELMA_NODE_REGISTRY__MAX_NODES")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(100_000);

        // Phase 4 policy (safe defaults)
        let default_min_reputation =
            env::var("RHELMA_NODE_REGISTRY__POLICY__DEFAULT_MIN_REPUTATION")
                .ok()
                .and_then(|v| v.parse::<i32>().ok())
                .unwrap_or(0);

        let default_require_attested =
            env::var("RHELMA_NODE_REGISTRY__POLICY__DEFAULT_REQUIRE_ATTESTED")
                .ok()
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(false);

        let require_manifest_signature =
            env::var("RHELMA_NODE_REGISTRY__POLICY__REQUIRE_MANIFEST_SIGNATURE")
                .ok()
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(false);

        let require_attestation_evidence =
            env::var("RHELMA_NODE_REGISTRY__POLICY__REQUIRE_ATTESTATION_EVIDENCE")
                .ok()
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(false);

        let require_attestation_verification =
            env::var("RHELMA_NODE_REGISTRY__POLICY__REQUIRE_ATTESTATION_VERIFICATION")
                .ok()
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(false);

        let attestation_verify_cmd =
            env::var("RHELMA_NODE_REGISTRY__POLICY__ATTESTATION_VERIFY_CMD")
                .ok()
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty());

        let tpm_pcr_allowlist_json =
            env::var("RHELMA_NODE_REGISTRY__POLICY__TPM_PCR_ALLOWLIST_JSON")
                .ok()
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty());

        let sgx_mrenclave_allowlist =
            env::var("RHELMA_NODE_REGISTRY__POLICY__SGX_MRENCLAVE_ALLOWLIST")
                .ok()
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty());

        let promote_threshold = env::var("RHELMA_NODE_REGISTRY__POLICY__PROMOTE_THRESHOLD")
            .ok()
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(5);

        let suspend_threshold = env::var("RHELMA_NODE_REGISTRY__POLICY__SUSPEND_THRESHOLD")
            .ok()
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(-20);

        let suspend_seconds = env::var("RHELMA_NODE_REGISTRY__POLICY__SUSPEND_SECONDS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(600);

        let delta_ok = env::var("RHELMA_NODE_REGISTRY__POLICY__DELTA_OK")
            .ok()
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(1);

        let delta_fail = env::var("RHELMA_NODE_REGISTRY__POLICY__DELTA_FAIL")
            .ok()
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(-5);

        let delta_timeout = env::var("RHELMA_NODE_REGISTRY__POLICY__DELTA_TIMEOUT")
            .ok()
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(-3);

        let delta_bad_result = env::var("RHELMA_NODE_REGISTRY__POLICY__DELTA_BAD_RESULT")
            .ok()
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(-10);

        let min_reputation = env::var("RHELMA_NODE_REGISTRY__POLICY__MIN_REPUTATION")
            .ok()
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(-100);

        let max_reputation = env::var("RHELMA_NODE_REGISTRY__POLICY__MAX_REPUTATION")
            .ok()
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(1000);

        let admin_token = env::var("RHELMA_NODE_REGISTRY__ADMIN_TOKEN")
            .ok()
            .filter(|s| !s.trim().is_empty());

        let bind_ip = IpAddr::from_str(&bind_host)
            .map_err(|e| RegistryError::config(format!("RHELMA_BIND_HOST invalid: {e}")))?;

        // Hardening: do not allow public binding in production when internal endpoints are enabled
        // AND internal routes are not isolated onto a separate listener, unless explicitly allowed.
        if central.environment.eq_ignore_ascii_case("production")
            && admin_token.is_some()
            && internal_bind.is_none()
            && (bind_ip.is_unspecified())
        {
            let allow = env::var("RHELMA_NODE_REGISTRY__ALLOW_PUBLIC_BIND")
                .or_else(|_| env::var("RHELMA_ALLOW_PUBLIC_BIND"))
                .ok()
                .map(|v| {
                    let v = v.trim();
                    !(v.is_empty() || v == "0" || v.eq_ignore_ascii_case("false"))
                })
                .unwrap_or(false);

            if !allow {
                return Err(RegistryError::config(
                    "Refusing to bind to 0.0.0.0 in production while RHELMA_NODE_REGISTRY__ADMIN_TOKEN is set. \
Set RHELMA_NODE_REGISTRY__ALLOW_PUBLIC_BIND=1 (or RHELMA_ALLOW_PUBLIC_BIND=1) if this is intentional.",
                ));
            }
        }

        // Hardening: internal listener should not be public by default.
        if central.environment.eq_ignore_ascii_case("production") {
            if let Some(ib) = internal_bind {
                if ib.ip().is_unspecified() {
                    let allow = env::var("RHELMA_NODE_REGISTRY__ALLOW_PUBLIC_INTERNAL_BIND")
                        .or_else(|_| env::var("RHELMA_ALLOW_PUBLIC_INTERNAL_BIND"))
                        .ok()
                        .map(|v| {
                            let v = v.trim();
                            !(v.is_empty() || v == "0" || v.eq_ignore_ascii_case("false"))
                        })
                        .unwrap_or(false);
                    if !allow {
                        return Err(RegistryError::config(
                            "Refusing to bind internal listener to 0.0.0.0 in production. Set RHELMA_NODE_REGISTRY__ALLOW_PUBLIC_INTERNAL_BIND=1 if this is intentional.",
                        ));
                    }
                }
            }
        }

        // -----------------------------------------------------------------
        // Phase 46 admission controls (permissionless expansion wiring)
        // -----------------------------------------------------------------
        let pow_enabled = env::var("RHELMA_NODE_REGISTRY__ADMISSION__POW_ENABLED")
            .ok()
            .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
            .unwrap_or(false);

        let pow_difficulty_bits = env::var("RHELMA_NODE_REGISTRY__ADMISSION__POW_DIFFICULTY_BITS")
            .ok()
            .and_then(|v| v.parse::<u8>().ok())
            .unwrap_or(18);

        let pow_challenge_ttl_secs =
            env::var("RHELMA_NODE_REGISTRY__ADMISSION__POW_CHALLENGE_TTL_SECONDS")
                .ok()
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(120);

        let register_rl_max = env::var("RHELMA_NODE_REGISTRY__ADMISSION__REGISTER_RATE_LIMIT_MAX")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(20);

        let register_rl_ttl_secs =
            env::var("RHELMA_NODE_REGISTRY__ADMISSION__REGISTER_RATE_LIMIT_TTL_SECONDS")
                .ok()
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(60);

        Ok(Self {
            central,
            core,
            service_name,
            bind_host: bind_ip,
            bind_port,
            internal_bind,
            tuning: NodeRegistryTuning {
                node_ttl: Duration::from_secs(node_ttl_secs),
                prune_interval: Duration::from_secs(prune_interval_secs),
                max_nodes,
            },

            policy: NodeRegistryPolicy {
                default_min_reputation,
                default_require_attested,
                require_manifest_signature,
                require_attestation_evidence,
                require_attestation_verification,
                attestation_verify_cmd,
                tpm_pcr_allowlist_json,
                sgx_mrenclave_allowlist,
                promote_threshold,
                suspend_threshold,
                suspend_seconds,
                delta_ok,
                delta_fail,
                delta_timeout,
                delta_bad_result,
                min_reputation,
                max_reputation,
            },
            admission: NodeRegistryAdmission {
                pow_enabled,
                pow_difficulty_bits,
                pow_challenge_ttl: Duration::from_secs(pow_challenge_ttl_secs),
                register_rate_limit_max: register_rl_max,
                register_rate_limit_ttl: Duration::from_secs(register_rl_ttl_secs),
                redis_url: env::var("RHELMA_NODE_REGISTRY__ADMISSION__REDIS_URL")
                    .ok()
                    .or_else(|| env::var("RHELMA_REDIS_URL").ok()),
                redis_prefix: env::var("RHELMA_NODE_REGISTRY__ADMISSION__REDIS_PREFIX")
                    .ok()
                    .map(|v| v.trim().to_string())
                    .filter(|v| !v.is_empty())
                    .unwrap_or_else(|| "rhelma:node-registry:admission".to_string()),
            },
            admin_token,
        })
    }
}

#[cfg(test)]
impl std::fmt::Debug for NodeRegistryConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeRegistryConfig")
            .field("central", &self.central)
            .field("core", &self.core)
            .field("service_name", &self.service_name)
            .field("bind_host", &self.bind_host)
            .field("bind_port", &self.bind_port)
            .field("internal_bind", &self.internal_bind)
            .field("tuning", &self.tuning)
            .field("policy", &self.policy)
            .field("admission", &self.admission)
            .field(
                "admin_token",
                &self.admin_token.as_ref().map(|_| "<redacted>"),
            )
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn set_required_central_env() {
        std::env::set_var("RHELMA_REGION", "eu-west-1");
        std::env::set_var("RHELMA_SERVICE_VERSION", "test");
        std::env::set_var("RHELMA_SERVICE_NAME", "node-registry");
        // Core config requires a database URL even when we're only validating bind-policy behavior.
        // Use a dummy value to keep tests hermetic.
        std::env::set_var("RHELMA_DB__URL", "postgres://localhost/dev");
    }

    #[test]
    fn production_refuses_public_bind_when_admin_enabled() {
        let _g = ENV_LOCK.lock().unwrap();

        set_required_central_env();
        std::env::set_var("RHELMA_ENV", "production");
        std::env::set_var("RHELMA_BIND_HOST", "0.0.0.0");
        std::env::set_var("RHELMA_NODE_REGISTRY__ADMIN_TOKEN", "super-secret-token");
        std::env::remove_var("RHELMA_NODE_REGISTRY__ALLOW_PUBLIC_BIND");
        std::env::remove_var("RHELMA_ALLOW_PUBLIC_BIND");

        let err = NodeRegistryConfig::from_env().unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("Refusing to bind"));
    }

    #[test]
    fn production_allows_public_bind_when_explicitly_allowed() {
        let _g = ENV_LOCK.lock().unwrap();

        set_required_central_env();
        std::env::set_var("RHELMA_ENV", "production");
        std::env::set_var("RHELMA_BIND_HOST", "0.0.0.0");
        std::env::set_var("RHELMA_NODE_REGISTRY__ADMIN_TOKEN", "super-secret-token");
        std::env::set_var("RHELMA_NODE_REGISTRY__ALLOW_PUBLIC_BIND", "1");

        let cfg = NodeRegistryConfig::from_env().expect("config");
        assert!(cfg.admin_token.is_some());
        assert!(cfg.bind_host.is_unspecified());
    }
}
