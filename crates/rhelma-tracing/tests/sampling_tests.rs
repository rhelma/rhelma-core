use rhelma_tracing::should_sample;

#[test]
fn should_sample_extremes() {
    assert!(!should_sample(0.0));
    assert!(!should_sample(-1.0));
    // rate >= 1.0 is always true
    assert!(should_sample(1.0));
    assert!(should_sample(2.0));
}
