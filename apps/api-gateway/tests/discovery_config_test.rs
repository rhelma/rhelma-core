#![forbid(unsafe_code)]

use std::time::Duration;

use api_gateway::config::GatewayConfig;

/// This test validates the *new* gateway knobs:
/// - RHELMA_CONTROL_SERVICE_URL
/// - RHELMA_GATEWAY_DISCOVERY_CACHE_TTL_SECONDS
///
/// It only checks config parsing; no network calls.
#[test]
fn parses_discovery_config_from_env() {
    // Keep test isolated: set the minimal set of env vars required by CentralEnv/CoreConfig.
    std::env::set_var("RHELMA_ENV", "test");
    std::env::set_var("RHELMA_REGION", "local");
    std::env::set_var("RHELMA_SERVICE_VERSION", "0.0.0-test");

    std::env::set_var(
        "RHELMA_DB__URL",
        "postgres://postgres:postgres@127.0.0.1:5432/rhelma",
    );
    std::env::set_var("RHELMA_REDIS__URL", "redis://127.0.0.1:6379");

    // Upstreams required by api-gateway config
    std::env::set_var("RHELMA_AUTH_SERVICE_URL", "http://127.0.0.1:7000");
    std::env::set_var("RHELMA_SEARCH_SERVICE_URL", "http://127.0.0.1:7001");
    std::env::set_var("RHELMA_USER_SERVICE_URL", "http://127.0.0.1:7002");
    std::env::set_var("RHELMA_AI_SERVICE_URL", "http://127.0.0.1:7003");

    // New bits
    std::env::set_var("RHELMA_CONTROL_SERVICE_URL", "http://127.0.0.1:8086");
    std::env::set_var("RHELMA_GATEWAY_DISCOVERY_CACHE_TTL_SECONDS", "42");

    let cfg = GatewayConfig::from_env().expect("gateway config");

    assert_eq!(
        cfg.services.control_service_url.as_deref(),
        Some("http://127.0.0.1:8086")
    );
    assert_eq!(cfg.discovery_cache_ttl, Duration::from_secs(42));
}
