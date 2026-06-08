//! Common string constants used across Rhelma services (Contract v5.2).

/// End-to-end request identifier header (UUIDv7 recommended).
pub const HEADER_MACH_REQUEST_ID: &str = "x-rhelma-request-id";

/// End-to-end correlation identifier header (UUIDv7 recommended).
pub const HEADER_MACH_CORRELATION_ID: &str = "x-rhelma-correlation-id";

/// Tenant identifier header.
pub const HEADER_TENANT_ID: &str = "x-tenant-id";

/// Region identifier header.
pub const HEADER_REGION: &str = "x-region";

/// Residency policy header.
pub const HEADER_RESIDENCY: &str = "x-residency";

/// W3C traceparent header.
pub const HEADER_TRACEPARENT: &str = "traceparent";

/// W3C tracestate header.
pub const HEADER_TRACESTATE: &str = "tracestate";

// W3C baggage header.
pub const HEADER_BAGGAGE: &str = "baggage";
