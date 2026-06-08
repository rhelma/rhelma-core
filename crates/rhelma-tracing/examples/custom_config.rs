use rhelma_tracing::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut cfg = TracingConfig::default().with_service_name("example-custom".to_string());
    cfg.environment = "staging".into();
    cfg.sampling_rate = 0.5;

    let tracing = RhelmaTracing::init("example-custom", cfg).await?;

    if tracing.should_sample() {
        let span = instrument_span!("custom_example", "sampled" => true);
        let _guard = span.enter();
        info!("sampled span");
    } else {
        info!("request not sampled");
    }

    Ok(())
}
