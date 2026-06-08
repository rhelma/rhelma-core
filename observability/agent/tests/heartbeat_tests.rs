use rhelma_observability_agent::io::heartbeat::HeartbeatPayload;
use rhelma_observability_agent::{ObservabilityAgentConfig, ResidencyMode};

#[test]
fn test_heartbeat_serialization() {
    let cfg = ObservabilityAgentConfig {
        service_name: "svc".into(),
        environment: "development".into(),
        region: "eu".into(),
        service_version: "1".into(),
        heartbeat_interval_secs: 15,
        stale_threshold_secs: 120,
        anomaly_window_size: 10,
        residency_mode: ResidencyMode::Global,
        degraded_mode_initial: false,
        sampling_reduction_initial: false,
        command_enabled: true,
        decision_enabled: true,
        kafka_bootstrap: None,
        kafka_command_topic: None,
        kafka_decision_topic: None,
        kafka_group_id: None,
    };

    let hb = HeartbeatPayload::new(&cfg, "healthy".into());

    let v = serde_json::to_value(hb).unwrap();
    assert!(v.get("service").is_some());
}
