#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Extract changed paths from a unified diff.
///
/// This parser is intentionally minimal and only looks at `diff --git` and
/// `+++ b/...` headers.
pub fn changed_paths(patch: &str) -> Vec<String> {
    let mut set: BTreeSet<String> = BTreeSet::new();

    for line in patch.lines() {
        if let Some(rest) = line.strip_prefix("diff --git ") {
            let parts: Vec<&str> = rest.split_whitespace().collect();
            if parts.len() >= 2 {
                if let Some(p) = parts[1].strip_prefix("b/") {
                    if !p.is_empty() {
                        set.insert(p.to_string());
                    }
                }
            }
            continue;
        }

        if let Some(rest) = line.strip_prefix("+++ ") {
            let rest = rest.trim();
            if rest == "/dev/null" {
                continue;
            }
            if let Some(p) = rest.strip_prefix("b/") {
                if !p.is_empty() {
                    set.insert(p.to_string());
                }
            }
        }
    }

    set.into_iter().collect()
}

/// Validate changed paths against allow/deny prefix lists.
///
/// - Every path must be relative (no leading `/`) and must not contain `..`.
/// - When `allowed_prefixes` is non-empty, every path must match at least one allowed prefix.
/// - No path may match a forbidden prefix.
pub fn validate_paths(
    paths: &[String],
    allowed_prefixes: &[String],
    forbidden_prefixes: &[String],
) -> Result<(), String> {
    for p in paths {
        if p.starts_with('/') {
            return Err(format!("absolute path not allowed: {p}"));
        }
        if p.contains("..") {
            return Err(format!("path traversal not allowed: {p}"));
        }

        if forbidden_prefixes
            .iter()
            .any(|pref| !pref.is_empty() && p.starts_with(pref))
        {
            return Err(format!("path is forbidden by policy: {p}"));
        }

        if !allowed_prefixes.is_empty()
            && !allowed_prefixes
                .iter()
                .any(|pref| !pref.is_empty() && p.starts_with(pref))
        {
            return Err(format!("path not allowlisted by policy: {p}"));
        }
    }

    Ok(())
}
