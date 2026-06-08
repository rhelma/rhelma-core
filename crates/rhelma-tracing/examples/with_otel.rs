use rhelma_tracing::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Requires: --features otel at build time.
    let mut cfg = TracingConfig::default().with_service_name("example-otel".to_string());
    cfg.otel_enabled = true;

    let _tracing = RhelmaTracing::init("example-otel", cfg).await?;

    let span = instrument_span!("otel_example", "example" => "with_otel");
    let _guard = span.enter();
    info!("hello from otel example");

    Ok(())
}
