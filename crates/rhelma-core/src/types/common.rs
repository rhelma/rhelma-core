//! Common utilities for Rhelma core types.
//!
//! This module intentionally contains only small, reusable helpers.
//! The canonical rate limit key builder lives in `types::rate_limit`.

use crate::types::rate_limit::RateLimitKeyBuilder as CanonicalRateLimitKeyBuilder;

/// Characters we consider unsafe in infrastructure keys (Redis, KV, logs).
pub const FORBIDDEN_KEY_CHARS: &[char] = &[
    ':', ';', '/', '\\', '#', '?', '@', '%', '{', '}', '[', ']', '(', ')', ',', ' ', '\t', '\n',
    '\r', '=', '&',
];

/// Sanitize a key part by replacing forbidden characters with `_`.
///
/// This is safe to use for:
/// - Redis keys
/// - KV stores
/// - Log correlation keys
pub fn sanitize_key_part(s: &str) -> String {
    s.chars()
        .map(|c| {
            if FORBIDDEN_KEY_CHARS.contains(&c) {
                '_'
            } else {
                c
            }
        })
        .collect()
}

/// Backwards-compatible alias for the canonical `RateLimitKeyBuilder`.
///
/// Historically this module defined its own `RateLimitKeyBuilder`.
/// As of Rhelma v5.1.1 the implementation has been unified in
/// `crate::types::rate_limit::RateLimitKeyBuilder`.
///
/// Any existing imports of `crate::types::common::RateLimitKeyBuilder`
/// will continue to work via this type alias.
pub type RateLimitKeyBuilder = CanonicalRateLimitKeyBuilder;
