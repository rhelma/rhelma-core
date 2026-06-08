use crate::config::TracingConfig;
use opentelemetry::KeyValue;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{trace as sdktrace, Resource};
use std::time::Duration;
use tracing_subscriber::Layer;

/// Build an OpenTelemetry layer with an OTLP HTTP exporter.
///
/// When `otel_enabled` is false, this function is never called.
pub fn init_otel_layer<S>(
    cfg: &TracingConfig,
) -> Result<impl Layer<S> + Send + Sync, Box<dyn std::error::Error + Send + Sync>>
where
    S: tracing::Subscriber
        + for<'span> tracing_subscriber::registry::LookupSpan<'span>
        + Send
        + Sync,
{
    let endpoint = cfg
        .otel_endpoint
        .as_ref()
        .map(|u| u.to_string())
        .unwrap_or_else(|| "http://localhost:4318/v1/traces".to_string());

    let instance_id = detect_instance_id();

    let resource = Resource::new(vec![
        KeyValue::new("service.name", cfg.service_name.clone()),
        KeyValue::new("service.version", cfg.service_version.clone()),
        KeyValue::new("deployment.environment", cfg.environment.clone()),
        KeyValue::new("cloud.region", cfg.region.clone()),
        KeyValue::new("service.instance.id", instance_id),
    ]);

    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .http()
                .with_endpoint(endpoint)
                // Keep exports bounded; do not block shutdown/startup indefinitely.
                .with_timeout(Duration::from_secs(5)),
        )
        .with_trace_config(sdktrace::Config::default().with_resource(resource))
        .install_batch(opentelemetry_sdk::runtime::Tokio)?;

    let layer = tracing_opentelemetry::layer().with_tracer(tracer);
    Ok(layer)
}

fn detect_instance_id() -> String {
    std::env::var("RHELMA_INSTANCE_ID")
        .or_else(|_| std::env::var("POD_NAME"))
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "unknown".to_string())
}
