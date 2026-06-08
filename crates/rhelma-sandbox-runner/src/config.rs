#![forbid(unsafe_code)]

use std::env;

#[derive(Debug, Clone)]
/// struct (documented for contract compliance).
pub struct SandboxRunnerConfig {
    /// Field `docker_enabled`.
    pub docker_enabled: bool,
    /// Field `docker_image`.
    pub docker_image: String,
    /// Field `workspace_root`.
    pub workspace_root: String,
    /// Field `max_patch_bytes`.
    pub max_patch_bytes: usize,
    /// Field `command_timeout_ms`.
    pub command_timeout_ms: u64,
    /// Field `allowed_command_prefixes`.
    pub allowed_command_prefixes: Vec<String>,

    /// Allowed path prefixes for files changed by a patch (comma-separated).
    pub allowed_path_prefixes: Vec<String>,

    /// Forbidden path prefixes for files changed by a patch (comma-separated).
    pub forbidden_path_prefixes: Vec<String>,

    /// Branch prefix used when creating an apply branch (e.g. `ai/improve`).
    pub apply_branch_prefix: String,

    /// Git remote used for pushing (default `origin`).
    pub apply_git_remote: String,

    /// Whether `git push` is enabled for apply operations (default false).
    pub apply_push_enabled: bool,

    /// Whether to fetch the remote branch before applying (improves idempotency when push is enabled).
    pub apply_fetch_remote: bool,

    /// Rollback branch prefix used when creating a rollback branch (e.g. `ai/rollback`).
    pub rollback_branch_prefix: String,

    /// Base branch used for rollback operations (default `main`).
    pub rollback_base_branch: String,

    /// Git remote used for pushing rollback branches (default `origin`).
    pub rollback_git_remote: String,

    /// Whether `git push` is enabled for rollback operations (default false).
    pub rollback_push_enabled: bool,

    /// Whether to fetch remotes for rollback operations (default true).
    pub rollback_fetch_remote: bool,

    /// Preferred: comma-separated key ring: `kid1:secret1,kid2:secret2`.
    /// If set, this overrides `attestation_hmac_secret`.
    pub attestation_hmac_keys: Option<String>,

    /// Optional primary key id (used for signing when `attestation_hmac_keys` is set).
    pub attestation_primary_kid: Option<String>,

    /// Legacy single secret used to sign/verify evaluation attestations (HS256).
    /// If unset, no signing is performed.
    pub attestation_hmac_secret: Option<String>,

    /// Optional key id for legacy `attestation_hmac_secret`.
    pub attestation_kid: Option<String>,

    /// If true, apply requests must carry a valid evaluation attestation.
    pub attestation_required: bool,
}

impl Default for SandboxRunnerConfig {
    fn default() -> Self {
        Self {
            docker_enabled: false,
            docker_image: "rust:1.91".to_string(),
            workspace_root: ".".to_string(),
            max_patch_bytes: 200_000,
            command_timeout_ms: 600_000,
            allowed_command_prefixes: vec![
                "git apply".to_string(),
                "cargo fmt".to_string(),
                "cargo clippy".to_string(),
                "cargo test".to_string(),
                "cargo check".to_string(),
            ],
            allowed_path_prefixes: vec![
                "apps/".to_string(),
                "crates/".to_string(),
                "observability/".to_string(),
            ],
            forbidden_path_prefixes: vec![
                ".github/".to_string(),
                "infra/".to_string(),
                "deploy/".to_string(),
                "crates/rhelma-auth".to_string(),
            ],
            apply_branch_prefix: "ai/improve".to_string(),
            apply_git_remote: "origin".to_string(),
            apply_push_enabled: false,
            apply_fetch_remote: true,
            rollback_branch_prefix: "ai/rollback".to_string(),
            rollback_base_branch: "main".to_string(),
            rollback_git_remote: "origin".to_string(),
            rollback_push_enabled: false,
            rollback_fetch_remote: true,
            attestation_hmac_keys: None,
            attestation_primary_kid: None,
            attestation_hmac_secret: None,
            attestation_kid: None,
            attestation_required: false,
        }
    }
}

