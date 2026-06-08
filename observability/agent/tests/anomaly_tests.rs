mod common;
use common::FakeEventBus;

use std::sync::Arc;

use rhelma_observability_agent::agent::config::{ObservabilityAgentConfig, ResidencyMode};
use rhelma_observability_agent::reflex::anomaly::NaiveAnomalyDetector;
use rhelma_observability_agent::reflex::signals::SignalPayload;
use rhelma_observability_agent::ObservabilityAgent;

#[tokio::test]
async fn anomaly_window_is_capped() {
    let bus = Arc::new(FakeEventBus::default());

    let cfg = Arc::new(ObservabilityAgentConfig {
        service_name: "svc".into(),
        environment: "development".into(),
        region: "eu".into(),
        service_version: "1.0.0".into(),

        heartbeat_interval_secs: 15,
        stale_threshold_secs: 120,
        anomaly_window_size: 10_000,

        residency_mode: ResidencyMode::Global,
        degraded_mode_initial: false,
        sampling_reduction_initial: false,
        command_enabled: false,
        decision_enabled: false,

        kafka_bootstrap: None,
        kafka_command_topic: None,
        kafka_decision_topic: None,
        kafka_group_id: None,
    });

    // ObservabilityAgent برای effective_severity_sync لازم است
    let agent = ObservabilityAgent::new(cfg.clone(), bus.clone());

    // Detector
    let mut det = NaiveAnomalyDetector::new_with_bus(cfg.clone(), bus.clone());

    // با process_signal_sync پرش می‌کنیم تا cap داخلی خود detector اعمال شود
    for _ in 0..25_000 {
        let s = SignalPayload::new(
            "latency_spike",
            "warning",
            "latency spiked in p95",
            serde_json::json!({"p95_ms": 450}),
        );

        let _ = det.process_signal_sync(&agent, s).unwrap();
    }

    assert!(det.window_len_for_test() <= 10_000);
}
