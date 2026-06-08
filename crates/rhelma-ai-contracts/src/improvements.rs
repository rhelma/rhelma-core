#![forbid(unsafe_code)]

//! Self-improvement (self-expanding) loop contracts.
//!
//! Topics:
//! - `ai.improve.proposal`
//! - `ai.improve.evaluation`
//! - `ai.improve.approval`
//! - `ai.improve.apply.request`
//! - `ai.improve.apply.result`
//! - `ai.improve.rollback.request`
//! - `ai.improve.rollback.result`

use chrono::{DateTime, Utc};
use rhelma_ai_attestation::AttestationV1;
use serde::{Deserialize, Serialize};
use validator::Validate;

/// Kafka topic for improvement proposals.
pub const TOPIC_IMPROVE_PROPOSAL: &str = "ai.improve.proposal";
/// Kafka topic for evaluation results.
pub const TOPIC_IMPROVE_EVALUATION: &str = "ai.improve.evaluation";
/// Kafka topic for approvals / rejections.
pub const TOPIC_IMPROVE_APPROVAL: &str = "ai.improve.approval";
/// Kafka topic for apply requests (after approval).
pub const TOPIC_IMPROVE_APPLY_REQUEST: &str = "ai.improve.apply.request";
/// Kafka topic for apply results (branch/commit created or failure).
pub const TOPIC_IMPROVE_APPLY_RESULT: &str = "ai.improve.apply.result";
/// Kafka topic for rollback requests (manual or automatic).
pub const TOPIC_IMPROVE_ROLLBACK_REQUEST: &str = "ai.improve.rollback.request";
/// Kafka topic for rollback results.
pub const TOPIC_IMPROVE_ROLLBACK_RESULT: &str = "ai.improve.rollback.result";

/// Schema references (v1).
pub const SCHEMA_IMPROVE_PROPOSAL_V1: &str = "rhelma://schemas/ai.improve.proposal@v1";
/// const (documented for contract compliance).
pub const SCHEMA_IMPROVE_EVALUATION_V1: &str = "rhelma://schemas/ai.improve.evaluation@v1";
/// const (documented for contract compliance).
pub const SCHEMA_IMPROVE_APPROVAL_V1: &str = "rhelma://schemas/ai.improve.approval@v1";
/// const (documented for contract compliance).
pub const SCHEMA_IMPROVE_APPLY_REQUEST_V1: &str = "rhelma://schemas/ai.improve.apply.request@v1";
/// const (documented for contract compliance).
pub const SCHEMA_IMPROVE_APPLY_RESULT_V1: &str = "rhelma://schemas/ai.improve.apply.result@v1";
/// const (documented for contract compliance).
pub const SCHEMA_IMPROVE_ROLLBACK_REQUEST_V1: &str =
    "rhelma://schemas/ai.improve.rollback.request@v1";
/// const (documented for contract compliance).
pub const SCHEMA_IMPROVE_ROLLBACK_RESULT_V1: &str =
    "rhelma://schemas/ai.improve.rollback.result@v1";

/// Risk level assigned to an improvement proposal.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
/// enum (documented for contract compliance).
pub enum ImprovementRiskLevel {
    #[default]
    /// Variant `Low`.
    Low,
    /// Variant `Medium`.
    Medium,
    /// Variant `High`.
    High,
    /// Variant `Critical`.
    Critical,
}

/// Lifecycle state for a proposal.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
/// enum (documented for contract compliance).
pub enum ImprovementStatus {
    #[default]
    /// Variant `Proposed`.
    Proposed,
    /// Variant `Evaluating`.
    Evaluating,
    /// Variant `Evaluated`.
    Evaluated,
    /// Variant `Approved`.
    Approved,
    /// Variant `ApplyRequested`.
    ApplyRequested,
    /// Variant `Applied`.
    Applied,
    /// Phase 33: post-deploy health gate is pending.
    PostCheckPending,
    /// Phase 33: post-deploy health gate passed.
    PostCheckPassed,
    /// Phase 33: post-deploy health gate failed.
    PostCheckFailed,
    // Phase 34: progressive rollout stages are pending/in-progress/done.
    /// Variant `RolloutPending`.
    RolloutPending,
    /// Variant `RolloutInProgress`.
    RolloutInProgress,
    /// Variant `RolloutPassed`.
    RolloutPassed,
    /// Variant `RolloutFailed`.
    RolloutFailed,
    /// Variant `RollbackRequested`.
    RollbackRequested,
    /// Variant `Rejected`.
    Rejected,
    /// Variant `RolledBack`.
    RolledBack,
    /// Variant `RollbackFailed`.
    RollbackFailed,
    /// Variant `Failed`.
    Failed,
}

