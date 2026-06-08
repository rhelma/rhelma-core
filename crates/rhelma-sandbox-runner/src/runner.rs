#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use tokio::process::Command;
use tokio::time::{timeout, Duration};
use tracing::{debug, warn};
use validator::Validate;

use rhelma_ai_contracts::improvements::{
    AiImproveApplyRequestV1, AiImproveApplyResultV1, AiImproveEvaluationV1, AiImproveProposalV1,
    AiImproveRollbackRequestV1, AiImproveRollbackResultV1, EvaluationAttestedPayloadV1,
    SandboxCommandResultV1,
};

use rhelma_ai_attestation::{
    parse_hs256_keys, sha256_hex, sign_hs256_with_keyring, verify_hs256_with_keyring, Hs256KeyRing,
};

use crate::config::SandboxRunnerConfig;
use crate::patch_policy::{changed_paths, validate_paths};

/// struct (documented for contract compliance).
pub struct SandboxRunner {
    cfg: SandboxRunnerConfig,
}

impl SandboxRunner {
    /// fn (documented for contract compliance).
    pub fn new(cfg: SandboxRunnerConfig) -> Self {
        Self { cfg }
    }

    fn attestation_keyring(&self) -> Hs256KeyRing {
        if let Some(raw) = self.cfg.attestation_hmac_keys.as_deref() {
            return Hs256KeyRing {
                keys: parse_hs256_keys(raw),
                primary_kid: self.cfg.attestation_primary_kid.clone(),
            };
        }

        match self.cfg.attestation_hmac_secret.as_deref() {
            Some(secret) => {
                Hs256KeyRing::from_single(secret.as_bytes(), self.cfg.attestation_kid.clone())
            }
            None => Hs256KeyRing::default(),
        }
    }

    /// async fn (documented for contract compliance).
    pub async fn evaluate_proposal(
        &self,
        proposal: &AiImproveProposalV1,
    ) -> Result<AiImproveEvaluationV1> {
        proposal
            .validate()
            .map_err(|e| anyhow!("invalid proposal: {e}"))?;

        if proposal.patch.len() > self.cfg.max_patch_bytes {
            return Err(anyhow!(
                "patch too large ({} bytes > max {})",
                proposal.patch.len(),
                self.cfg.max_patch_bytes
            ));
        }

        if self.cfg.docker_enabled {
            self.evaluate_docker(proposal).await
        } else {
            self.evaluate_local(proposal).await
        }
    }

    /// async fn (documented for contract compliance).
    pub async fn apply_request(
        &self,
        req: &AiImproveApplyRequestV1,
    ) -> Result<AiImproveApplyResultV1> {
        req.validate()
            .map_err(|e| anyhow!("invalid apply request: {e}"))?;

        if req.patch.len() > self.cfg.max_patch_bytes {
            return Err(anyhow!(
                "patch too large ({} bytes > max {})",
                req.patch.len(),
                self.cfg.max_patch_bytes
            ));
        }

        let paths = changed_paths(&req.patch);
        validate_paths(
            &paths,
            &self.cfg.allowed_path_prefixes,
            &self.cfg.forbidden_path_prefixes,
        )
        .map_err(|e| anyhow!(e))?;

        // ------------------------------------------------------------------
        // Attestation verification (recommended; can be enforced)
        // ------------------------------------------------------------------
        let patch_sha = sha256_hex(req.patch.as_bytes());
        if patch_sha != req.patch_sha256_hex {
            return Err(anyhow!(
                "patch sha256 mismatch (computed {} != claimed {})",
                patch_sha,
                req.patch_sha256_hex
            ));
        }

        if req.evaluation_attested_payload.proposal_id != req.proposal_id {
            return Err(anyhow!("attested payload proposal_id mismatch"));
        }
        if req.evaluation_attested_payload.patch_sha256_hex != req.patch_sha256_hex {
            return Err(anyhow!("attested payload patch_sha256_hex mismatch"));
        }

        let plan_joined = req.test_plan.join("\n");
        let plan_sha = sha256_hex(plan_joined.as_bytes());
        if req.evaluation_attested_payload.test_plan_sha256_hex != plan_sha {
            return Err(anyhow!("attested payload test_plan_sha256_hex mismatch"));
        }

        if !req.evaluation_attested_payload.ok {
            return Err(anyhow!("refusing to apply: evaluation ok=false"));
        }

        let payload = serde_json::to_value(&req.evaluation_attested_payload)
            .context("serialize evaluation_attested_payload")?;

        let keyring = self.attestation_keyring();
        match (self.cfg.attestation_required, keyring.is_empty()) {
            (true, true) => {
                return Err(anyhow!(
                    "attestation required but no keys configured (set RHELMA_AI_ATTESTATION__HMAC_KEYS or RHELMA_AI_ATTESTATION__HMAC_SECRET)"
                ));
            }
            (true, false) => {
                let Some(att) = req.evaluation_attestation.as_ref() else {
                    return Err(anyhow!("attestation required but missing"));
                };
                verify_hs256_with_keyring(&payload, att, &keyring)
                    .map_err(|e| anyhow!("attestation verification failed: {e}"))?;
            }
            (false, false) => {
                if let Some(att) = req.evaluation_attestation.as_ref() {
                    verify_hs256_with_keyring(&payload, att, &keyring)
                        .map_err(|e| anyhow!("attestation verification failed: {e}"))?;
                }
            }
            (false, true) => {}
        }

        if self.cfg.docker_enabled {
            self.apply_docker(req).await
        } else {
            self.apply_local(req).await
        }
    }

