//! JSON merge helpers for rhelma-config.

use serde_json::{Map, Value};

/// Recursively merge `override_` into `base`.
///
/// Objects are merged key-by-key, while all other types are replaced.
pub fn deep_merge(base: Value, override_: Value) -> Value {
    match (base, override_) {
        (Value::Object(mut b), Value::Object(o)) => {
            for (k, v) in o {
                let existing = b.remove(&k);
                let merged = match existing {
                    Some(prev) => deep_merge(prev, v),
                    None => v,
                };
                b.insert(k, merged);
            }
            Value::Object(b)
        }
        (_, o) => o,
    }
}

/// Convert flattened keys ("logger.level") into a nested JSON object.
pub fn flattened_to_nested(flat: Map<String, Value>) -> Value {
    let mut root = Map::new();

    for (key, value) in flat {
        insert_nested(&mut root, &key, value);
    }

    Value::Object(root)
}

/// SAFE و borrow-checker-friendly:
/// "a.b.c" را به صورت تو در تو در Map می‌نویسد.
pub fn insert_nested(root: &mut Map<String, Value>, path: &str, value: Value) {
    let parts: Vec<&str> = path.split('.').collect();
    insert_nested_parts(root, &parts, value);
}

fn insert_nested_parts(current: &mut Map<String, Value>, parts: &[&str], value: Value) {
    if parts.is_empty() {
        return;
    }

    if parts.len() == 1 {
        current.insert(parts[0].to_string(), value);
        return;
    }

    let key = parts[0].to_string();

    let entry = current
        .entry(key)
        .or_insert_with(|| Value::Object(Map::new()));

    match entry {
        Value::Object(ref mut child_map) => {
            insert_nested_parts(child_map, &parts[1..], value);
        }
        _ => {
            *entry = Value::Object(Map::new());
            if let Value::Object(ref mut child_map) = entry {
                insert_nested_parts(child_map, &parts[1..], value);
            }
        }
    }
}
