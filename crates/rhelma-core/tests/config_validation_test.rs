use rhelma_core::prelude::*;

mod common;

#[test]
fn config_allows_missing_service_name() {
    // Env is process-global; isolate to prevent flakes when tests run in parallel.
    common::with_isolated_prefix_env("Rhelma", || {
        // minimally valid env
        std::env::set_var("RHELMA_ENV", "production");
        std::env::set_var("RHELMA_REGION", "eu-west-1");

        let cfg = AppConfig::from_env_only().unwrap();

        // validate_all() must succeed
        cfg.validate_all().unwrap();

        // service_name is OPTIONAL by design
        assert!(cfg.service_name.is_none());
    })
}
