#![forbid(unsafe_code)]

//! Documentation helpers for the API Gateway routes module.
//!
//! The canonical implementations live in `crate::docs` (so they can be
//! referenced from other places without pulling in the routes module).
//! This file simply re-exports the handlers so `routes::build_router` can
//! mount them.

pub use crate::docs::{docs_landing, openapi_json};
