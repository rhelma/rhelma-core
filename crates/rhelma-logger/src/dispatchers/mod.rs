pub mod file;
#[cfg(feature = "nats-dispatcher")]
pub mod nats;
#[cfg(feature = "redis-dispatcher")]
pub mod redis;
pub mod stdout;

#[cfg(feature = "io")]
pub mod file_service;

pub use stdout::StdoutDispatcher;
