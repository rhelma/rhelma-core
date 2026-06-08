use criterion::{criterion_group, criterion_main, Criterion};
use rhelma_logger::builder::LogBuilder;
use rhelma_logger::config::{
    BackpressureStrategy, DispatchMode, Environment, LogFormat, LoggerConfig, PerformanceProfile,
};
use rhelma_logger::event::LogLevel;
use rhelma_logger::RhelmaLogger;

fn bench_logging(c: &mut Criterion) {
    let cfg = LoggerConfig {
        service_name: "bench".into(),
        service_version: "1".into(),
        service_instance_id: None,
        environment: Environment::Development,
        region: "local".into(),
        log_level: "info".into(),
        log_format: LogFormat::Json,
        json_enabled: true,
        console_enabled: false,
        sampling_rate: 1.0,
        performance_profile: PerformanceProfile::LowLatency,
        dispatch_mode: DispatchMode::Sync,
        queue_capacity: 1024,
        backpressure: BackpressureStrategy::DropNewest,
        flush_interval_ms: 0,
    };

    #[allow(deprecated)]
    {
        RhelmaLogger::init_with_config(&cfg).ok();
    }

    c.bench_function("log_info_basic", |b| {
        b.iter(|| {
            LogBuilder::new(LogLevel::Info, "hello from benchmark").emit();
        });
    });
}

criterion_group!(benches, bench_logging);
criterion_main!(benches);
