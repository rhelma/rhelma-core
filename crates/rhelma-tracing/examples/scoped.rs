use rhelma_config::UnifiedObservabilityConfig;
use rhelma_tracing::RhelmaTracing;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let service_name = "rhelma-tracing-scoped-example";

    // `UnifiedObservabilityConfig` no longer implements `Default`; use a baseline config.
    let unified = UnifiedObservabilityConfig::baseline(service_name.to_string());

    // Build a subscriber from the unified config, and install it for this scope.
    let tracing = RhelmaTracing::init_from_unified(service_name, &unified).await?;
    let subscriber = tracing.build_subscriber()?;
    let _guard = tracing::subscriber::set_default(subscriber);

    info!("Hello from a scoped RhelmaTracing subscriber!");
    Ok(())
}