/// Improvement proposal payload.
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct AiImproveProposalV1 {
    /// Proposal id (uuidv7 as string).
    #[validate(length(min = 10))]
    pub proposal_id: String,

    /// Human-friendly title.
    #[validate(length(min = 3, max = 160))]
    pub title: String,

    /// Target path/module (e.g. `apps/ai-orchestrator`).
    #[validate(length(min = 1, max = 256))]
    pub target: String,

    /// Unified diff patch.
    #[validate(length(min = 10))]
    pub patch: String,

    /// Commands to evaluate in the sandbox.
    #[validate(length(min = 1, max = 10))]
    pub test_plan: Vec<String>,

    /// Risk level.
    pub risk_level: ImprovementRiskLevel,

    /// Who requested this change.
    #[validate(length(min = 1, max = 128))]
    pub actor: String,

    /// Timestamp.
    pub created_at: DateTime<Utc>,
}

/// Result of executing a single command in the sandbox.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxCommandResultV1 {
    /// Field `command`.
    pub command: String,
    /// Field `ok`.
    pub ok: bool,
    /// Field `exit_code`.
    pub exit_code: Option<i32>,
    /// Field `duration_ms`.
    pub duration_ms: u64,
    /// Field `stdout`.
    pub stdout: String,
    /// Field `stderr`.
    pub stderr: String,
}

/// Phase 33: result of checking a single health endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UrlHealthCheckV1 {
    /// Field `url`.
    pub url: String,
    /// Field `ok`.
    pub ok: bool,
    /// Field `status_code`.
    pub status_code: Option<u16>,
    /// Field `duration_ms`.
    pub duration_ms: u64,
    /// Field `error`.
    pub error: Option<String>,
}

/// Phase 33: post-deploy health gate result (stored/persisted; not necessarily emitted as an event).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiImprovePostCheckResultV1 {
    /// Field `proposal_id`.
    pub proposal_id: String,
    /// Field `ok`.
    pub ok: bool,
    /// Field `attempts`.
    pub attempts: u32,
    /// Field `checks`.
    pub checks: Vec<UrlHealthCheckV1>,
    /// Field `started_at`.
    pub started_at: DateTime<Utc>,
    /// Field `finished_at`.
    pub finished_at: DateTime<Utc>,
    /// Field `summary`.
    pub summary: String,
}

/// Phase 35: Prometheus comparator for a query gate.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
/// enum (documented for contract compliance).
pub enum PrometheusComparatorV1 {
    /// Variant `Lt`.
    Lt,
    #[default]
    /// Variant `Lte`.
    Lte,
    /// Variant `Gt`.
    Gt,
    /// Variant `Gte`.
    Gte,
}

/// Phase 35: A single Prometheus query gate (scalar value compared to a threshold).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrometheusQueryGateV1 {
    /// Human-friendly name (e.g. "error_rate", "latency_p95_ms").
    pub name: String,
    /// PromQL expression (should evaluate to a single scalar / 1-element vector).
    pub expr: String,
    /// Comparator.
    pub comparator: PrometheusComparatorV1,
    /// Threshold value.
    pub threshold: f64,
    /// Optional description (shown in audit/logs).
    pub description: Option<String>,
}

/// Phase 35: The evaluated result of a single Prometheus query gate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrometheusQueryResultV1 {
    /// Field `name`.
    pub name: String,
    /// Field `expr`.
    pub expr: String,
    /// Field `comparator`.
    pub comparator: PrometheusComparatorV1,
    /// Field `threshold`.
    pub threshold: f64,
    /// Field `ok`.
    pub ok: bool,
    /// Field `value`.
    pub value: Option<f64>,
    /// Field `error`.
    pub error: Option<String>,
}

