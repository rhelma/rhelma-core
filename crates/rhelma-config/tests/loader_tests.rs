use serde::Deserialize;

use rhelma_config::loader::load_with_prefix;

mod common;

#[derive(Debug, Default, Deserialize)]
struct TestServiceCfg {
    port: u16,
}

#[test]
fn load_with_prefix_defaults_only_when_empty() {
    common::with_isolated_prefix_env("TEST", || {
        // Required by CoreConfig::from_env
        std::env::set_var("RHELMA_DB__URL", "postgres://user:pass@localhost:5432/db");

        // Ensure prefixed config is absent (the isolation helper already cleared TEST__*)
        std::env::remove_var("TEST__PORT");

        let cfg = load_with_prefix::<TestServiceCfg>("TEST").expect("should load");
        assert_eq!(cfg.service.port, 0);
    })
}

#[test]
fn load_with_prefix_errors_on_invalid_value() {
    common::with_isolated_prefix_env("TEST", || {
        std::env::set_var("RHELMA_DB__URL", "postgres://user:pass@localhost:5432/db");

        // Provide an invalid value that cannot deserialize into u16
        std::env::set_var("TEST__PORT", "not-a-number");

        let err = load_with_prefix::<TestServiceCfg>("TEST").unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.to_lowercase().contains("failed to deserialize")
                || msg.to_lowercase().contains("parse error")
        );
    })
}
