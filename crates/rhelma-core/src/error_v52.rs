//! Compatibility shim for the historical v5.2 error module name.
//!
//! The implementation lives in `error_envelope` so current service error
//! handling has one source of truth while existing imports keep compiling.

pub use crate::error_envelope::*;
