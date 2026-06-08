//! PII / Secret redaction for rhelma-logger.
//!
//! Design goals (global, production-grade standards):
//! - **Fail-safe**: prefer over-redaction to accidental leakage.
//! - **Non-reversible where possible** (hashing for certain identifiers).
//! - **Cheap** on the hot path (no heavy regex; mostly string checks).
//!
//! Notes:
//! - `redact()` returns `Some(new_value)` when the value should be replaced.
//! - The redactor should *never* panic.

use sha2::{Digest, Sha256};

/// Trait اصلی برای PII redaction.
/// انواعی که این trait را implement می‌کنند باید Thread-safe باشند.
pub trait PiiRedactor: Send + Sync {
    /// اگر مقدار باید redact شود، یک نسخه‌ی جدید از آن را برمی‌گرداند.
    fn redact(&self, key: &str, value: &str) -> Option<String>;

    /// جهت clone کردن trait object
    fn box_clone(&self) -> Box<dyn PiiRedactor + Send + Sync>;
}

/// Redactor پیش‌فرض.
/// - Passwords / tokens / secrets → ***REDACTED***
/// - Emails / user_id / session_id → sha256 (pseudonymous)
/// - Phone numbers → masked
/// - IP addresses → partially masked
#[derive(Debug, Clone, Default)]
pub struct DefaultPiiRedactor;

impl DefaultPiiRedactor {
    fn hash_sha256_hex(&self, input: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        let out = hasher.finalize();
        // hex without extra dependency
        let mut s = String::with_capacity(out.len() * 2);
        for b in out {
            s.push_str(&format!("{:02x}", b));
        }
        s
    }

    fn looks_like_email(&self, v: &str) -> bool {
        // Fast heuristic: one '@', at least one '.' after it, no spaces.
        if v.contains(' ') {
            return false;
        }
        let mut parts = v.split('@');
        let a = parts.next().unwrap_or("");
        let b = parts.next().unwrap_or("");
        if a.is_empty() || b.is_empty() || parts.next().is_some() {
            return false;
        }
        b.contains('.')
    }

    fn looks_like_jwt(&self, v: &str) -> bool {
        // Heuristic: three base64url-ish segments separated by '.'
        let v = v.trim();
        let mut parts = v.split('.');
        let a = parts.next().unwrap_or("");
        let b = parts.next().unwrap_or("");
        let c = parts.next().unwrap_or("");
        if a.is_empty() || b.is_empty() || c.is_empty() || parts.next().is_some() {
            return false;
        }
        // JWT segments are often 10s-100s chars. Keep it cheap.
        fn ok(seg: &str) -> bool {
            if seg.len() < 8 {
                return false;
            }
            seg.bytes()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == b'-' || ch == b'_' || ch == b'=')
        }
        ok(a) && ok(b) && ok(c)
    }

    fn looks_like_bearer(&self, v: &str) -> bool {
        let v = v.trim();
        if let Some(rest) = v.strip_prefix("Bearer ") {
            return rest.trim().len() >= 20;
        }
        false
    }

    fn looks_like_private_key_block(&self, v: &str) -> bool {
        v.contains("BEGIN") && v.contains("PRIVATE KEY") && v.contains("END")
    }

    fn looks_like_known_api_key_prefix(&self, v: &str) -> bool {
        let v = v.trim();
        // Common prefixes. This list is intentionally conservative.
        v.starts_with("sk-") // OpenAI-style
            || v.starts_with("xoxb-") // Slack bot token
            || v.starts_with("xoxa-")
            || v.starts_with("xoxp-")
            || v.starts_with("ghp_") // GitHub PAT
            || v.starts_with("github_pat_")
            || v.starts_with("AIza") // Google API key
            || v.starts_with("AKIA") // AWS access key id
    }

    fn looks_like_long_hex(&self, v: &str) -> bool {
        let v = v.trim();
        if v.len() < 32 {
            return false;
        }
        v.bytes().all(|ch| ch.is_ascii_hexdigit())
    }

    fn looks_like_long_base64ish(&self, v: &str) -> bool {
        let v = v.trim();
        if v.len() < 32 {
            return false;
        }
        // base64/base64url-ish heuristic
        v.bytes().all(|ch| {
            ch.is_ascii_alphanumeric()
                || ch == b'+'
                || ch == b'/'
                || ch == b'='
                || ch == b'-'
                || ch == b'_'
        })
    }

    fn mask_phone(&self, v: &str) -> String {
        // Keep last 2 digits if present; mask the rest.
        let digits: String = v.chars().filter(|c| c.is_ascii_digit()).collect();
        if digits.len() <= 2 {
            return "***REDACTED***".to_string();
        }
        let last2 = &digits[digits.len() - 2..];
        format!("********{last2}")
    }

    fn mask_ipv4(&self, v: &str) -> Option<String> {
        let parts: Vec<&str> = v.split('.').collect();
        if parts.len() != 4 {
            return None;
        }
        if parts.iter().all(|p| p.parse::<u8>().is_ok()) {
            return Some(format!("{}.{}.{}.x", parts[0], parts[1], parts[2]));
        }
        None
    }

    fn is_secret_key(&self, key_lc: &str) -> bool {
        key_lc.contains("password")
            || key_lc.contains("pass")
            || key_lc.contains("pwd")
            || key_lc.contains("token")
            || key_lc.contains("secret")
            || key_lc.contains("api_key")
            || key_lc.contains("authorization")
            || key_lc.contains("cookie")
            || key_lc.contains("private_key")
            || key_lc.contains("bearer")
    }

    fn is_hash_key(&self, key_lc: &str) -> bool {
        key_lc.contains("email") || key_lc.contains("user_id") || key_lc.contains("session_id")
    }

    fn is_phone_key(&self, key_lc: &str) -> bool {
        key_lc.contains("phone")
    }

    fn is_ip_key(&self, key_lc: &str) -> bool {
        key_lc.contains("ip") || key_lc.contains("client_ip")
    }
}

