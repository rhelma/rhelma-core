//! Canonical JSON helpers for Rhelma events.
//!
//! We use canonical JSON for:
//! - `ops.audit*` payload hashing (sha256)
//! - deterministic signature verification
//!
//! Canonicalization rules:
//! - Objects: keys are sorted lexicographically (byte-wise) and values are canonicalized recursively.
//! - Arrays: order is preserved; elements are canonicalized recursively.
//! - Primitives: serialized as-is.
//! - When hashing audit payloads, we drop legacy/compat fields from the **payload object** if present:
//!   - `signature`
//!   - `chain_hash`

#![forbid(unsafe_code)]

use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

/// Produce a canonicalized JSON value.
///
/// This function is deterministic across platforms and serde_json versions.
pub fn canonicalize_json(v: &Value) -> Value {
    match v {
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();

            let mut out: Map<String, Value> = Map::new();
            for k in keys {
                if let Some(val) = map.get(k) {
                    out.insert(k.clone(), canonicalize_json(val));
                }
            }
            Value::Object(out)
        }
        Value::Array(arr) => Value::Array(arr.iter().map(canonicalize_json).collect()),
        _ => v.clone(),
    }
}

/// Return a clone of `payload` with legacy signing fields removed (if it is an object).
pub fn strip_audit_legacy_fields(payload: &Value) -> Value {
    match payload {
        Value::Object(map) => {
            let mut m = map.clone();
            m.remove("signature");
            m.remove("chain_hash");
            Value::Object(m)
        }
        _ => payload.clone(),
    }
}

/// Deterministic JSON string without extra whitespace.
///
/// This does **not** rely on serde_json's map ordering.
pub fn canonical_json_string(v: &Value) -> String {
    match v {
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {
            serde_json::to_string(v).expect("serde_json::Value is always serializable")
        }
        Value::Array(arr) => {
            let mut out = String::from("[");
            for (i, item) in arr.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&canonical_json_string(item));
            }
            out.push(']');
            out
        }
        Value::Object(obj) => {
            let mut keys: Vec<&String> = obj.keys().collect();
            keys.sort();

            let mut out = String::from("{");
            for (i, k) in keys.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&serde_json::to_string(k).expect("string keys are serializable"));
                out.push(':');
                out.push_str(&canonical_json_string(&obj[*k]));
            }
            out.push('}');
            out
        }
    }
}

/// Compute sha256(canonical_json(strip(signature, chain_hash, payload))).
pub fn sha256_canonical_audit_payload(payload: &Value) -> [u8; 32] {
    let stripped = strip_audit_legacy_fields(payload);
    let canonical = canonicalize_json(&stripped);
    let s = canonical_json_string(&canonical);
    let digest = Sha256::digest(s.as_bytes());
    digest.into()
}

/// Lowercase hex encoding of `sha256_canonical_audit_payload`.
pub fn canonical_audit_payload_hash_hex(payload: &Value) -> String {
    let digest = sha256_canonical_audit_payload(payload);
    hex::encode(digest)
}

/// Lowercase hex encoding of the canonical payload SHA-256 digest.
///
/// This is the generic alias for platform events. It intentionally reuses the
/// existing canonical audit hashing rules so event and audit hashes stay aligned
/// across crates.
pub fn canonical_payload_hash_hex(payload: &Value) -> String {
    canonical_audit_payload_hash_hex(payload)
}

/// Backward-compatible alias used by older modules/tests.
pub fn audit_payload_digest(payload: &Value) -> [u8; 32] {
    sha256_canonical_audit_payload(payload)
}
