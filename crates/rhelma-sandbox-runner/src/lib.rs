#![forbid(unsafe_code)]

/// mod (documented for contract compliance).
pub mod config;
/// mod (documented for contract compliance).
pub mod patch_policy;
/// mod (documented for contract compliance).
pub mod runner;

// -----------------------------------------------------------------------------
// Backwards-compatible helpers for older tests/callers.
// -----------------------------------------------------------------------------

use std::time::Duration;
use tokio::process::Command;
use uuid::Uuid;

/// Backwards-compatible sandbox policy type.
///
/// Newer code uses [`config::SandboxRunnerConfig`] and [`runner::SandboxRunner`].
/// This struct is retained so older tests (and potential downstream code) keep
/// compiling.
#[derive(Debug, Clone)]
/// struct (documented for contract compliance).
pub struct SandboxPolicy {
    /// Field `max_patch_bytes`.
    pub max_patch_bytes: usize,
    /// Field `forbidden_path_prefixes`.
    pub forbidden_path_prefixes: Vec<String>,
    /// Field `allowed_command_prefixes`.
    pub allowed_command_prefixes: Vec<String>,
    /// Field `command_timeout`.
    pub command_timeout: Duration,
    /// Field `max_log_bytes`.
    pub max_log_bytes: usize,
    /// Field `redacted_env_prefixes`.
    pub redacted_env_prefixes: Vec<String>,
}

impl SandboxPolicy {
    /// True when `cmd` starts with any allowlisted prefix.
    pub fn is_allowed_cmd(&self, cmd: &str) -> bool {
        let s = cmd.trim();
        self.allowed_command_prefixes
            .iter()
            .any(|p| s.starts_with(p))
    }
}

/// Backwards-compatible helper that extracts changed paths from a unified diff
/// and validates them against forbidden prefixes.
///
/// Returns `Some(reason)` when rejected, otherwise `None`.
pub fn validate_patch_paths(patch: &str, forbidden_prefixes: &[String]) -> Option<String> {
    let paths = patch_policy::changed_paths(patch);
    patch_policy::validate_paths(&paths, &[], forbidden_prefixes).err()
}

/// Command result returned by [`evaluate_patch_plan`].
#[derive(Debug, Clone)]
pub struct PatchPlanCommandResult {
    /// Field `command`.
    pub command: String,
    /// Field `ok`.
    pub ok: bool,
    /// Field `exit_code`.
    pub exit_code: Option<i32>,
    /// Field `duration_ms`.
    pub duration_ms: u64,
    /// Field `stdout_tail`.
    pub stdout_tail: String,
    /// Field `stderr_tail`.
    pub stderr_tail: String,
}

/// Evaluation result returned by [`evaluate_patch_plan`].
#[derive(Debug, Clone)]
pub struct PatchPlanEvaluation {
    /// Field `evaluation_id`.
    pub evaluation_id: String,
    /// Field `ok`.
    pub ok: bool,
    /// Field `message`.
    pub message: String,
    /// Field `results`.
    pub results: Vec<PatchPlanCommandResult>,
}

fn tail_limited(s: String, max_bytes: usize) -> String {
    if max_bytes == 0 {
        return String::new();
    }
    if s.len() <= max_bytes {
        return s;
    }
    // Keep the last N bytes; ensure UTF-8 boundary.
    let mut start = s.len().saturating_sub(max_bytes);
    while start < s.len() && !s.is_char_boundary(start) {
        start += 1;
    }
    s[start..].to_string()
}

