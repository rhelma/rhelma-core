//! ai module — AI coordination layer for Observability-Agent (Rhelma v5.2)

pub mod decision;
pub mod incident;

// ─────────────────────────────────────────────
// Incoming from AI-Orchestrator
// ─────────────────────────────────────────────

pub use decision::{apply_ai_decision, AiDecisionResult, AiIncidentDecision, TOPIC_AI_DECISION};

// ─────────────────────────────────────────────
// Outgoing to AI-Orchestrator
// ─────────────────────────────────────────────

pub use incident::{AiIncidentProposed, TOPIC_AI_INCIDENT_PROPOSED};
