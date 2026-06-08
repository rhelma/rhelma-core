use rhelma_logger::{
    log_audit, log_error, log_heartbeat, log_info, BackpressureStrategy, DispatchMode, Environment,
    LogFormat, LoggerConfig, PerformanceProfile, RhelmaLogger,
};

fn main() {
    let cfg = LoggerConfig {
        service_name: "example".into(),
        service_version: "1.0.0".into(),
        service_instance_id: None,
        environment: Environment::Development,
        region: "local".into(),

        log_level: "info".into(),
        log_format: LogFormat::Json,
        json_enabled: true,
        console_enabled: true,

        sampling_rate: 1.0,
        performance_profile: PerformanceProfile::Balanced,
        dispatch_mode: DispatchMode::Sync,
        backpressure: BackpressureStrategy::DropNewest,

        queue_capacity: 128,
        flush_interval_ms: 0,
    };

    // نصب سراسری logger
    #[allow(deprecated)]
    RhelmaLogger::init_with_config(&cfg).expect("logger init failed");

    // ----------------
    // Logging Examples
    // ----------------

    // ساده
    log_info!("hello from example");

    // با فیلدهای ساختار یافته
    log_info!("processing request", "user.id" => 42, "path" => "/api/v1");

    // خطا
    log_error!("something failed", "reason" => "bad input");

    // Audit (Rhelma v5.1.1)
    log_audit!(
        "user updated profile",
        "user",
        "update_profile",
        "user_profile",
        "42",
        "changed_field" => "email"
    );

    // Heartbeat
    log_heartbeat!("service alive");
}