    /// Execute a rollback request (revert a previously applied commit) in an isolated workspace.
    pub async fn rollback_request(
        &self,
        req: &AiImproveRollbackRequestV1,
    ) -> Result<AiImproveRollbackResultV1> {
        req.validate()
            .map_err(|e| anyhow!("invalid rollback request: {e}"))?;

        let tmp = tempfile::tempdir().context("create tempdir")?;
        let tmp_path = tmp.path().to_path_buf();

        self.clone_repo(&tmp_path).await?;

        // Best-effort fetch so the target commit and remote rollback branch are available.
        if self.cfg.rollback_fetch_remote {
            let _ = self
                .git_fetch_all(&tmp_path, &self.cfg.rollback_git_remote)
                .await;
        }

        let branch = req
            .branch
            .clone()
            .unwrap_or_else(|| self.rollback_branch_name(&req.request_id, &req.proposal_id));

        // Idempotency: if the rollback branch already exists on the remote, reuse it.
        if self.cfg.rollback_push_enabled {
            let remote_ref = self.remote_branch_ref(&self.cfg.rollback_git_remote, &branch);
            if self
                .git_ref_exists(&tmp_path, &remote_ref)
                .await
                .unwrap_or(false)
            {
                let commit = self.git_rev_parse_ref(&tmp_path, &remote_ref).await.ok();
                return Ok(AiImproveRollbackResultV1 {
                    proposal_id: req.proposal_id.clone(),
                    request_id: Some(req.request_id.clone()),
                    ok: true,
                    mode: if self.cfg.docker_enabled {
                        "docker".to_string()
                    } else {
                        "local".to_string()
                    },
                    branch: Some(branch),
                    commit,
                    results: Vec::new(),
                    summary: "idempotent: rollback branch already exists".to_string(),
                    rolled_back_at: Utc::now(),
                });
            }
        }

        // Start from the configured base branch.
        self.git_checkout_branch(&tmp_path, &self.cfg.rollback_base_branch)
            .await?;
        self.git_checkout_new_branch(&tmp_path, &branch).await?;

        // Perform revert.
        self.git_revert_commit(&tmp_path, &req.commit).await?;

        // Run verification plan.
        let plan = req.verification_plan.clone().unwrap_or_default();
        let mut results = Vec::new();
        let mut ok = true;
        for cmd in &plan {
            let r = if self.cfg.docker_enabled {
                self.run_docker_allowlisted(&tmp_path, cmd).await?
            } else {
                self.run_command_allowlisted(&tmp_path, cmd).await?
            };
            ok &= r.ok;
            results.push(r);
            if !ok {
                break;
            }
        }

        // Commit already created by `git revert`.
        let commit = if ok {
            Some(self.git_rev_parse_head(&tmp_path).await?)
        } else {
            None
        };

        if ok && self.cfg.rollback_push_enabled {
            self.git_push_branch_with_remote(&tmp_path, &self.cfg.rollback_git_remote, &branch)
                .await?;
        }

        Ok(AiImproveRollbackResultV1 {
            proposal_id: req.proposal_id.clone(),
            request_id: Some(req.request_id.clone()),
            ok,
            mode: if self.cfg.docker_enabled {
                "docker".to_string()
            } else {
                "local".to_string()
            },
            branch: if ok { Some(branch) } else { None },
            commit,
            results,
            summary: if ok {
                "rolled back".to_string()
            } else {
                "verification failed".to_string()
            },
            rolled_back_at: Utc::now(),
        })
    }

