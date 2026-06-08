//! RFC 7807 – Problem Details for HTTP APIs
//! Rhelma Contract compliant error representation.

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ProblemDetails {
    /// A URI reference that identifies the problem type.
    #[serde(rename = "type")]
    pub type_url: &'static str,

    /// A short, human-readable summary.
    pub title: &'static str,

    /// The HTTP status code.
    pub status: u16,

    /// A machine-readable error code (stable).
    pub code: &'static str,

    /// Optional human-readable detail.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[cfg(feature = "http")]
#[test]
fn residency_violation_problem_details() {
    let err = RhelmaError::residency_violation("eu-only tenant");
    let p = err.to_problem();

    assert_eq!(p.status, 451);
    assert_eq!(p.code, "RHELMA_451_001");
}
