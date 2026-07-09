//! Stage 11F — rhelma-event evidence contract tests.

use chrono::Utc;
use rhelma_event::evidence::{
    build_platform_event, decode_evidence_envelope, is_evidence_event_type, reject_if_secret,
    wrap_platform_event, CorrelationId, EvidenceBundle, EvidenceContext, Incident,
    IncidentCreatedV1, IncidentEvidenceCollectedV1, LogPattern, MetricWindow, Severity, Signal,
    SignalDetectedV1, TraceReference, PLATFORM_INCIDENT_CREATED_V1, PLATFORM_SIGNAL_DETECTED_V1,
};
use rhelma_event::{purpose, EventSource};

fn sample_signal() -> Signal {
    Signal {
        signal_id: "sig-1".into(),
        kind: "error_spike".into(),
        severity: Severity::Serious,
        correlation_id: CorrelationId::new("corr-1"),
        service: "api-gateway".into(),
        message: "5xx rate elevated".into(),
        detected_at: Utc::now(),
        metric: Some(MetricWindow {
            metric: "http_5xx_rate".into(),
            window_start: Utc::now(),
            window_end: Utc::now(),
            sample_count: 100,
            value: 0.2,
            baseline: Some(0.01),
            threshold: Some(0.05),
            aggregation: "rate".into(),
            unit: Some("ratio".into()),
        }),
        log_pattern: Some(LogPattern {
            pattern: "connection refused to <host>".into(),
            count: 42,
            level: "error".into(),
            first_seen: Utc::now(),
            last_seen: Utc::now(),
            example: None,
        }),
        trace: Some(TraceReference {
            trace_id: "trace-abc".into(),
            span_id: Some("span-1".into()),
            operation: Some("GET /v1/x".into()),
            duration_ms: Some(950.0),
        }),
    }
}

fn sample_incident() -> Incident {
    Incident {
        incident_id: "inc-1".into(),
        correlation_id: CorrelationId::new("corr-1"),
        service: "api-gateway".into(),
        title: "elevated 5xx".into(),
        severity: Severity::Serious,
        status: "open".into(),
        opened_at: Utc::now(),
        signal_ids: vec!["sig-1".into()],
    }
}

fn sample_bundle() -> EvidenceBundle {
    EvidenceBundle {
        bundle_id: "evb-1".into(),
        incident_id: "inc-1".into(),
        correlation_id: CorrelationId::new("corr-1"),
        service: "api-gateway".into(),
        collected_at: Utc::now(),
        metrics: vec![],
        log_patterns: vec![],
        traces: vec![],
        health: vec![],
        recent_deploys: vec![],
        diagnosis: None,
    }
}

#[test]
fn signal_payload_serde_roundtrips() {
    let payload = SignalDetectedV1 {
        signal: sample_signal(),
    };
    let json = serde_json::to_string(&payload).unwrap();
    let back: SignalDetectedV1 = serde_json::from_str(&json).unwrap();
    assert_eq!(payload, back);
}

#[test]
fn incident_payload_serde_roundtrips() {
    let payload = IncidentCreatedV1 {
        incident: sample_incident(),
    };
    let json = serde_json::to_string(&payload).unwrap();
    let back: IncidentCreatedV1 = serde_json::from_str(&json).unwrap();
    assert_eq!(payload, back);
}

#[test]
fn evidence_bundle_serde_roundtrips() {
    let payload = IncidentEvidenceCollectedV1 {
        evidence: sample_bundle(),
    };
    let json = serde_json::to_string(&payload).unwrap();
    let back: IncidentEvidenceCollectedV1 = serde_json::from_str(&json).unwrap();
    assert_eq!(payload, back);
}

#[test]
fn evidence_event_envelope_validates_and_decodes() {
    let source = EventSource::new("observability-agent", "1.0.0", "local");
    let payload = SignalDetectedV1 {
        signal: sample_signal(),
    };
    let ctx = EvidenceContext {
        correlation_id: Some("corr-1".into()),
        tenant_id: Some("acme".into()),
        ..Default::default()
    };
    let platform = build_platform_event(source.clone(), &payload, &ctx).unwrap();
    platform.validate().expect("platform envelope valid");
    assert_eq!(platform.event_type, PLATFORM_SIGNAL_DETECTED_V1);
    assert!(is_evidence_event_type(&platform.event_type));

    // Wrap for transport, then decode + validate back.
    let env = wrap_platform_event(&platform, source, purpose::OBSERVABILITY_AGENT).unwrap();
    assert_eq!(env.topic, PLATFORM_SIGNAL_DETECTED_V1);
    let decoded = decode_evidence_envelope(&env)
        .unwrap()
        .expect("is evidence");
    assert_eq!(decoded.correlation_id, "corr-1");
    assert_eq!(decoded.tenant_id.as_deref(), Some("acme"));
    decoded.validate().expect("decoded envelope valid");
}

#[test]
fn non_evidence_envelope_decodes_to_none() {
    let source = EventSource::new("agent", "1", "local");
    let payload = SignalDetectedV1 {
        signal: sample_signal(),
    };
    let platform =
        build_platform_event(source.clone(), &payload, &EvidenceContext::default()).unwrap();
    let mut env = wrap_platform_event(&platform, source, purpose::OBSERVABILITY_AGENT).unwrap();
    env.topic = "ai.improve.proposal".into();
    assert!(decode_evidence_envelope(&env).unwrap().is_none());
}

#[test]
fn sanitization_rejects_obvious_secrets() {
    // A signal whose message embeds a secret path is rejected.
    let mut signal = sample_signal();
    signal.message = "loaded creds from /app/.env".into();
    let payload = SignalDetectedV1 { signal };
    assert!(reject_if_secret(&payload).is_err());

    // A clean payload passes.
    let clean = SignalDetectedV1 {
        signal: sample_signal(),
    };
    assert!(reject_if_secret(&clean).is_ok());

    // A raw token key anywhere is rejected (nested in a diagnosis-like field).
    let bad_incident = IncidentCreatedV1 {
        incident: Incident {
            title: "aws_secret_access_key=AKIAEXAMPLE".into(),
            ..sample_incident()
        },
    };
    assert!(reject_if_secret(&bad_incident).is_err());
}

#[test]
fn event_type_constants_are_stable() {
    assert_eq!(PLATFORM_SIGNAL_DETECTED_V1, "platform.signal.detected.v1");
    assert_eq!(PLATFORM_INCIDENT_CREATED_V1, "platform.incident.created.v1");
}
