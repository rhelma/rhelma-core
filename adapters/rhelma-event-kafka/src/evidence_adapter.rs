//! Central-side evidence adapter (Stage 11D).
//!
//! The AI orchestrator / central intelligence consumes evidence events from the
//! agent side over Kafka. It must depend only on **`rhelma-event` contracts** and
//! this **`rhelma-event-kafka` transport** — never on raw observability
//! internals.
//!
//! This adapter takes an inbound transport [`EventEnvelope`], recognizes the four
//! evidence event types, decodes + validates the embedded durable platform
//! event, and yields a typed [`EvidenceInput`] the orchestrator can triage.

use async_trait::async_trait;
use rhelma_event::evidence::{
    decode_evidence_envelope, is_evidence_event_type, EvidenceBundle, Incident, IncidentCreatedV1,
    IncidentEscalatedV1, IncidentEvidenceCollectedV1, Signal, SignalDetectedV1,
    PLATFORM_INCIDENT_CREATED_V1, PLATFORM_INCIDENT_ESCALATED_V1,
    PLATFORM_INCIDENT_EVIDENCE_COLLECTED_V1, PLATFORM_SIGNAL_DETECTED_V1,
};
use rhelma_event::{EventBusError, EventEnvelope};

/// Typed evidence handed to the AI orchestrator after adapting a Kafka event.
///
/// Each variant carries the decoded payload plus the correlation id so the
/// orchestrator can group evidence for one situation without re-parsing.
#[derive(Debug, Clone)]
pub enum EvidenceInput {
    /// A `platform.signal.detected.v1` observation.
    SignalDetected {
        /// Shared correlation id.
        correlation_id: String,
        /// The detected signal.
        signal: Signal,
    },
    /// A `platform.incident.created.v1` event.
    IncidentCreated {
        /// Shared correlation id.
        correlation_id: String,
        /// The opened incident.
        incident: Incident,
    },
    /// A `platform.incident.evidence_collected.v1` event.
    EvidenceCollected {
        /// Shared correlation id.
        correlation_id: String,
        /// The collected evidence bundle.
        evidence: EvidenceBundle,
    },
    /// A `platform.incident.escalated.v1` event.
    IncidentEscalated {
        /// Shared correlation id.
        correlation_id: String,
        /// The escalated incident.
        incident: Incident,
        /// Optional supporting evidence.
        evidence: Option<EvidenceBundle>,
    },
}

impl EvidenceInput {
    /// The correlation id common to every variant.
    pub fn correlation_id(&self) -> &str {
        match self {
            EvidenceInput::SignalDetected { correlation_id, .. }
            | EvidenceInput::IncidentCreated { correlation_id, .. }
            | EvidenceInput::EvidenceCollected { correlation_id, .. }
            | EvidenceInput::IncidentEscalated { correlation_id, .. } => correlation_id,
        }
    }

    /// The canonical `platform.*.v1` event type this input came from.
    pub fn event_type(&self) -> &'static str {
        match self {
            EvidenceInput::SignalDetected { .. } => PLATFORM_SIGNAL_DETECTED_V1,
            EvidenceInput::IncidentCreated { .. } => PLATFORM_INCIDENT_CREATED_V1,
            EvidenceInput::EvidenceCollected { .. } => PLATFORM_INCIDENT_EVIDENCE_COLLECTED_V1,
            EvidenceInput::IncidentEscalated { .. } => PLATFORM_INCIDENT_ESCALATED_V1,
        }
    }
}

