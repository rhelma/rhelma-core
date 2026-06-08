use rhelma_config::validation::validate_base;
use rhelma_config::{CentralEnv, UnifiedObservabilityConfig};

mod common;

#[test]
fn unified_from_central_env_basic() {
    common::with_env_lock(|| {
        std::env::set_var("RHELMA_REGION", "eu-west-1");
        std::env::set_var("RHELMA_ENV", "production");
        std::env::set_var("RHELMA_SERVICE_VERSION", "1.2.3");

        // In production, OTEL is required by default. Provide a well-formed endpoint so
        // the base validator can succeed without relying on the developer's machine env.
        std::env::set_var("RHELMA_OBS__OTEL_ENDPOINT", "http://localhost:4317");

        let central = CentralEnv::from_env();
        let cfg = UnifiedObservabilityConfig::from_central_env(&central, "test-service");

        assert_eq!(cfg.service_name, "test-service");
        assert_eq!(cfg.region, "eu-west-1");
        match cfg.environment {
            rhelma_config::Environment::Production => {}
            _ => panic!("expected Production environment"),
        }

        validate_base(&cfg).unwrap();
    })
}