/// Phase 34: optional decision metrics returned by a metrics endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiImproveRolloutDecisionMetricsV1 {
    /// Optional HTTP JSON metrics URL.
    pub url: String,
    /// Overall metrics gate status (true if all enabled metric sources passed).
    pub ok: bool,

    /// Phase 35: status of the HTTP JSON metrics endpoint (if configured).
    pub json_ok: Option<bool>,
    /// Phase 35: status of the Prometheus query gate (if configured).
    pub prometheus_ok: Option<bool>,

    /// Optional canonical fields (best-effort) extracted from either JSON metrics or Prometheus queries.
    pub error_rate: Option<f64>,
    /// Field `latency_p95_ms`.
    pub latency_p95_ms: Option<u64>,

    /// Raw response from JSON metrics endpoint, if any.
    pub raw: Option<String>,
    /// Error message if gate failed (best-effort).
    pub error: Option<String>,

    /// Phase 35: Prometheus base URL (if used).
    pub prometheus_base_url: Option<String>,
    /// Phase 35: evaluated Prometheus query results (if configured).
    pub prometheus_results: Option<Vec<PrometheusQueryResultV1>>,
}

/// Phase 37: weights for weighted rollout scoring.
///
/// The final score is computed as a weighted average of the enabled component scores.
/// Components that are `None` are ignored in the denominator (their weights are skipped).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AiImproveRolloutScoreWeightsV1 {
    /// Weight of the health score.
    pub health: f64,
    /// Weight of the HTTP JSON metrics score.
    pub json_metrics: f64,
    /// Weight of the Prometheus gate score.
    pub prometheus: f64,
    /// Weight of the stability score (Phase 36).
    pub stability: f64,
    /// Weight of the threshold score (max_error_rate/max_latency_p95_ms).
    pub thresholds: f64,
}

impl Default for AiImproveRolloutScoreWeightsV1 {
    fn default() -> Self {
        Self {
            health: 0.5,
            json_metrics: 0.2,
            prometheus: 0.2,
            stability: 0.1,
            thresholds: 0.0,
        }
    }
}

/// Phase 37: weighted scoring report for a rollout stage.
///
/// This is stored inside the rollout result for auditability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiImproveRolloutScoreReportV1 {
    /// Minimum score required for the stage to pass (if scoring is enabled).
    pub min_score: f64,
    /// Final aggregated score.
    pub score: f64,
    /// True if score >= min_score.
    pub ok: bool,
    /// Weights used to compute the score.
    pub weights: AiImproveRolloutScoreWeightsV1,

    /// Component scores (0..=1) if present.
    pub health_score: Option<f64>,
    /// Field `json_metrics_score`.
    pub json_metrics_score: Option<f64>,
    /// Field `prometheus_score`.
    pub prometheus_score: Option<f64>,
    /// Field `stability_score`.
    pub stability_score: Option<f64>,
    /// Field `thresholds_score`.
    pub thresholds_score: Option<f64>,

    /// Best-effort notes for debugging.
    pub notes: Vec<String>,
}

/// Phase 34: rollout stage plan (canary/progressive rollout).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AiImproveRolloutStagePlanV1 {
    /// Field `name`.
    pub name: String,
    /// Optional deploy webhook URL invoked before checking health/metrics.
    pub deploy_webhook_url: Option<String>,
    /// Optional seconds to wait after deploy webhook before health checks.
    pub deploy_wait_seconds: i64,

    /// Field `health_urls`.
    pub health_urls: Vec<String>,
    /// Field `require_all`.
    pub require_all: bool,

    /// Field `max_attempts`.
    pub max_attempts: u32,
    /// Field `attempt_delay_seconds`.
    pub attempt_delay_seconds: i64,
    /// Field `timeout_ms`.
    pub timeout_ms: u64,

    /// Optional metrics endpoint returning JSON like {"error_rate":0.01,"latency_p95_ms":120}.
    pub metrics_url: Option<String>,
    /// Field `require_metrics`.
    pub require_metrics: bool,
    /// Field `max_error_rate`.
    pub max_error_rate: Option<f64>,
    /// Field `max_latency_p95_ms`.
    pub max_latency_p95_ms: Option<u64>,

    /// Phase 35: Prometheus base URL for query gates (e.g. http://prometheus:9090).
    pub prometheus_base_url: Option<String>,
    /// Phase 35: list of Prometheus query gates.
    pub prometheus_queries: Vec<PrometheusQueryGateV1>,
    /// If true, missing/failed Prometheus evaluation fails the stage.
    pub require_prometheus: bool,

    /// Phase 36: stability gate (consecutive passing samples).
    ///
    /// If `stability_required_passes` > 1, the orchestrator will re-check health/metrics
    /// `stability_required_passes` times in a row (consecutive) within `stability_window_seconds`.
    ///
    /// This is meant to prevent "one lucky pass" during rollout.
    pub stability_required_passes: u32,
    /// Phase 36: maximum time window to achieve the required consecutive passes.
    pub stability_window_seconds: u64,
    /// Phase 36: time between stability samples.
    pub stability_interval_seconds: u64,

    /// Phase 37: enable weighted scoring gate.
    ///
    /// If enabled, the stage passes only if `score >= score_min` *and* all required
    /// component gates (require_metrics / require_prometheus) are satisfied.
    pub score_gate_enabled: bool,
    /// Phase 37: minimum score required to pass (0..=1).
    pub score_min: f64,
    /// Phase 37: optional weights for scoring.
    ///
    /// If `None`, the orchestrator uses `AiImproveRolloutScoreWeightsV1::default()`.
    pub score_weights: Option<AiImproveRolloutScoreWeightsV1>,
}

