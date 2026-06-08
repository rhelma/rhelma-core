use rhelma_config::{Environment, LogFormat, PerformanceProfile, UnifiedObservabilityConfig};
use rhelma_observability_core::{to_logger_config, to_metrics_config, to_tracing_config};

fn base_cfg() -> UnifiedObservabilityConfig {
    UnifiedObservabilityConfig {
        service_name: "test-service".into(),
        environment: Environment::Development,
        region: "local".into(),
        service_version: "0.1.0".into(),
        log_level: "info".into(),
        log_format: LogFormat::Json,
        json_enabled: true,
        console_enabled: false,
        sampling_rate: 1.0,
        performance_profile: PerformanceProfile::Balanced,
        otel_enabled: false,
        otel_required: false,
        otel_endpoint: None,
        enable_metrics: true,
        prometheus_port: 9090,
    }
}

#[test]
fn logger_config_sanitizes_log_level() {
    let mut cfg = base_cfg();
    cfg.log_level = "not-a-level".into();

    let logger_cfg = to_logger_config(&cfg);
    assert_eq!(logger_cfg.log_level, "info");
}

#[test]
fn tracing_config_invalid_endpoint_fails_validation() {
    let mut cfg = base_cfg();
    cfg.otel_enabled = true;
    cfg.otel_endpoint = Some("::::::bad-url".into());

    let tracing_cfg = to_tracing_config(&cfg);
    assert!(tracing_cfg.validate().is_err());
}

#[test]
fn metrics_config_contains_core_labels() {
    let cfg = base_cfg();
    let m = to_metrics_config(&cfg);

    let labels: std::collections::BTreeMap<_, _> = m.default_labels.iter().cloned().collect();

    assert_eq!(
        labels.get("service_name"),
        Some(&"test-service".to_string())
    );
    assert_eq!(labels.get("region"), Some(&"local".to_string()));
    assert_eq!(labels.get("service_version"), Some(&"0.1.0".to_string()));
}
