#![forbid(unsafe_code)]

pub mod error;
pub mod metrics;
pub mod migrations;
pub mod policy;
pub mod pool;
pub mod repository;
pub mod tracing_ext;
pub mod transaction;
pub mod types;

// فقط foundation exports:
pub use error::{DbError, DbResult};
pub use pool::Database;
pub use repository::{BaseRepository, GovernanceRepository};

#[cfg(feature = "observability-config")]
pub mod config_provider;
#[cfg(feature = "observability-config")]
pub mod models;
#[cfg(feature = "observability-config")]
pub use config_provider::DbConfigProvider;

pub mod builder;
pub use builder::{DatabaseBuilder, DbConnectConfig};