impl Default for AiImproveRolloutStagePlanV1 {
    fn default() -> Self {
        Self {
            name: "canary".to_string(),
            deploy_webhook_url: None,
            deploy_wait_seconds: 0,
            health_urls: Vec::new(),
            require_all: true,
            max_attempts: 3,
            attempt_delay_seconds: 10,
            timeout_ms: 1500,
            metrics_url: None,
            require_metrics: false,
            max_error_rate: None,
            max_latency_p95_ms: None,
            prometheus_base_url: None,
            prometheus_queries: Vec::new(),
            require_prometheus: false,

            stability_required_passes: 1,
            stability_window_seconds: 300,
            stability_interval_seconds: 10,

            score_gate_enabled: false,
            score_min: 1.0,
            score_weights: None,
        }
    }
}

/// Phase 38: global rollout budget/policy.
///
/// This allows cross-stage stopping decisions like:
/// - total rollout time budget
/// - minimum canary (first stage) score
/// - minimum overall (running) score
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct AiImproveRolloutBudgetV1 {
    /// If > 0, the rollout must finish within this total duration.
    pub max_total_duration_seconds: u64,

    /// If set, the first stage must have score >= this value.
    pub first_stage_min_score: Option<f64>,

    /// If set, the running overall score (min stage score so far) must remain >= this value.
    pub overall_min_score: Option<f64>,
}

/// Phase 38: optional wrapper around stage plans.
///
/// For backwards compatibility, orchestrator still accepts plain JSON arrays of stages.
/// This struct enables cross-stage budgets in a single JSON value.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct AiImproveRolloutPlanV1 {
    /// Field `budget`.
    pub budget: AiImproveRolloutBudgetV1,
    /// Field `stages`.
    pub stages: Vec<AiImproveRolloutStagePlanV1>,
}

/// Phase 36: stability report for a rollout stage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiImproveRolloutStabilityReportV1 {
    /// Field `required_passes`.
    pub required_passes: u32,
    /// Field `achieved_consecutive_passes`.
    pub achieved_consecutive_passes: u32,
    /// Field `total_samples`.
    pub total_samples: u32,
    /// Field `passed_samples`.
    pub passed_samples: u32,
    /// Simple score: passed_samples / total_samples.
    pub score: f64,
    /// Field `window_seconds`.
    pub window_seconds: u64,
    /// Field `interval_seconds`.
    pub interval_seconds: u64,
    /// Field `started_at`.
    pub started_at: DateTime<Utc>,
    /// Field `finished_at`.
    pub finished_at: DateTime<Utc>,
    /// Best-effort summaries of sampled checks.
    pub sample_summaries: Vec<String>,
}

/// Phase 39: per-stage telemetry summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiImproveRolloutStageTelemetryV1 {
    /// Field `name`.
    pub name: String,
    /// Field `duration_ms`.
    pub duration_ms: u64,
    /// Field `ok`.
    pub ok: bool,
    #[serde(default)]
    /// Field `failure_code`.
    pub failure_code: Option<String>,
}

/// Phase 39: rollout telemetry summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiImproveRolloutTelemetryV1 {
    /// Field `total_duration_ms`.
    pub total_duration_ms: u64,
    /// Field `stages`.
    pub stages: Vec<AiImproveRolloutStageTelemetryV1>,
    #[serde(default)]
    /// Field `rollback_requested`.
    pub rollback_requested: bool,
    #[serde(default)]
    /// Field `rollback_requested_at`.
    pub rollback_requested_at: Option<DateTime<Utc>>,
    #[serde(default)]
    /// Field `rollback_reason`.
    pub rollback_reason: Option<String>,
}

