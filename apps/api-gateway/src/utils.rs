use std::time::{SystemTime, UNIX_EPOCH};

/// Generate a unique ID.
pub fn generate_id() -> String {
    uuid::Uuid::now_v7().to_string()
}

/// Date/time utilities.
pub fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
