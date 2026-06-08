use rhelma_tracing::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let cfg = TracingConfig::default().with_service_name("example-basic".to_string());

    let _tracing = RhelmaTracing::init("example-basic", cfg).await?;

    let span = instrument_span!("basic_example");
    let _guard = span.enter();
    info!("hello from basic example");

    Ok(())
}
