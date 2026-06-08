#![forbid(unsafe_code)]

use std::env;

use serde::{Deserialize, Serialize};

fn env_get(key: &str) -> Option<String> {
    env::var(key).ok().filter(|v| !v.trim().is_empty())
}

fn env_bool(key: &str, default: bool) -> bool {
    env_get(key)
        .map(|v| matches!(v.trim().to_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(default)
}

fn env_usize(key: &str, default: usize) -> usize {
    env_get(key)
        .and_then(|v| v.trim().parse::<usize>().ok())
        .unwrap_or(default)
}

fn env_u64(key: &str, default: u64) -> u64 {
    env_get(key)
        .and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(default)
}

fn env_csv(key: &str, default: &[&str]) -> Vec<String> {
    match env_get(key) {
        Some(v) => v
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
        None => default.iter().map(|s| s.to_string()).collect(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// struct (documented for contract compliance).
pub struct SandboxRunnerConfig {
    /// Field `service_name`.
    pub service_name: String,

    // Kafka
    /// Field `kafka_brokers`.
    pub kafka_brokers: String,
    /// Field `kafka_topic_prefix`.
    pub kafka_topic_prefix: String,
    /// Field `kafka_group_id`.
    pub kafka_group_id: String,

    // Workspace
    /// Field `workspace_root`.
    pub workspace_root: String,

    // Policy
    /// Field `max_patch_bytes`.
    pub max_patch_bytes: usize,
    /// Field `forbidden_path_prefixes`.
    pub forbidden_path_prefixes: Vec<String>,
    /// Field `allowed_command_prefixes`.
    pub allowed_command_prefixes: Vec<String>,
    /// Field `command_timeout_ms`.
    pub command_timeout_ms: u64,
    /// Field `max_log_bytes`.
    pub max_log_bytes: usize,
    /// Field `redacted_env_prefixes`.
    pub redacted_env_prefixes: Vec<String>,

    // Docker isolation
    /// Field `docker_enabled`.
    pub docker_enabled: bool,
    /// Field `docker_image`.
    pub docker_image: String,
    /// Field `docker_network`.
    pub docker_network: String,
    /// Field `docker_cpus`.
    pub docker_cpus: Option<String>,
    /// Field `docker_memory`.
    pub docker_memory: Option<String>,
    /// Field `docker_user`.
    pub docker_user: Option<String>,
    /// Field `docker_extra_args`.
    pub docker_extra_args: Vec<String>,
}

impl SandboxRunnerConfig {
    /// fn (documented for contract compliance).
    pub fn from_env_strict() -> Result<Self, String> {
        let service_name = env_get("RHELMA_SANDBOX_RUNNER__SERVICE_NAME")
            .unwrap_or_else(|| "sandbox-runner".to_string());

        let kafka_brokers = env_get("RHELMA_SANDBOX_RUNNER__KAFKA_BROKERS")
            .ok_or_else(|| "RHELMA_SANDBOX_RUNNER__KAFKA_BROKERS is required".to_string())?;

        let kafka_topic_prefix =
            env_get("RHELMA_SANDBOX_RUNNER__KAFKA_TOPIC_PREFIX").unwrap_or_else(|| "".to_string());

        let kafka_group_id = env_get("RHELMA_SANDBOX_RUNNER__KAFKA_GROUP_ID")
            .unwrap_or_else(|| format!("{}-consumer", service_name));

        let workspace_root = env_get("RHELMA_SANDBOX_RUNNER__WORKSPACE_ROOT")
            .unwrap_or_else(|| ".".to_string());

        let max_patch_bytes = env_usize("RHELMA_SANDBOX_RUNNER__MAX_PATCH_BYTES", 200_000);
        let max_log_bytes = env_usize("RHELMA_SANDBOX_RUNNER__MAX_LOG_BYTES", 32_000);
        let command_timeout_ms = env_u64("RHELMA_SANDBOX_RUNNER__COMMAND_TIMEOUT_MS", 600_000);

        let forbidden_path_prefixes = env_csv(
            "RHELMA_SANDBOX_RUNNER__FORBIDDEN_PATH_PREFIXES",
            &[
                "crates/rhelma-auth",
                "crates/rhelma-core",
                "crates/rhelma-event",
                "crates/rhelma-tracing",
            ],
        );

        let allowed_command_prefixes = env_csv(
            "RHELMA_SANDBOX_RUNNER__ALLOWED_COMMAND_PREFIXES",
            &["cargo fmt", "cargo clippy", "cargo test", "cargo check"],
        );

        let redacted_env_prefixes = env_csv(
            "RHELMA_SANDBOX_RUNNER__REDACTED_ENV_PREFIXES",
            &["OPENAI_API_KEY", "ANTHROPIC_API_KEY", "AWS_SECRET_ACCESS_KEY"],
        );

        let docker_enabled = env_bool("RHELMA_SANDBOX_RUNNER__DOCKER_ENABLED", true);
        let docker_image = env_get("RHELMA_SANDBOX_RUNNER__DOCKER_IMAGE")
            .unwrap_or_else(|| "rust:latest".to_string());
        let docker_network = env_get("RHELMA_SANDBOX_RUNNER__DOCKER_NETWORK")
            .unwrap_or_else(|| "none".to_string());
        let docker_cpus = env_get("RHELMA_SANDBOX_RUNNER__DOCKER_CPUS");
        let docker_memory = env_get("RHELMA_SANDBOX_RUNNER__DOCKER_MEMORY");
        let docker_user = env_get("RHELMA_SANDBOX_RUNNER__DOCKER_USER");
        let docker_extra_args = env_csv("RHELMA_SANDBOX_RUNNER__DOCKER_EXTRA_ARGS", &[]);

        Ok(Self {
            service_name,
            kafka_brokers,
            kafka_topic_prefix,
            kafka_group_id,
            workspace_root,
            max_patch_bytes,
            forbidden_path_prefixes,
            allowed_command_prefixes,
            command_timeout_ms,
            max_log_bytes,
            redacted_env_prefixes,
            docker_enabled,
            docker_image,
            docker_network,
            docker_cpus,
            docker_memory,
            docker_user,
            docker_extra_args,
        })
    }
}
