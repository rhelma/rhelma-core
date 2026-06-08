//! Unified result type for Rhelma Platform v5.1.
//!
//! # Examples
//!
//! ```
//! use rhelma_core::{RhelmaError, RhelmaResult};
//!
//! fn do_something() -> RhelmaResult<u32> {
//!     Ok(123)
//! }
//!
//! assert_eq!(do_something().unwrap(), 123);
//! ```

use crate::RhelmaError;

/// Primary result type used across the Rhelma platform.
pub type RhelmaResult<T> = Result<T, RhelmaError>;
