//! Extension traits for Rhelma core.
//!
//! This module centralizes extension traits that enhance core machinery:
//! - ErrorExt: add context to `RhelmaError`
//! - ResultExt: convenience helpers for working with `RhelmaResult`
//!
//! Services may define additional extensions in their own crates.

pub use crate::ErrorExt;

/// Optional: convenience blanket-impl for RhelmaResult
pub trait ResultExt<T>: Sized {
    /// Equivalent to `.map_err(|e| e.rhelma_context(ctx))`
    fn with_context<C>(self, ctx: C) -> Self
    where
        C: std::fmt::Display + Send + Sync + 'static;
}

impl<T> ResultExt<T> for Result<T, crate::error::RhelmaError> {
    fn with_context<C>(self, ctx: C) -> Self
    where
        C: std::fmt::Display + Send + Sync + 'static,
    {
        self.rhelma_context(ctx)
    }
}
