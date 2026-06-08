//! Trace→Metrics NOOP surface for rhelma-observability-core.
//!
//! NOTE:
//!   Real implementation (span duration metrics, anomaly scoring, AI signals)
//!   lives in rhelma-observability-agent.

use crate::WiredComponents;

/// Records span duration (NOOP in core)
///
/// # Arguments
/// * `_components` - Wired components (unused)
/// * `_span_name` - Span name (unused)
/// * `_duration_secs` - Duration in seconds (unused)
pub fn record_span_duration(_components: &WiredComponents, _span_name: &str, _duration_secs: f64) {
    // intentionally NOOP in core
}

/// Records trace event (NOOP in core)
///
/// # Arguments
/// * `_components` - Wired components (unused)
/// * `_event_name` - Event name (unused)
pub fn record_trace_event(_components: &WiredComponents, _event_name: &str) {
    // intentionally NOOP in core
}
