use rhelma_logger::config::{BackpressureStrategy, DispatchMode, LoggerConfig};

#[test]
fn test_config_from_unified_balanced() {
    let unified = rhelma_config::UnifiedObservabilityConfig::baseline("test-svc".into());
    let cfg = LoggerConfig::from_unified(&unified, None);

    assert_eq!(cfg.dispatch_mode, DispatchMode::Async);
    assert_eq!(cfg.queue_capacity, 8192);
    assert_eq!(cfg.backpressure, BackpressureStrategy::DropNewest);
}

#[test]
fn test_config_from_unified_applies_core_overrides() {
    use secrecy::Secret;

    let mut unified = rhelma_config::UnifiedObservabilityConfig::baseline("test-svc".into());
    unified.json_enabled = false;
    unified.log_level = "info".into();

    let core = rhelma_config::CoreConfig {
        db_url: Secret::new("postgres://localhost/dev".into()),
        db_read_replica_url: None,
        db_max_connections: None,
        db_min_connections: None,
        redis_url: None,
        redis_default_ttl_secs: None,
        file_backend: rhelma_config::FileBackend::Local,
        file_local_root: None,
        file_s3_endpoint: None,
        file_s3_region: None,
        file_s3_bucket: None,
        obs_json_logs: true,
        obs_log_level: Some("error".into()),
        obs_otel_endpoint: None,
        obs_prometheus_port: None,
    };

    let cfg = LoggerConfig::from_unified(&unified, Some(&core));

    // Core overrides must win.
    assert!(cfg.json_enabled);
    assert_eq!(cfg.log_level, "error");
}
