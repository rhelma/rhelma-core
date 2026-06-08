//! Resolver smoke tests.
//!
//! Note: these tests are intentionally lightweight and only assert stable, public API
//! to keep the workspace green under `-D warnings`.

#![forbid(unsafe_code)]

use rhelma_config::UnifiedObservabilityConfig;

#[test]
fn baseline_sets_service_name() {
    let cfg = UnifiedObservabilityConfig::baseline("s1".to_string());
    assert_eq!(cfg.service_name, "s1");
}
