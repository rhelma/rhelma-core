use rhelma_tracing::prelude::*;

#[tokio::test]
async fn init_tracing_works() {
    let cfg = TracingConfig::default().with_service_name("test-service".into());
    let result = RhelmaTracing::init("test-service", cfg).await;
    assert!(result.is_ok());
}
