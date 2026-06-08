//! Database migrations (foundation-level)

pub use sqlx::migrate::Migrator;

/// Migrator for this crate.
/// Migration files must live in `crates/rhelma-db/migrations/*.sql`
pub fn migrator() -> Migrator {
    sqlx::migrate!("./migrations")
}