    async fn evaluate_local(
        &self,
        proposal: &AiImproveProposalV1,
    ) -> Result<AiImproveEvaluationV1> {
        let patch_sha = sha256_hex(proposal.patch.as_bytes());
        let plan_joined = proposal.test_plan.join("\n");
        let plan_sha = sha256_hex(plan_joined.as_bytes());

        let tmp = tempfile::tempdir().context("create tempdir")?;
        let tmp_path = tmp.path().to_path_buf();

        self.clone_repo(&tmp_path).await?;
        self.apply_patch(&tmp_path, &proposal.patch).await?;

        let mut results = Vec::new();
        let mut ok = true;
        for cmd in &proposal.test_plan {
            let r = self.run_command_allowlisted(&tmp_path, cmd).await?;
            ok &= r.ok;
            results.push(r);
            if !ok {
                break;
            }
        }

        let results_sha = sha256_hex(
            serde_json::to_vec(&results)
                .context("serialize results")?
                .as_slice(),
        );
        let evaluated_at = Utc::now();

        let attested_payload = EvaluationAttestedPayloadV1 {
            proposal_id: proposal.proposal_id.clone(),
            patch_sha256_hex: patch_sha.clone(),
            test_plan_sha256_hex: plan_sha.clone(),
            results_sha256_hex: results_sha.clone(),
            ok,
            mode: "local".to_string(),
            evaluated_at,
        };

        let keyring = self.attestation_keyring();
        let attestation = if keyring.is_empty() {
            None
        } else {
            let payload =
                serde_json::to_value(&attested_payload).context("serialize attested payload")?;
            Some(
                sign_hs256_with_keyring(&payload, &keyring)
                    .map_err(|e| anyhow!("sign attestation failed: {e}"))?,
            )
        };

        Ok(AiImproveEvaluationV1 {
            proposal_id: proposal.proposal_id.clone(),
            ok,
            patch_sha256_hex: patch_sha,
            test_plan_sha256_hex: plan_sha,
            results_sha256_hex: results_sha,
            mode: "local".to_string(),
            results,
            summary: if ok {
                "ok".to_string()
            } else {
                "failed".to_string()
            },
            attested_payload,
            attestation,
            evaluated_at,
        })
    }

