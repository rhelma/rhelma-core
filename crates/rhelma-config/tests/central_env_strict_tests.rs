use rhelma_config::CentralEnv;

mod common;

#[test]
fn strict_requires_env_vars() {
    common::with_env_lock(|| {
        std::env::remove_var("RHELMA_ENV");
        std::env::remove_var("RHELMA_ENVIRONMENT");
        std::env::remove_var("RHELMA_REGION");
        std::env::remove_var("RHELMA_SERVICE_VERSION");
        std::env::remove_var("RHELMA_TENANT_ID");

        let err = CentralEnv::from_env_strict().unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("missing required field"));
    })
}

#[test]
fn strict_trims_and_normalizes() {
    common::with_env_lock(|| {
        std::env::set_var("RHELMA_ENV", "  Production  ");
        std::env::set_var("RHELMA_REGION", "  eu-west-1 ");
        std::env::set_var("RHELMA_SERVICE_VERSION", " 1.2.3 ");
        std::env::remove_var("RHELMA_TENANT_ID");

        let central = CentralEnv::from_env_strict().unwrap();
        assert_eq!(central.environment, "production");
        assert_eq!(central.region, "eu-west-1");
        assert_eq!(central.service_version, "1.2.3");
    })
}

#[test]
fn strict_prod_rejects_local_region() {
    common::with_env_lock(|| {
        std::env::set_var("RHELMA_ENV", "production");
        std::env::set_var("RHELMA_REGION", "local");
        std::env::set_var("RHELMA_SERVICE_VERSION", "1.2.3");

        let err = CentralEnv::from_env_strict().unwrap_err();
        assert!(err.to_string().contains("must not be 'local'"));
    })
}
