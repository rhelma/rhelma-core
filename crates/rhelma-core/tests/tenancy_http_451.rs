#![cfg(feature = "http")]

use rhelma_core::prelude::*;

#[test]
fn residency_violation_maps_to_451() {
    // Create a residency violation error
    let err = RhelmaError::residency_violation("eu tenant cannot access us-west-2");

    // Verify it's detected as a residency violation
    assert!(err.is_residency_violation());

    // Verify the error string contains the residency code
    let err_string = err.to_string();
    assert!(err_string.contains("residency_violation:"));
    assert!(err_string.contains("eu tenant cannot access us-west-2"));

    // Verify it's a SecurityPolicy error variant
    assert!(matches!(err, RhelmaError::SecurityPolicy(_)));
}
