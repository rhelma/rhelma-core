#![forbid(unsafe_code)]

use std::time::Duration;

use rhelma_sandbox_runner::{validate_patch_paths, SandboxPolicy};

#[test]
fn validate_patch_paths_blocks_forbidden_prefix() {
    let patch = "--- a/crates/rhelma-auth/src/lib.rs\n+++ b/crates/rhelma-auth/src/lib.rs\n@@ -1 +1 @@\n-foo\n+bar\n";
    let forbidden = vec!["crates/rhelma-auth".to_string()];

    let reason = validate_patch_paths(patch, &forbidden).expect("should reject");
    assert!(reason.contains("forbidden"));
}

#[test]
fn validate_patch_paths_allows_safe_prefix() {
    let patch = "--- a/crates/rhelma-core/src/lib.rs\n+++ b/crates/rhelma-core/src/lib.rs\n@@ -1 +1 @@\n-foo\n+bar\n";
    let forbidden = vec!["crates/rhelma-auth".to_string()];

    assert!(validate_patch_paths(patch, &forbidden).is_none());
}

#[test]
fn allowed_cmd_works_by_prefix() {
    let policy = SandboxPolicy {
        max_patch_bytes: 1000,
        forbidden_path_prefixes: vec![],
        allowed_command_prefixes: vec!["cargo test".to_string(), "git apply".to_string()],
        command_timeout: Duration::from_secs(1),
        max_log_bytes: 64,
        redacted_env_prefixes: vec![],
    };

    assert!(policy.is_allowed_cmd("cargo test -p rhelma-core"));
    assert!(!policy.is_allowed_cmd("rm -rf /"));
}