impl PiiRedactor for DefaultPiiRedactor {
    fn redact(&self, key: &str, value: &str) -> Option<String> {
        let key_lc = key.to_ascii_lowercase();

        // 1) Secrets: always hard-redact.
        if self.is_secret_key(&key_lc) {
            return Some("***REDACTED***".into());
        }

        // 2) Key-based hashing (pseudonymous).
        if self.is_hash_key(&key_lc) {
            let h = self.hash_sha256_hex(value);
            return Some(format!("sha256:{h}"));
        }

        // 3) Phone masking
        if self.is_phone_key(&key_lc) {
            return Some(self.mask_phone(value));
        }

        // 4) IP masking (best effort)
        if self.is_ip_key(&key_lc) {
            if let Some(m) = self.mask_ipv4(value) {
                return Some(m);
            }
            // Unknown/ipv6: do not leak
            return Some("***REDACTED***".into());
        }

        // 5) Value heuristics (fail-safe): if it *looks* like an email, hash it.
        if self.looks_like_email(value) {
            let h = self.hash_sha256_hex(value);
            return Some(format!("sha256:{h}"));
        }

        // 6) Value heuristics for secrets/tokens (fail-safe).
        // These are applied regardless of key, because accidental leakage is common.
        if self.looks_like_bearer(value)
            || self.looks_like_jwt(value)
            || self.looks_like_private_key_block(value)
            || self.looks_like_known_api_key_prefix(value)
            || self.looks_like_long_hex(value)
            || self.looks_like_long_base64ish(value)
        {
            return Some("***REDACTED***".into());
        }

        None
    }

    fn box_clone(&self) -> Box<dyn PiiRedactor + Send + Sync> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_redactor_secrets() {
        let r = DefaultPiiRedactor;

        assert_eq!(
            r.redact("user.password", "1234").as_deref(),
            Some("***REDACTED***")
        );
        assert_eq!(
            r.redact("auth.token", "abcd").as_deref(),
            Some("***REDACTED***")
        );
        assert_eq!(
            r.redact("authorization", "Bearer xxx").as_deref(),
            Some("***REDACTED***")
        );
    }

    #[test]
    fn default_redactor_hash_email() {
        let r = DefaultPiiRedactor;

        let out = r.redact("user.email", "a@b.com").unwrap();
        assert!(out.starts_with("sha256:"));
        assert!(out.len() > 10);
    }

    #[test]
    fn default_redactor_masks_phone() {
        let r = DefaultPiiRedactor;

        let out = r.redact("user.phone", "+49 151 2345 67").unwrap();
        assert!(out.starts_with("********"));
    }

    #[test]
    fn default_redactor_masks_ip() {
        let r = DefaultPiiRedactor;

        assert_eq!(
            r.redact("client_ip", "192.168.1.10").as_deref(),
            Some("192.168.1.x")
        );
    }

    #[test]
    fn default_redactor_value_heuristic_email() {
        let r = DefaultPiiRedactor;

        let out = r.redact("any.field", "user@example.com").unwrap();
        assert!(out.starts_with("sha256:"));
    }

    #[test]
    fn default_redactor_value_heuristic_bearer() {
        let r = DefaultPiiRedactor;

        let out = r
            .redact("any.field", "Bearer thisisaverylongtokenvalue1234567890")
            .unwrap();
        assert_eq!(out, "***REDACTED***");
    }

    #[test]
    fn default_redactor_value_heuristic_jwt() {
        let r = DefaultPiiRedactor;
        let jwt = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.sflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
        let out = r.redact("any.field", jwt).unwrap();
        assert_eq!(out, "***REDACTED***");
    }

    #[test]
    fn default_redactor_value_heuristic_known_prefix() {
        let r = DefaultPiiRedactor;
        let out = r
            .redact("any.field", "sk-1234567890abcdefghijklmnopqrstuvwxyz")
            .unwrap();
        assert_eq!(out, "***REDACTED***");
    }
}
