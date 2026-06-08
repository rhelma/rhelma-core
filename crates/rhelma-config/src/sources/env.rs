//! Environment-based config sources for Rhelma observability.

use once_cell::sync::OnceCell;
use serde_json::{Map, Value};

use crate::deprecation;
use crate::errors::{ConfigError, ConfigResult};
use crate::merge::flattened_to_nested;

static DEPRECATED_OBS_PREFIX_WARNED: OnceCell<()> = OnceCell::new();

fn warn_deprecated_obs_prefix_once() {
    deprecation::warn_once(
        &DEPRECATED_OBS_PREFIX_WARNED,
        "RHELMA_OBSERVABILITY__* is deprecated; use RHELMA_OBS__* instead",
    );
}

/// Read an observability env var.
///
/// Prefers canonical `primary`, but still supports `deprecated` with a warn-once message.
pub fn obs_var(primary: &str, deprecated: &str) -> Option<String> {
    if let Ok(v) = std::env::var(primary) {
        return Some(v);
    }
    if let Ok(v) = std::env::var(deprecated) {
        warn_deprecated_obs_prefix_once();
        return Some(v);
    }
    None
}

/// Load RHELMA_OBS__* / RHELMA_OBSERVABILITY__* env overrides into a JSON object.
pub fn load_env_overrides() -> ConfigResult<Value> {
    // The number of RHELMA_OBS__ keys is typically small; pre-allocate to reduce rehashing.
    let mut flat: Map<String, Value> = Map::with_capacity(16);

    for (key, value) in std::env::vars() {
        let (suffix, used_deprecated) = if let Some(rest) = key.strip_prefix("RHELMA_OBS__") {
            (Some(rest), false)
        } else if let Some(rest) = key.strip_prefix("RHELMA_OBSERVABILITY__") {
            (Some(rest), true)
        } else {
            (None, false)
        };

        let suffix = match suffix {
            Some(s) => s,
            None => continue,
        };

        if used_deprecated {
            warn_deprecated_obs_prefix_once();
        }

        let logical = suffix.to_ascii_lowercase().replace("__", ".");

        // Typing heuristics: *_PORT -> integer, *_ENABLED -> bool.
        let v: Value = if logical.ends_with("_port") || logical.ends_with(".port") {
            match value.parse::<u64>() {
                Ok(n) => Value::from(n),
                Err(_) => {
                    return Err(ConfigError::InvalidValue {
                        field: "env",
                        message: format!("invalid integer for {key}={value:?}"),
                    });
                }
            }
        } else if logical.ends_with("_enabled") || logical.ends_with(".enabled") {
            let l = value.to_ascii_lowercase();
            let b = match l.as_str() {
                "1" | "true" | "yes" | "on" => true,
                "0" | "false" | "no" | "off" => false,
                _ => {
                    return Err(ConfigError::InvalidValue {
                        field: "env",
                        message: format!("invalid boolean for {key}={value:?}"),
                    });
                }
            };
            Value::from(b)
        } else {
            Value::from(value)
        };

        flat.insert(logical, v);
    }

    Ok(flattened_to_nested(flat))
}