impl SandboxRunnerConfig {
    /// fn (documented for contract compliance).
    pub fn from_env() -> Self {
        let mut cfg = Self::default();

        if let Ok(v) = env::var("RHELMA_SANDBOX_RUNNER__DOCKER_ENABLED") {
            cfg.docker_enabled = matches!(
                v.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "y" | "on"
            );
        }
        if let Ok(v) = env::var("RHELMA_SANDBOX_RUNNER__DOCKER_IMAGE") {
            if !v.trim().is_empty() {
                cfg.docker_image = v;
            }
        }
        if let Ok(v) = env::var("RHELMA_SANDBOX_RUNNER__WORKSPACE_ROOT") {
            if !v.trim().is_empty() {
                cfg.workspace_root = v;
            }
        }
        if let Ok(v) = env::var("RHELMA_SANDBOX_RUNNER__MAX_PATCH_BYTES") {
            if let Ok(n) = v.trim().parse::<usize>() {
                cfg.max_patch_bytes = n;
            }
        }
        if let Ok(v) = env::var("RHELMA_SANDBOX_RUNNER__COMMAND_TIMEOUT_MS") {
            if let Ok(n) = v.trim().parse::<u64>() {
                cfg.command_timeout_ms = n;
            }
        }
        if let Ok(v) = env::var("RHELMA_SANDBOX_RUNNER__ALLOWED_COMMAND_PREFIXES") {
            let parts: Vec<String> = v
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if !parts.is_empty() {
                cfg.allowed_command_prefixes = parts;
            }
        }

        if let Ok(v) = env::var("RHELMA_SANDBOX_RUNNER__ALLOWED_PATH_PREFIXES") {
            let parts: Vec<String> = v
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if !parts.is_empty() {
                cfg.allowed_path_prefixes = parts;
            }
        }
        if let Ok(v) = env::var("RHELMA_SANDBOX_RUNNER__FORBIDDEN_PATH_PREFIXES") {
            let parts: Vec<String> = v
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if !parts.is_empty() {
                cfg.forbidden_path_prefixes = parts;
            }
        }
        if let Ok(v) = env::var("RHELMA_SANDBOX_RUNNER__APPLY_BRANCH_PREFIX") {
            if !v.trim().is_empty() {
                cfg.apply_branch_prefix = v;
            }
        }
        if let Ok(v) = env::var("RHELMA_SANDBOX_RUNNER__APPLY_GIT_REMOTE") {
            if !v.trim().is_empty() {
                cfg.apply_git_remote = v;
            }
        }

        if let Ok(v) = env::var("RHELMA_SANDBOX_RUNNER__APPLY_PUSH_ENABLED") {
            cfg.apply_push_enabled = matches!(
                v.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "y" | "on"
            );
        }
        if let Ok(v) = env::var("RHELMA_SANDBOX_RUNNER__APPLY_FETCH_REMOTE") {
            cfg.apply_fetch_remote = matches!(
                v.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "y" | "on"
            );
        }

        if let Ok(v) = env::var("RHELMA_SANDBOX_RUNNER__ROLLBACK_BRANCH_PREFIX") {
            if !v.trim().is_empty() {
                cfg.rollback_branch_prefix = v;
            }
        }
        if let Ok(v) = env::var("RHELMA_SANDBOX_RUNNER__ROLLBACK_BASE_BRANCH") {
            if !v.trim().is_empty() {
                cfg.rollback_base_branch = v;
            }
        }
        if let Ok(v) = env::var("RHELMA_SANDBOX_RUNNER__ROLLBACK_GIT_REMOTE") {
            if !v.trim().is_empty() {
                cfg.rollback_git_remote = v;
            }
        }
        if let Ok(v) = env::var("RHELMA_SANDBOX_RUNNER__ROLLBACK_PUSH_ENABLED") {
            cfg.rollback_push_enabled = matches!(
                v.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "y" | "on"
            );
        }
        if let Ok(v) = env::var("RHELMA_SANDBOX_RUNNER__ROLLBACK_FETCH_REMOTE") {
            cfg.rollback_fetch_remote = matches!(
                v.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "y" | "on"
            );
        }

        // Shared attestation settings (recommended to be set identically across sandbox-runner + patch-applier).
        if let Ok(v) = env::var("RHELMA_AI_ATTESTATION__HMAC_KEYS") {
            if !v.trim().is_empty() {
                cfg.attestation_hmac_keys = Some(v);
            }
        }
        if let Ok(v) = env::var("RHELMA_AI_ATTESTATION__PRIMARY_KID") {
            if !v.trim().is_empty() {
                cfg.attestation_primary_kid = Some(v);
            }
        }
        if let Ok(v) = env::var("RHELMA_AI_ATTESTATION__HMAC_SECRET") {
            if !v.trim().is_empty() {
                cfg.attestation_hmac_secret = Some(v);
            }
        }
        if let Ok(v) = env::var("RHELMA_AI_ATTESTATION__KID") {
            if !v.trim().is_empty() {
                cfg.attestation_kid = Some(v);
            }
        }
        if let Ok(v) = env::var("RHELMA_SANDBOX_RUNNER__ATTESTATION_REQUIRED") {
            cfg.attestation_required = matches!(
                v.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "y" | "on"
            );
        }

        cfg
    }
}
