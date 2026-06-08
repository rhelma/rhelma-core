#![forbid(unsafe_code)]

//! Library facade for `social-service`.
//!
//! This service owns the "social/news" domain (posts, comments, reactions) and
//! exposes an HTTP API aligned with Rhelma request-context headers.

pub mod config;
pub mod error;
pub mod middleware;
pub mod models;
pub mod routes;
pub mod state;

pub use config::SocialConfig;
pub use state::AppState;
