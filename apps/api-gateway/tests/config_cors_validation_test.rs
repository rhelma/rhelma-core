#![forbid(unsafe_code)]

use api_gateway::config::GatewayConfig;

mod common;

fn set_env(k: &str, v: &str) {
    std::env::set_var(k, v);
}

#[test]
fn cors_wildcard_with_credentials_is_rejected() {
    // Env is process-global; isolate to prevent flakes when tests run in parallel.
    common::with_isolated_prefix_env("Rhelma", || {
        // Minimal CoreConfig requirements
        set_env("RHELMA_ENV", "production");
        set_env("RHELMA_REGION", "eu-west-1");
        set_env("RHELMA_SERVICE_VERSION", "1.0.0-test");
        set_env("RHELMA_DB__URL", "postgres://user:pass@127.0.0.1:5432/db");

        // CORS misconfig
        set_env("RHELMA_GATEWAY_CORS_ALLOWED_ORIGINS", "*");
        set_env("RHELMA_GATEWAY_CORS_ALLOW_CREDENTIALS", "true");

        let cfg = GatewayConfig::from_env();
        assert!(cfg.is_err());
    })
}

#[test]
fn cors_wildcard_in_production_is_rejected() {
    common::with_isolated_prefix_env("Rhelma", || {
        set_env("RHELMA_ENV", "production");
        set_env("RHELMA_REGION", "eu-west-1");
        set_env("RHELMA_SERVICE_VERSION", "1.0.0-test");
        set_env("RHELMA_DB__URL", "postgres://user:pass@127.0.0.1:5432/db");

        set_env("RHELMA_GATEWAY_CORS_ALLOWED_ORIGINS", "*");
        set_env("RHELMA_GATEWAY_CORS_ALLOW_CREDENTIALS", "false");

        let cfg = GatewayConfig::from_env();
        assert!(cfg.is_err());
    })
}