    async fn evaluate_docker(
        &self,
        proposal: &AiImproveProposalV1,
    ) -> Result<AiImproveEvaluationV1> {
        let patch_sha = sha256_hex(proposal.patch.as_bytes());
        let plan_joined = proposal.test_plan.join("\n");
        let plan_sha = sha256_hex(plan_joined.as_bytes());

        let tmp = tempfile::tempdir().context("create tempdir")?;
        let tmp_path = tmp.path().to_path_buf();

        self.clone_repo(&tmp_path).await?;
        self.apply_patch(&tmp_path, &proposal.patch).await?;

        let mut results = Vec::new();
        let mut ok = true;
        for cmd in &proposal.test_plan {
            let r = self.run_docker_allowlisted(&tmp_path, cmd).await?;
            ok &= r.ok;
            results.push(r);
            if !ok {
                break;
            }
        }

        let results_sha = sha256_hex(
            serde_json::to_vec(&results)
                .context("serialize results")?
                .as_slice(),
        );
        let evaluated_at = Utc::now();

        let attested_payload = EvaluationAttestedPayloadV1 {
            proposal_id: proposal.proposal_id.clone(),
            patch_sha256_hex: patch_sha.clone(),
            test_plan_sha256_hex: plan_sha.clone(),
            results_sha256_hex: results_sha.clone(),
            ok,
            mode: "docker".to_string(),
            evaluated_at,
        };

        let keyring = self.attestation_keyring();
        let attestation = if keyring.is_empty() {
            None
        } else {
            let payload =
                serde_json::to_value(&attested_payload).context("serialize attested payload")?;
            Some(
                sign_hs256_with_keyring(&payload, &keyring)
                    .map_err(|e| anyhow!("sign attestation failed: {e}"))?,
            )
        };

        Ok(AiImproveEvaluationV1 {
            proposal_id: proposal.proposal_id.clone(),
            ok,
            patch_sha256_hex: patch_sha,
            test_plan_sha256_hex: plan_sha,
            results_sha256_hex: results_sha,
            mode: "docker".to_string(),
            results,
            summary: if ok {
                "ok".to_string()
            } else {
                "failed".to_string()
            },
            attested_payload,
            attestation,
            evaluated_at,
        })
    }

    async fn clone_repo(&self, dst: &PathBuf) -> Result<()> {
        // We require `git` and a working tree at workspace_root.
        // This keeps copying fast and reliable (no bespoke fs traversal).
        let src = PathBuf::from(&self.cfg.workspace_root);
        let src = src.canonicalize().context("canonicalize workspace root")?;

        debug!(src = %src.display(), dst = %dst.display(), "sandbox clone");

        let mut cmd = Command::new("git");
        cmd.arg("clone")
            .arg("--no-hardlinks")
            .arg("--depth")
            .arg("1")
            .arg(src)
            .arg(dst)
            .kill_on_drop(true);

        let out = cmd.output().await.context("git clone")?;
        if !out.status.success() {
            return Err(anyhow!(
                "git clone failed: {}",
                String::from_utf8_lossy(&out.stderr)
            ));
        }
        Ok(())
    }

    async fn apply_patch(&self, workdir: &PathBuf, patch: &str) -> Result<()> {
        let mut cmd = Command::new("git");
        cmd.current_dir(workdir)
            .arg("apply")
            .arg("--whitespace=nowarn")
            .arg("-")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);

        let mut child = cmd.spawn().context("spawn git apply")?;
        {
            let mut stdin = child.stdin.take().context("stdin")?;
            use tokio::io::AsyncWriteExt;
            stdin.write_all(patch.as_bytes()).await?;
        }

        let out = child.wait_with_output().await?;
        if !out.status.success() {
            return Err(anyhow!(
                "git apply failed: {}",
                String::from_utf8_lossy(&out.stderr)
            ));
        }

