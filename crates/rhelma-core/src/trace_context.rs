//!
//! Core Requirements:
//! - Full W3C `traceparent` support
//! - Graceful fallback to legacy headers (`x-trace-id`, `x-span-id`)
//! - `generate()` MUST create valid W3C-compliant identifiers
//! - `extract_from_headers()` MUST NEVER panic
//! - Always backend-agnostic (no OTEL dependencies here)
//!
//! W3C traceparent format:
//!   version   = 2 hex chars (00)
//!   trace-id  = 32 hex chars (16 bytes) and MUST NOT be all-zero
//!   span-id   = 16 hex chars (8 bytes) and MUST NOT be all-zero
//!   flags     = 2 hex chars (sampling bit in LSB)

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceContext {
    /// 32-hex trace identifier (W3C)
    pub trace_id: Option<String>,
    /// 16-hex span identifier (W3C)
    pub span_id: Option<String>,
}

impl TraceContext {
    pub fn new(trace_id: Option<String>, span_id: Option<String>) -> Self {
        Self { trace_id, span_id }
    }

    /// Generate a fully valid W3C trace + span pair.
    pub fn generate() -> Self {
        Self {
            trace_id: Some(Self::generate_trace_id()),
            span_id: Some(Self::generate_span_id()),
        }
    }

    fn generate_trace_id() -> String {
        // UUID v4 -> 32 hex chars (remove hyphens)
        Uuid::new_v4().to_string().replace('-', "")
    }

    fn generate_span_id() -> String {
        // UUID v4 -> 32 hex chars; take first 16 for span-id
        let raw = Uuid::new_v4().to_string().replace('-', "");
        raw[..16].to_string()
    }

    /// Parse a W3C `traceparent` header.
    /// Example: "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01"
    ///
    /// Strict rules (W3C):
    /// - version must be 2 hex chars
    /// - trace-id must be 32 hex chars and NOT all-zero
    /// - span-id must be 16 hex chars and NOT all-zero
    /// - flags must be 2 hex chars
    pub fn from_traceparent(header: &str) -> Option<Self> {
        let parts: Vec<&str> = header.split('-').collect();
        if parts.len() != 4 {
            return None;
        }

        let version = parts[0].trim();
        let trace = parts[1].trim();
        let span = parts[2].trim();
        let flags = parts[3].trim();

        if version.len() != 2 || flags.len() != 2 {
            return None;
        }
        if !Self::is_hex(version) || !Self::is_hex(flags) {
            return None;
        }

        if trace.len() != 32 || span.len() != 16 {
            return None;
        }
        if !Self::is_hex(trace) || !Self::is_hex(span) {
            return None;
        }

        // W3C: all-zero IDs are invalid
        if Self::is_all_zeros(trace) || Self::is_all_zeros(span) {
            return None;
        }

        Some(Self {
            trace_id: Some(trace.to_ascii_lowercase()),
            span_id: Some(span.to_ascii_lowercase()),
        })
    }

    /// Build W3C traceparent header.
    pub fn to_traceparent(&self) -> Option<String> {
        Some(format!(
            "00-{}-{}-01",
            self.trace_id.as_ref()?,
            self.span_id.as_ref()?
        ))
    }

    /// Zero-trust header extraction.
    ///
    /// Rules:
    /// - If `traceparent` exists but is invalid: generate a fresh pair (do not fall back)
    /// - If only a span-id is present (orphan): generate a fresh pair
    /// - If only a trace-id is present: generate span-id
    pub fn extract_from_headers<'a, H>(headers: H) -> Self
    where
        H: IntoIterator<Item = (&'a str, &'a str)>,
    {
        let mut trace_id: Option<String> = None;
        let mut span_id: Option<String> = None;

        let mut saw_any_trace_header = false;
        let mut saw_traceparent = false;

        for (k, v) in headers {
            let key = k.trim().to_ascii_lowercase();
            let value = v.trim();

            match key.as_str() {
                "traceparent" => {
                    saw_any_trace_header = true;
                    saw_traceparent = true;

                    if let Some(tp) = Self::from_traceparent(value) {
                        return tp;
                    }
                }

                // legacy + internal
                "x-trace-id" | "x-rhelma-trace-id" => {
                    saw_any_trace_header = true;
                    trace_id = Self::normalize_trace_id(value);
                }
                "x-span-id" | "x-rhelma-span-id" => {
                    saw_any_trace_header = true;
                    span_id = Self::normalize_span_id(value);
                }

                _ => {}
            }
        }

        if saw_traceparent {
            return Self::generate();
        }
        if !saw_any_trace_header {
            return Self::generate();
        }
        if trace_id.is_none() && span_id.is_none() {
            return Self::generate();
        }
        if trace_id.is_none() && span_id.is_some() {
            // orphan span-id is untrusted
            return Self::generate();
        }
        if trace_id.is_some() && span_id.is_none() {
            span_id = Some(Self::generate_span_id());
        }

        Self { trace_id, span_id }
    }

    pub fn current_trace_id(&self) -> Option<&str> {
        self.trace_id.as_deref()
    }

    fn normalize_trace_id(s: &str) -> Option<String> {
        let v = s.trim();
        if v.len() != 32 {
            return None;
        }
        if !Self::is_hex(v) {
            return None;
        }
        if Self::is_all_zeros(v) {
            return None;
        }
        Some(v.to_ascii_lowercase())
    }

    fn normalize_span_id(s: &str) -> Option<String> {
        let v = s.trim();
        if v.len() != 16 {
            return None;
        }
        if !Self::is_hex(v) {
            return None;
        }
        if Self::is_all_zeros(v) {
            return None;
        }
        Some(v.to_ascii_lowercase())
    }

    fn is_hex(s: &str) -> bool {
        s.chars().all(|c| c.is_ascii_hexdigit())
    }

    fn is_all_zeros(s: &str) -> bool {
        s.chars().all(|c| c == '0')
    }
}

#[cfg(test)]
mod tests {
    use super::TraceContext;

    #[test]
    fn w3c_traceparent_roundtrip() {
        let ctx = TraceContext::generate();
        let tp = ctx.to_traceparent().unwrap();
        let parsed = TraceContext::from_traceparent(&tp).unwrap();

        assert_eq!(parsed.trace_id.unwrap().len(), 32);
        assert_eq!(parsed.span_id.unwrap().len(), 16);
    }

    #[test]
    fn rejects_all_zero_ids() {
        let bad = "00-00000000000000000000000000000000-0000000000000000-01";
        assert!(TraceContext::from_traceparent(bad).is_none());
    }
}