/// Convert an inbound transport envelope into a typed [`EvidenceInput`].
///
/// - Returns `Ok(None)` for non-evidence traffic (unknown topic) so the central
///   consumer can ignore it safely.
/// - Returns `Err` when the envelope *claims* to be an evidence event but is
///   malformed (bad hash, wrong embedded payload) — the caller decides whether
///   to DLQ.
pub fn adapt_evidence_event(env: &EventEnvelope) -> Result<Option<EvidenceInput>, EventBusError> {
    let Some(platform) =
        decode_evidence_envelope(env).map_err(|e| EventBusError::Serialization(e.to_string()))?
    else {
        return Ok(None);
    };
    let correlation_id = platform.correlation_id.clone();

    let input = match platform.event_type.as_str() {
        PLATFORM_SIGNAL_DETECTED_V1 => {
            let p: SignalDetectedV1 = serde_json::from_value(platform.payload)
                .map_err(|e| EventBusError::Serialization(e.to_string()))?;
            EvidenceInput::SignalDetected {
                correlation_id,
                signal: p.signal,
            }
        }
        PLATFORM_INCIDENT_CREATED_V1 => {
            let p: IncidentCreatedV1 = serde_json::from_value(platform.payload)
                .map_err(|e| EventBusError::Serialization(e.to_string()))?;
            EvidenceInput::IncidentCreated {
                correlation_id,
                incident: p.incident,
            }
        }
        PLATFORM_INCIDENT_EVIDENCE_COLLECTED_V1 => {
            let p: IncidentEvidenceCollectedV1 = serde_json::from_value(platform.payload)
                .map_err(|e| EventBusError::Serialization(e.to_string()))?;
            EvidenceInput::EvidenceCollected {
                correlation_id,
                evidence: p.evidence,
            }
        }
        PLATFORM_INCIDENT_ESCALATED_V1 => {
            let p: IncidentEscalatedV1 = serde_json::from_value(platform.payload)
                .map_err(|e| EventBusError::Serialization(e.to_string()))?;
            EvidenceInput::IncidentEscalated {
                correlation_id,
                incident: p.incident,
                evidence: p.evidence,
            }
        }
        // decode_evidence_envelope already guarantees an evidence type, so this
        // is unreachable in practice; treat defensively as "not evidence".
        _ => return Ok(None),
    };
    Ok(Some(input))
}

/// A sink the central side implements to receive typed evidence.
#[async_trait]
pub trait EvidenceSink: Send + Sync {
    /// Consume one typed evidence input.
    async fn on_evidence(&self, input: EvidenceInput) -> Result<(), EventBusError>;
}

/// Bridges the raw Kafka [`FallibleEventHandler`](crate::consumer::FallibleEventHandler)
/// contract to a typed [`EvidenceSink`].
///
/// Non-evidence events are ignored (not an error). Malformed evidence events are
/// returned as errors so the subscriber's DLQ path can handle them.
pub struct EvidenceHandler<S: EvidenceSink> {
    sink: S,
}

impl<S: EvidenceSink> EvidenceHandler<S> {
    /// Wrap a sink as a Kafka event handler.
    pub fn new(sink: S) -> Self {
        Self { sink }
    }
}