        Ok(())
    }

    fn is_allowlisted(&self, cmd: &str) -> bool {
        let s = cmd.trim();
        self.cfg
            .allowed_command_prefixes
            .iter()
            .any(|p| s.starts_with(p))
    }

    async fn run_command_allowlisted(
        &self,
        workdir: &PathBuf,
        cmd: &str,
    ) -> Result<SandboxCommandResultV1> {
        if !self.is_allowlisted(cmd) {
            return Err(anyhow!("command not allowlisted: {cmd}"));
        }

        // No shell: split on whitespace. This is intentionally strict.
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        let (prog, args) = parts
            .split_first()
            .ok_or_else(|| anyhow!("empty command"))?;

        let start = Instant::now();

        let child = Command::new(prog)
            .args(args)
            .current_dir(workdir)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .context("spawn command")?;

        let tout = Duration::from_millis(self.cfg.command_timeout_ms);
        let out = timeout(tout, child.wait_with_output())
            .await
            .map_err(|_| anyhow!("command timeout: {cmd}"))??;

        let dur = start.elapsed().as_millis() as u64;
        let ok = out.status.success();
        if !ok {
            warn!(command = %cmd, "sandbox command failed");
        }

        Ok(SandboxCommandResultV1 {
            command: cmd.to_string(),
            ok,
            exit_code: out.status.code(),
            duration_ms: dur,
            stdout: String::from_utf8_lossy(&out.stdout).to_string(),
            stderr: String::from_utf8_lossy(&out.stderr).to_string(),
        })
    }

    async fn run_docker_allowlisted(
        &self,
        workdir: &Path,
        cmd: &str,
    ) -> Result<SandboxCommandResultV1> {
        if !self.is_allowlisted(cmd) {
            return Err(anyhow!("command not allowlisted: {cmd}"));
        }

        // Run command inside container using bash -lc for environment setup.
        // The host workspace is mounted read-write into /work.
        let start = Instant::now();

        let mut child = Command::new("docker");
        child
            .arg("run")
            .arg("--rm")
            .arg("--network")
            .arg("none")
            .arg("--cap-drop")
            .arg("ALL")
            .arg("--security-opt")
            .arg("no-new-privileges")
            .arg("-v")
            .arg(format!("{}:/work", workdir.display()))
            .arg("-w")
            .arg("/work")
            .arg(&self.cfg.docker_image)
            .arg("bash")
            .arg("-lc")
            .arg(cmd)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);

        let tout = Duration::from_millis(self.cfg.command_timeout_ms);
        let out = timeout(tout, child.output())
            .await
            .map_err(|_| anyhow!("docker command timeout: {cmd}"))??;

        let dur = start.elapsed().as_millis() as u64;
        let ok = out.status.success();
        if !ok {
            debug!(stderr = %String::from_utf8_lossy(&out.stderr), "docker command stderr");
        }

        Ok(SandboxCommandResultV1 {
            command: cmd.to_string(),
            ok,
            exit_code: out.status.code(),
            duration_ms: dur,
            stdout: String::from_utf8_lossy(&out.stdout).to_string(),
            stderr: String::from_utf8_lossy(&out.stderr).to_string(),
        })
    }

    async fn apply_local(&self, req: &AiImproveApplyRequestV1) -> Result<AiImproveApplyResultV1> {
        let tmp = tempfile::tempdir().context("create tempdir")?;
        let tmp_path = tmp.path().to_path_buf();

        self.clone_repo(&tmp_path).await?;

        let branch = self.branch_name(&req.request_id, &req.title);

        // Idempotency: when push is enabled, reuse an existing remote branch if present.
        if self.cfg.apply_push_enabled && self.cfg.apply_fetch_remote {
            let _ = self
                .git_fetch_all(&tmp_path, &self.cfg.apply_git_remote)
                .await;
        }
        if self.cfg.apply_push_enabled {
            let remote_ref = self.remote_branch_ref(&self.cfg.apply_git_remote, &branch);
            if self
                .git_ref_exists(&tmp_path, &remote_ref)
                .await
                .unwrap_or(false)
            {
                let commit = self.git_rev_parse_ref(&tmp_path, &remote_ref).await.ok();
                return Ok(AiImproveApplyResultV1 {
                    proposal_id: req.proposal_id.clone(),
                    request_id: Some(req.request_id.clone()),
                    ok: true,
                    mode: "local".to_string(),
                    branch: Some(branch),
                    commit,
                    results: Vec::new(),
                    summary: "idempotent: apply branch already exists".to_string(),
                    applied_at: Utc::now(),
                });
            }
        }
        self.git_checkout_new_branch(&tmp_path, &branch).await?;

        self.apply_patch(&tmp_path, &req.patch).await?;

        let mut results = Vec::new();
        let mut ok = true;
        for cmd in &req.test_plan {
            let r = self.run_command_allowlisted(&tmp_path, cmd).await?;
            ok &= r.ok;
            results.push(r);
            if !ok {
                break;
            }
        }

        let mut commit: Option<String> = None;
        if ok {
            self.git_add_all(&tmp_path).await?;
            self.git_commit(&tmp_path, &req.proposal_id, &req.title, &req.actor)
                .await?;
            commit = Some(self.git_rev_parse_head(&tmp_path).await?);

            if self.cfg.apply_push_enabled {
                self.git_push_branch(&tmp_path, &branch).await?;
            }
        }

        Ok(AiImproveApplyResultV1 {
            proposal_id: req.proposal_id.clone(),
            request_id: Some(req.request_id.clone()),
            ok,
            mode: "local".to_string(),
            branch: if ok { Some(branch) } else { None },
            commit,
            results,
            summary: if ok {
                "applied".to_string()
            } else {
                "failed".to_string()
            },
            applied_at: Utc::now(),
        })
    }

    async fn apply_docker(&self, req: &AiImproveApplyRequestV1) -> Result<AiImproveApplyResultV1> {
        let tmp = tempfile::tempdir().context("create tempdir")?;
        let tmp_path = tmp.path().to_path_buf();

        self.clone_repo(&tmp_path).await?;

        let branch = self.branch_name(&req.request_id, &req.title);

        // Idempotency: when push is enabled, reuse an existing remote branch if present.
        if self.cfg.apply_push_enabled && self.cfg.apply_fetch_remote {
            let _ = self
                .git_fetch_all(&tmp_path, &self.cfg.apply_git_remote)
                .await;
        }
        if self.cfg.apply_push_enabled {
            let remote_ref = self.remote_branch_ref(&self.cfg.apply_git_remote, &branch);
            if self
                .git_ref_exists(&tmp_path, &remote_ref)
                .await
                .unwrap_or(false)
            {
                let commit = self.git_rev_parse_ref(&tmp_path, &remote_ref).await.ok();
                return Ok(AiImproveApplyResultV1 {
                    proposal_id: req.proposal_id.clone(),
                    request_id: Some(req.request_id.clone()),
                    ok: true,
                    mode: "docker".to_string(),
                    branch: Some(branch),
                    commit,
                    results: Vec::new(),
                    summary: "idempotent: apply branch already exists".to_string(),
                    applied_at: Utc::now(),
                });
            }
        }
        self.git_checkout_new_branch(&tmp_path, &branch).await?;

        self.apply_patch(&tmp_path, &req.patch).await?;

        let mut results = Vec::new();
        let mut ok = true;
        for cmd in &req.test_plan {
            let r = self.run_docker_allowlisted(&tmp_path, cmd).await?;
            ok &= r.ok;
            results.push(r);
            if !ok {
                break;
            }
        }

        let mut commit: Option<String> = None;
        if ok {
            self.git_add_all(&tmp_path).await?;
            self.git_commit(&tmp_path, &req.proposal_id, &req.title, &req.actor)
                .await?;
            commit = Some(self.git_rev_parse_head(&tmp_path).await?);

            if self.cfg.apply_push_enabled {
                self.git_push_branch(&tmp_path, &branch).await?;
            }
        }

        Ok(AiImproveApplyResultV1 {
            proposal_id: req.proposal_id.clone(),
            request_id: Some(req.request_id.clone()),
            ok,
            mode: "docker".to_string(),
            branch: if ok { Some(branch) } else { None },
            commit,
            results,
            summary: if ok {
                "applied".to_string()
            } else {
                "failed".to_string()
            },
            applied_at: Utc::now(),
        })
    }

    fn rollback_branch_name(&self, request_id: &str, proposal_id: &str) -> String {
        let short_req: String = request_id.chars().take(8).collect();
        let short_pid: String = proposal_id.chars().take(8).collect();
        format!(
            "{}/{}-{}",
            self.cfg.rollback_branch_prefix, short_pid, short_req
        )
    }

    fn remote_branch_ref(&self, remote: &str, branch: &str) -> String {
        // refs/remotes/origin/ai/improve/foo
        format!("refs/remotes/{}/{}", remote, branch)
    }

    fn branch_name(&self, request_id: &str, title: &str) -> String {
        let slug: String = title
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() {
                    c.to_ascii_lowercase()
                } else {
                    '-'
                }
            })
            .collect();

        let slug = slug.trim_matches('-');
        let short = request_id.chars().take(8).collect::<String>();

        format!("{}/{}-{}", self.cfg.apply_branch_prefix, slug, short)
    }

    async fn git_fetch_all(&self, workdir: &PathBuf, remote: &str) -> Result<()> {
        self.git(workdir, &["fetch", "--prune", remote]).await
    }

    async fn git_ref_exists(&self, workdir: &PathBuf, reference: &str) -> Result<bool> {
        let mut cmd = Command::new("git");
        cmd.current_dir(workdir)
            .arg("show-ref")
            .arg("--verify")
            .arg("--quiet")
            .arg(reference)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true);
        let out = cmd.output().await.context("git show-ref")?;
        Ok(out.status.success())
    }

    async fn git_rev_parse_ref(&self, workdir: &PathBuf, reference: &str) -> Result<String> {
        let out = self.git_output(workdir, &["rev-parse", reference]).await?;
        Ok(out.trim().to_string())
    }

    async fn git_checkout_branch(&self, workdir: &PathBuf, branch: &str) -> Result<()> {
        self.git(workdir, &["checkout", branch]).await
    }

    async fn git_revert_commit(&self, workdir: &PathBuf, commit: &str) -> Result<()> {
        self.git(workdir, &["revert", "--no-edit", commit]).await
    }

    async fn git_push_branch_with_remote(
        &self,
        workdir: &PathBuf,
        remote: &str,
        branch: &str,
    ) -> Result<()> {
        self.git(workdir, &["push", remote, branch]).await
    }

    async fn git_checkout_new_branch(&self, workdir: &PathBuf, branch: &str) -> Result<()> {
        self.git(workdir, &["checkout", "-b", branch]).await
    }

    async fn git_add_all(&self, workdir: &PathBuf) -> Result<()> {
        self.git(workdir, &["add", "-A"]).await
    }

    async fn git_commit(
        &self,
        workdir: &PathBuf,
        proposal_id: &str,
        title: &str,
        actor: &str,
    ) -> Result<()> {
        let msg = format!("ai-improve: {} ({})", title, proposal_id);

        let mut cmd = Command::new("git");
        cmd.current_dir(workdir)
            .arg("-c")
            .arg("user.name=rhelma-ai")
            .arg("-c")
            .arg("user.email=rhelma-ai@local")
            .arg("commit")
            .arg("--no-gpg-sign")
            .arg("--no-verify")
            .arg("-m")
            .arg(msg)
            .env("GIT_AUTHOR_NAME", actor)
            .env("GIT_AUTHOR_EMAIL", "rhelma-ai@local")
            .env("GIT_COMMITTER_NAME", "rhelma-ai")
            .env("GIT_COMMITTER_EMAIL", "rhelma-ai@local")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);

        let out = cmd.output().await.context("git commit")?;
        if !out.status.success() {
            return Err(anyhow!(
                "git commit failed: {}",
                String::from_utf8_lossy(&out.stderr)
            ));
        }
        Ok(())
    }

    async fn git_rev_parse_head(&self, workdir: &PathBuf) -> Result<String> {
        let out = self.git_output(workdir, &["rev-parse", "HEAD"]).await?;
        Ok(out.trim().to_string())
    }

    async fn git_push_branch(&self, workdir: &PathBuf, branch: &str) -> Result<()> {
        self.git(workdir, &["push", &self.cfg.apply_git_remote, branch])
            .await
    }

    async fn git(&self, workdir: &PathBuf, args: &[&str]) -> Result<()> {
        let mut cmd = Command::new("git");
        cmd.current_dir(workdir)
            .args(args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);

        let out = cmd.output().await.context("git")?;
        if !out.status.success() {
            return Err(anyhow!(
                "git {:?} failed: {}",
                args,
                String::from_utf8_lossy(&out.stderr)
            ));
        }
        Ok(())
    }

    async fn git_output(&self, workdir: &PathBuf, args: &[&str]) -> Result<String> {
        let mut cmd = Command::new("git");
        cmd.current_dir(workdir)
            .args(args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);

        let out = cmd.output().await.context("git")?;
        if !out.status.success() {
            return Err(anyhow!(
                "git {:?} failed: {}",
                args,
                String::from_utf8_lossy(&out.stderr)
            ));
        }
        Ok(String::from_utf8_lossy(&out.stdout).to_string())
    }
}