/// Evaluate a patch + test plan against a local workspace.
///
/// This helper is designed for *developer workflows* and returns a
/// best-effort result instead of failing the caller with an error.
pub async fn evaluate_patch_plan(
    workspace_root: &str,
    policy: &SandboxPolicy,
    patch: &str,
    test_plan: &[String],
) -> PatchPlanEvaluation {
    let evaluation_id = Uuid::now_v7().to_string();

    // Fast validation.
    if patch.len() > policy.max_patch_bytes {
        return PatchPlanEvaluation {
            evaluation_id,
            ok: false,
            message: format!(
                "patch exceeds max_patch_bytes: {} > {}",
                patch.len(),
                policy.max_patch_bytes
            ),
            results: Vec::new(),
        };
    }
    if let Some(reason) = validate_patch_paths(patch, &policy.forbidden_path_prefixes) {
        return PatchPlanEvaluation {
            evaluation_id,
            ok: false,
            message: format!("patch rejected by policy: {reason}"),
            results: Vec::new(),
        };
    }

    for cmd in test_plan {
        if !policy.is_allowed_cmd(cmd) {
            return PatchPlanEvaluation {
                evaluation_id,
                ok: false,
                message: format!("command not allowlisted: {cmd}"),
                results: vec![PatchPlanCommandResult {
                    command: cmd.clone(),
                    ok: false,
                    exit_code: None,
                    duration_ms: 0,
                    stdout_tail: String::new(),
                    stderr_tail: String::new(),
                }],
            };
        }
    }

    // Prepare temp workspace.
    let td = match tempfile::TempDir::new() {
        Ok(v) => v,
        Err(e) => {
            return PatchPlanEvaluation {
                evaluation_id,
                ok: false,
                message: format!("failed to create temp dir: {e}"),
                results: Vec::new(),
            }
        }
    };
    let repo_dir = td.path().join("repo");

    // Prefer `git clone --local` to preserve permissions and avoid manual copy.
    let clone_status = Command::new("git")
        .arg("clone")
        .arg("--local")
        .arg(workspace_root)
        .arg(&repo_dir)
        .status()
        .await;

    if !matches!(clone_status, Ok(s) if s.success()) {
        return PatchPlanEvaluation {
            evaluation_id,
            ok: false,
            message: "failed to clone workspace (requires git)".to_string(),
            results: Vec::new(),
        };
    }

    // Apply patch.
    let patch_path = td.path().join("patch.diff");
    if let Err(e) = tokio::fs::write(&patch_path, patch).await {
        return PatchPlanEvaluation {
            evaluation_id,
            ok: false,
            message: format!("failed to write patch: {e}"),
            results: Vec::new(),
        };
    }

    let apply_status = Command::new("git")
        .arg("apply")
        .arg(&patch_path)
        .current_dir(&repo_dir)
        .status()
        .await;

    if !matches!(apply_status, Ok(s) if s.success()) {
        return PatchPlanEvaluation {
            evaluation_id,
            ok: false,
            message: "git apply failed".to_string(),
            results: Vec::new(),
        };
    }

    // Run plan.
    let mut all_ok = true;
    let mut results = Vec::new();

    for cmd in test_plan {
        let start = std::time::Instant::now();

        let mut c = if cfg!(windows) {
            let mut c = Command::new("cmd");
            c.arg("/C").arg(cmd);
            c
        } else {
            let mut c = Command::new("sh");
            c.arg("-lc").arg(cmd);
            c
        };
        c.current_dir(&repo_dir);

        let run = tokio::time::timeout(policy.command_timeout, c.output()).await;

        let (ok, exit_code, stdout_tail, stderr_tail) = match run {
            Err(_) => {
                all_ok = false;
                (false, None, String::new(), "command timed out".to_string())
            }
            Ok(Err(e)) => {
                all_ok = false;
                (
                    false,
                    None,
                    String::new(),
                    format!("failed to run command: {e}"),
                )
            }
            Ok(Ok(out)) => {
                let ok = out.status.success();
                if !ok {
                    all_ok = false;
                }
                let exit_code = out.status.code();
                let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                (
                    ok,
                    exit_code,
                    tail_limited(stdout, policy.max_log_bytes),
                    tail_limited(stderr, policy.max_log_bytes),
                )
            }
        };

        results.push(PatchPlanCommandResult {
            command: cmd.clone(),
            ok,
            exit_code,
            duration_ms: start.elapsed().as_millis() as u64,
            stdout_tail,
            stderr_tail,
        });
    }

    PatchPlanEvaluation {
        evaluation_id,
        ok: all_ok,
        message: if all_ok {
            "ok".to_string()
        } else {
            "one or more commands failed".to_string()
        },
        results,
    }
}
