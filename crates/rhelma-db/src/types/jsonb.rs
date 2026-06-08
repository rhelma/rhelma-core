//! JSONB helper type.
//!
//! We intentionally reuse sqlx's built-in JSON wrapper.
//! For Postgres, sqlx maps this to JSONB.
//!
//! Usage:
//!   let v: Jsonb<serde_json::Value> = Jsonb(json!({ "a": 1 }));
pub use sqlx::types::Json as Jsonb;
