use rhelma_config::{CentralEnv, CentralRuntime};

mod common;

#[test]
fn env_model_v1_strict_requires_sentinel() {
    common::with_env_lock(|| {
        std::env::remove_var("RHELMA_ENV_MODEL_v1");
        std::env::set_var("RHELMA_ENV", "staging");
        std::env::set_var("RHELMA_REGION", "eu-west-1");
        std::env::set_var("RHELMA_SERVICE_VERSION", "1.2.3");

        assert!(CentralEnv::from_env_model_v1_strict().is_err());
    })
}

#[test]
fn central_runtime_strict_requires_service_name() {
    common::with_env_lock(|| {
        std::env::set_var("RHELMA_ENV", "production");
        std::env::set_var("RHELMA_REGION", "eu-west-1");
        std::env::set_var("RHELMA_SERVICE_VERSION", "1.2.3");
        std::env::remove_var("RHELMA_SERVICE_NAME");

        assert!(CentralRuntime::from_env_strict().is_err());
    })
}

#[test]
fn central_runtime_strict_ok_when_service_name_present() {
    common::with_env_lock(|| {
        std::env::set_var("RHELMA_ENV", "production");
        std::env::set_var("RHELMA_REGION", "eu-west-1");
        std::env::set_var("RHELMA_SERVICE_VERSION", "1.2.3");
        std::env::set_var("RHELMA_SERVICE_NAME", "svc-a");

        let rt = CentralRuntime::from_env_strict().unwrap();
        assert_eq!(rt.service_name, "svc-a");
        assert_eq!(rt.central.environment, "production");
    })
}
