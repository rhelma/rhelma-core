use rhelma_core::prelude::*;

fn reset_env() {
    std::env::remove_var("RHELMA_ENV");
    std::env::remove_var("RHELMA_REGION");
    std::env::remove_var("RHELMA_SERVICE_NAME");
}

#[test]
fn config_from_env_only_uses_real_defaults() {
    reset_env();

    let cfg = AppConfig::from_env_only().unwrap();

    // According to rhelma-core defaults:
    // ENVIRONMENT = "development"
    // REGION = "local"
    // SERVICE_NAME = None

    assert_eq!(cfg.environment, "development");
    assert_eq!(cfg.region, "local");
    assert!(cfg.service_name.is_none());
}
