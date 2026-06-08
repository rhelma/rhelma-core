// crates/rhelma-cache/src/prelude.rs
//! Prelude module for easy imports

pub use crate::{backends::*, config::*, types::*, CacheError, CacheResult, CacheService};

pub use rhelma_core::prelude::*;
pub use rhelma_tracing::prelude::*;

/// Re-export commonly used traits
pub use async_trait::async_trait;
pub use serde::{Deserialize, Serialize};
pub use std::time::Duration;

/// Cache macros
pub use crate::{cached, cached_fn};
