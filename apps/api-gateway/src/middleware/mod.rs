#![forbid(unsafe_code)]

pub mod timeout;

pub mod auth_extractor;
pub mod auth_layer;
pub mod cors;
pub mod error_envelope;
pub mod http_metrics;
pub mod observability;
pub mod rate_limit;
pub mod rbac;
pub mod request_guard;

#[allow(unused_imports)]
pub use auth_extractor::{AuthUserExtractor, OptionalAuthUserExtractor};

pub use cors::*;
pub use error_envelope::error_envelope_middleware;
pub use http_metrics::http_metrics_middleware;
pub use observability::observability_middleware;
pub use rate_limit::rate_limit_middleware;
pub use request_guard::request_guard_middleware;

pub use timeout::timeout_middleware;
