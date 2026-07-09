#![forbid(unsafe_code)]

use rhelma_event::EventBusError;

/// Strict topic validation shared across producer/consumer/DLQ.
///
/// Policy alignment:
/// - No wildcard / glob / regex-like characters (`*`, `^`, etc.)
/// - Only ASCII [A-Za-z0-9._-]
pub fn validate_topic_strict(topic: &str) -> Result<(), EventBusError> {
    let t = topic.trim();
    if t.is_empty() {
        return Err(EventBusError::Transport(
            "empty topic is not allowed".into(),
        ));
    }

    // Forbidden patterns (wildcards/regex operators and common broker-routing glyphs).
    const FORBIDDEN: &[char] = &[
        '*', '?', '#', '>', '^', '$', '[', ']', '{', '}', '(', ')', '|', '\\',
    ];
    if t.chars().any(|c| FORBIDDEN.contains(&c)) {
        return Err(EventBusError::Transport(format!(
            "invalid topic: wildcard/regex patterns are not allowed: '{t}'"
        )));
    }

    if !t
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-')
    {
        return Err(EventBusError::Transport(format!(
            "invalid topic: only [A-Za-z0-9._-] allowed: '{t}'"
        )));
    }

    Ok(())
}

/// Resolve final topic name using a configured prefix.
///
/// - The caller passes the *logical* topic (without prefix).
/// - If it's already prefixed, we keep it as-is.
/// - We validate both the raw and resolved topic names.
pub fn resolve_topic(prefix: &str, raw: &str) -> Result<String, EventBusError> {
    let raw = raw.trim();
    validate_topic_strict(raw)?;

    let out = if prefix.is_empty() || raw.starts_with(prefix) {
        raw.to_string()
    } else {
        format!("{prefix}{raw}")
    };

    // Defensive: ensure prefix didn't introduce invalid characters.
    validate_topic_strict(&out)?;
    Ok(out)
}