#[async_trait]
impl<S: EvidenceSink> crate::consumer::FallibleEventHandler for EvidenceHandler<S> {
    async fn handle(&self, event: EventEnvelope) -> Result<(), EventBusError> {
        // Fast path: ignore non-evidence topics without deserializing payloads.
        if !is_evidence_event_type(&event.topic) {
            return Ok(());
        }
        match adapt_evidence_event(&event)? {
            Some(input) => self.sink.on_evidence(input).await,
            None => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rhelma_event::evidence::{
        build_platform_event, wrap_platform_event, CorrelationId, EvidenceContext, Severity, Signal,
    };
    use rhelma_event::{purpose, EventEnvelope, EventSource, PolicyMeta, Residency};
    use std::sync::Mutex;

    fn evidence_envelope() -> EventEnvelope {
        let source = EventSource::new("observability-agent", "1.0.0", "local");
        let payload = SignalDetectedV1 {
            signal: Signal {
                signal_id: "sig-9".into(),
                kind: "latency_spike".into(),
                severity: Severity::Serious,
                correlation_id: CorrelationId::new("corr-9"),
                service: "search-service".into(),
                message: "p95 latency elevated".into(),
                detected_at: chrono::Utc::now(),
                metric: None,
                log_pattern: None,
                trace: None,
            },
        };
        let ctx = EvidenceContext {
            correlation_id: Some("corr-9".into()),
            ..Default::default()
        };
        let platform = build_platform_event(source.clone(), &payload, &ctx).unwrap();
        wrap_platform_event(&platform, source, purpose::OBSERVABILITY_AGENT).unwrap()
    }

    #[test]
    fn adapts_valid_evidence_event() {
        let env = evidence_envelope();
        let input = adapt_evidence_event(&env).unwrap().expect("is evidence");
        assert_eq!(input.event_type(), PLATFORM_SIGNAL_DETECTED_V1);
        assert_eq!(input.correlation_id(), "corr-9");
        match input {
            EvidenceInput::SignalDetected { signal, .. } => {
                assert_eq!(signal.service, "search-service");
                assert_eq!(signal.severity, Severity::Serious);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn ignores_non_evidence_event() {
        let mut env = evidence_envelope();
        env.topic = "ai.improve.proposal".to_string();
        // Unknown topic → not evidence → None (safe ignore).
        assert!(adapt_evidence_event(&env).unwrap().is_none());
    }

    #[test]
    fn rejects_tampered_evidence_event() {
        // A valid evidence topic but a corrupted embedded payload must error,
        // not silently pass.
        let mut env = evidence_envelope();
        // Corrupt the embedded platform payload so hash validation fails.
        if let Some(obj) = env.payload.as_object_mut() {
            obj.insert(
                "event_type".to_string(),
                serde_json::Value::String(PLATFORM_SIGNAL_DETECTED_V1.to_string()),
            );
            obj.insert(
                "payload".to_string(),
                serde_json::json!({"signal": {"tampered": true}}),
            );
        }
        let err = adapt_evidence_event(&env).unwrap_err();
        assert!(matches!(err, EventBusError::Serialization(_)));
    }

    #[test]
    fn rejects_evidence_topic_with_garbage_payload() {
        let now = chrono::Utc::now();
        let env = EventEnvelope {
            event_id: "e1".into(),
            event_version: 1,
            topic: PLATFORM_SIGNAL_DETECTED_V1.to_string(),
            key: None,
            timestamp: now,
            published_at: now,
            source: EventSource::new("x", "1", "local"),
            request: Default::default(),
            trace: Default::default(),
            payload: serde_json::json!({"not": "a platform envelope"}),
            payload_type: "application/json".into(),
            schema_ref: "rhelma://schemas/platform.event@v1".into(),
            policy: PolicyMeta::public(purpose::AI_ORCH),
            residency: Residency::Global,
            encryption: None,
            signature: None,
            hash: None,
        };
        assert!(adapt_evidence_event(&env).is_err());
    }

    #[derive(Default)]
    struct RecordingSink {
        seen: Mutex<Vec<String>>,
    }

    #[async_trait]
    impl EvidenceSink for RecordingSink {
        async fn on_evidence(&self, input: EvidenceInput) -> Result<(), EventBusError> {
            self.seen
                .lock()
                .unwrap()
                .push(input.correlation_id().to_string());
            Ok(())
        }
    }

    #[tokio::test]
    async fn handler_routes_evidence_and_ignores_other() {
        use crate::consumer::FallibleEventHandler;
        let handler = EvidenceHandler::new(RecordingSink::default());

        // Evidence event → sink sees it.
        handler.handle(evidence_envelope()).await.unwrap();

        // Non-evidence event → ignored, no error.
        let mut other = evidence_envelope();
        other.topic = "obs.heartbeat".into();
        handler.handle(other).await.unwrap();

        assert_eq!(handler.sink.seen.lock().unwrap().as_slice(), &["corr-9"]);
    }
}
