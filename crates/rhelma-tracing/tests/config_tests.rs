use rhelma_tracing::TracingConfig;

#[test]
fn invalid_sampling_rate_is_rejected() {
    // Above 1.0 should be rejected.
    let cfg = TracingConfig {
        sampling_rate: 1.5,
        ..Default::default()
    };
    assert!(cfg.validate().is_err());

    // Below 0.0 should be rejected.
    let cfg = TracingConfig {
        sampling_rate: -0.1,
        ..Default::default()
    };
    assert!(cfg.validate().is_err());
}

#[test]
fn valid_sampling_rate_is_accepted() {
    let cfg = TracingConfig {
        sampling_rate: 0.5,
        ..Default::default()
    };
    assert!(cfg.validate().is_ok());
}
