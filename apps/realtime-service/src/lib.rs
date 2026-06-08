#![forbid(unsafe_code)]

// Library facade for realtime-service.
// This enables integration tests (and internal reuse) without depending on the binary entrypoint.

pub mod auth;
pub mod config;
pub mod error;
pub mod eventing;
pub mod metrics_endpoint;
pub mod metrics_layer;
pub mod middleware;
pub mod presence;
pub mod rooms;
pub mod routes;
pub mod state;
pub mod ws;

pub use config::RealtimeConfig;
pub use state::AppState;