/// Phase 34: rollout stage execution result.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiImproveRolloutStageResultV1 {
    /// Field `plan`.
    pub plan: AiImproveRolloutStagePlanV1,
    /// Field `ok`.
    pub ok: bool,
    /// Field `attempts`.
    pub attempts: u32,
    /// Field `checks`.
    pub checks: Vec<UrlHealthCheckV1>,
    /// Field `metrics`.
    pub metrics: Option<AiImproveRolloutDecisionMetricsV1>,
    /// Phase 36: stability report (if stability gate was enabled).
    #[serde(default)]
    pub stability: Option<AiImproveRolloutStabilityReportV1>,
    /// Phase 37: weighted scoring report (if enabled).
    #[serde(default)]
    pub score: Option<AiImproveRolloutScoreReportV1>,
    /// Phase 39: machine-readable failure code (best-effort).
    #[serde(default)]
    pub failure_code: Option<String>,
    /// Field `started_at`.
    pub started_at: DateTime<Utc>,
    /// Field `finished_at`.
    pub finished_at: DateTime<Utc>,
    /// Field `summary`.
    pub summary: String,
}

/// Phase 34: overall rollout result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiImproveRolloutResultV1 {
    /// Field `proposal_id`.
    pub proposal_id: String,
    /// Field `commit`.
    pub commit: Option<String>,
    /// Field `ok`.
    pub ok: bool,
    /// Field `stages`.
    pub stages: Vec<AiImproveRolloutStageResultV1>,
    /// Phase 37: aggregated rollout score across stages (best-effort).
    #[serde(default)]
    pub overall_score: Option<f64>,
    /// Phase 37: how `overall_score` was computed (e.g. "min_stage_score").
    #[serde(default)]
    pub score_method: Option<String>,
    /// Phase 38: optional rollout budget/policy (if provided via AiImproveRolloutPlanV1).
    #[serde(default)]
    pub budget: Option<AiImproveRolloutBudgetV1>,
    /// Phase 38: true if the rollout was stopped early by budget/policy (even if all executed stages passed).
    #[serde(default)]
    pub stopped_early: bool,
    /// Phase 38: reason for early stop (if any).
    #[serde(default)]
    pub stop_reason: Option<String>,
    /// Phase 39: machine-readable stop code for early stop/failure.
    #[serde(default)]
    pub stop_code: Option<String>,

    /// Phase 39: telemetry summary (durations, rollback trigger info).
    #[serde(default)]
    pub telemetry: Option<AiImproveRolloutTelemetryV1>,
    /// Field `started_at`.
    pub started_at: DateTime<Utc>,
    /// Field `finished_at`.
    pub finished_at: DateTime<Utc>,
    /// Field `summary`.
    pub summary: String,
}

/// Canonical, signed binding between a proposal's patch/test-plan and an evaluation outcome.
///
/// This is the *only* structure that is signed/verified for `attestation`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationAttestedPayloadV1 {
    /// Field `proposal_id`.
    pub proposal_id: String,
    /// Field `patch_sha256_hex`.
    pub patch_sha256_hex: String,
    /// Field `test_plan_sha256_hex`.
    pub test_plan_sha256_hex: String,
    /// Field `results_sha256_hex`.
    pub results_sha256_hex: String,
    /// Field `ok`.
    pub ok: bool,
    /// Field `mode`.
    pub mode: String,
    /// Field `evaluated_at`.
    pub evaluated_at: DateTime<Utc>,
}

/// Evaluation payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiImproveEvaluationV1 {
    /// Field `proposal_id`.
    pub proposal_id: String,
    /// Field `ok`.
    pub ok: bool,

    /// SHA-256 hex of the proposed patch (unified diff string bytes).
    pub patch_sha256_hex: String,

    /// SHA-256 hex of the test plan (joined with `\n`).
    pub test_plan_sha256_hex: String,

    /// SHA-256 hex of the evaluation results (canonical JSON).
    pub results_sha256_hex: String,

    /// Execution mode (e.g. `local`, `docker`).
    pub mode: String,

    /// Per-command results.
    pub results: Vec<SandboxCommandResultV1>,

    /// Optional summary.
    pub summary: String,

    /// Canonical payload that is signed/verified with `attestation`.
    pub attested_payload: EvaluationAttestedPayloadV1,

    /// Optional cryptographic attestation of `attested_payload`.
    pub attestation: Option<AttestationV1>,

    /// Field `evaluated_at`.
    pub evaluated_at: DateTime<Utc>,
}

/// Approval decision.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
/// enum (documented for contract compliance).
pub enum ApprovalDecision {
    /// Variant `Approve`.
    Approve,
    /// Variant `Reject`.
    Reject,
}

