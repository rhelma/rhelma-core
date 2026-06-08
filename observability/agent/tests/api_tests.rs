mod common;
use common::FakeEventBus;

use rhelma_observability_agent::{ObservabilityAgent, ObservabilityAgentConfig, ResidencyMode};
use std::sync::Arc;

#[test]
fn test_agent_initialization() {
    let bus = Arc::new(FakeEventBus::default());

    let cfg = ObservabilityAgentConfig {
        service_name: "svc".into(),
        environment: "development".into(),
        region: "eu".into(),
        service_version: "1.0.0".into(),
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

    let _agent = ObservabilityAgent::new(Arc::new(cfg), bus);
}