/// Approval payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiImproveApprovalV1 {
    /// Field `proposal_id`.
    pub proposal_id: String,
    /// Field `decision`.
    pub decision: ApprovalDecision,
    /// Field `actor`.
    pub actor: String,
    /// Field `reason`.
    pub reason: Option<String>,
    /// Field `at`.
    pub at: DateTime<Utc>,
}

/// Apply request payload (issued after a human approval).
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct AiImproveApplyRequestV1 {
    /// Proposal id (uuidv7 as string).
    #[validate(length(min = 10))]
    pub proposal_id: String,

    /// Stable idempotency key for this apply request.
    ///
    /// Must remain stable across retries (reconciler, duplicate delivery).
    #[validate(length(min = 10))]
    pub request_id: String,

    /// Human-friendly title.
    #[validate(length(min = 3, max = 160))]
    pub title: String,

    /// Target path/module (e.g. `apps/ai-orchestrator`).
    #[validate(length(min = 1, max = 256))]
    pub target: String,

    /// Unified diff patch.
    #[validate(length(min = 10))]
    pub patch: String,

    /// SHA-256 hex of `patch` (unified diff string bytes).
    pub patch_sha256_hex: String,

    /// Commands to run as a verification plan before committing.
    #[validate(length(min = 1, max = 20))]
    pub test_plan: Vec<String>,

    /// Who approved / requested apply.
    #[validate(length(min = 1, max = 128))]
    pub actor: String,

    /// Canonical evaluation payload that was signed by the evaluator.
    pub evaluation_attested_payload: EvaluationAttestedPayloadV1,

    /// Attestation that proves the evaluation result came from a trusted evaluator.
    pub evaluation_attestation: Option<AttestationV1>,

    /// When this apply request was created.
    pub requested_at: DateTime<Utc>,
}

/// Apply result payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiImproveApplyResultV1 {
    /// Field `proposal_id`.
    pub proposal_id: String,

    /// The request idempotency key from the corresponding apply request.
    ///
    /// Backwards compatible: may be missing for results persisted before Phase 41.
    #[serde(default)]
    pub request_id: Option<String>,
    /// Field `ok`.
    pub ok: bool,

    /// Execution mode (e.g. `local`, `docker`).
    pub mode: String,

    /// The created branch name when successful.
    pub branch: Option<String>,

    /// The created commit hash when successful.
    pub commit: Option<String>,

    /// Per-command results.
    pub results: Vec<SandboxCommandResultV1>,

    /// Optional summary.
    pub summary: String,

    /// Field `applied_at`.
    pub applied_at: DateTime<Utc>,
}

/// Rollback request payload (manual or automatic).
///
/// This is used to revert a previously applied improvement (typically by reverting a commit).
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct AiImproveRollbackRequestV1 {
    /// Proposal id (uuidv7 as string).
    #[validate(length(min = 10))]
    pub proposal_id: String,

    /// Stable idempotency key for this rollback request.
    ///
    /// Must remain stable across retries (reconciler, duplicate delivery).
    #[validate(length(min = 10))]
    pub request_id: String,

    /// Actor initiating the rollback (human or system).
    #[validate(length(min = 1, max = 128))]
    pub actor: String,

    /// Commit hash that should be reverted.
    #[validate(length(min = 6, max = 64))]
    pub commit: String,

    /// Optional branch name to create for the rollback.
    pub branch: Option<String>,

    /// Optional reason.
    pub reason: Option<String>,

    /// Optional verification plan to run after revert (defaults to runner config if empty).
    pub verification_plan: Option<Vec<String>>,

    /// Field `requested_at`.
    pub requested_at: DateTime<Utc>,
}

/// Rollback result payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiImproveRollbackResultV1 {
    /// Field `proposal_id`.
    pub proposal_id: String,

    /// The request idempotency key from the corresponding rollback request.
    ///
    /// Backwards compatible: may be missing for results persisted before Phase 41.
    #[serde(default)]
    pub request_id: Option<String>,
    /// Field `ok`.
    pub ok: bool,

    /// Execution mode (e.g. `local`, `docker`).
    pub mode: String,

    /// The created rollback branch name when successful.
    pub branch: Option<String>,

    /// The created rollback commit hash when successful.
    pub commit: Option<String>,

    /// Per-command results.
    pub results: Vec<SandboxCommandResultV1>,

    /// Optional summary.
    pub summary: String,

    /// Field `rolled_back_at`.
    pub rolled_back_at: DateTime<Utc>,
}
