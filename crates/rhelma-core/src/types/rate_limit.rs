//! Rate limit key builder for Rhelma Platform v5.1.
//!
//! This type produces **deterministic, sanitized** keys suitable for Redis,
//! KV stores, and metrics labels. It is the canonical implementation used
//! across Rhelma services.

use crate::types::common::sanitize_key_part;
use crate::types::{RegionId, TenantId, UserId};

/// Builder for strong, deterministic rate limit keys.
///
/// Keys are constructed in the following shape:
///
/// ```text
/// rl|prefix=<namespace>|action=<action>|tenant=<tenant_id>|region=<region>|user=<user_uuid>|extra=...
/// ```
///
/// - All dynamic parts are sanitized to be safe for infrastructure backends.
/// - Missing parts (tenant/region/user/extra) are simply omitted.
#[derive(Debug, Clone, Default)]
#[must_use]
pub struct RateLimitKeyBuilder {
    prefix: String,
    user: Option<UserId>,
    tenant: Option<TenantId>,
    region: Option<RegionId>,
    extra: Vec<String>,
}

impl RateLimitKeyBuilder {
    /// Create a new builder with the given logical namespace/prefix.
    ///
    /// The prefix is sanitized and stored as `prefix=<value>` in the key.
    pub fn new(prefix: &str) -> Self {
        Self {
            prefix: sanitize_key_part(prefix),
            user: None,
            tenant: None,
            region: None,
            extra: vec![],
        }
    }

    /// Attach a user identifier to the key.
    pub fn with_user(mut self, user: UserId) -> Self {
        self.user = Some(user);
        self
    }

    /// Attach a tenant identifier to the key.
    pub fn with_tenant(mut self, tenant: TenantId) -> Self {
        self.tenant = Some(tenant);
        self
    }

    /// Attach a region identifier to the key.
    pub fn with_region(mut self, region: RegionId) -> Self {
        self.region = Some(region);
        self
    }

    /// Attach an arbitrary extra, sanitized value.
    ///
    /// This can be used for things like:
    /// - endpoint names
    /// - logical buckets (`"login"`, `"search"`, ...)
    pub fn with_extra<S: Into<String>>(mut self, extra: S) -> Self {
        self.extra.push(sanitize_key_part(&extra.into()));
        self
    }

    /// Build the final key for a given logical action.
    ///
    /// The `action` is sanitized and always included.
    pub fn build(self, action: &str) -> String {
        let mut parts = vec![
            "rl".to_string(),
            format!("prefix={}", self.prefix),
            format!("action={}", sanitize_key_part(action)),
        ];

        if let Some(t) = self.tenant {
            parts.push(format!("tenant={}", sanitize_key_part(t.as_str())));
        }

        if let Some(r) = self.region {
            parts.push(format!("region={}", sanitize_key_part(r.as_str())));
        }

        if let Some(u) = self.user {
            parts.push(format!("user={}", u.as_uuid()));
        }

        for e in self.extra {
            parts.push(format!("extra={}", e));
        }

        parts.join("|")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn builds_basic_key() {
        let key = RateLimitKeyBuilder::new("auth").build("login");

        assert!(key.starts_with("rl|prefix=auth|action=login"));
    }

    #[test]
    fn sanitizes_forbidden_chars() {
        let key = RateLimitKeyBuilder::new("au:th")
            .with_extra("path=/login")
            .build("lo?gin");

        assert!(!key.contains(':'));
        assert!(!key.contains('/'));
        assert!(!key.contains('?'));
    }

    #[test]
    fn includes_all_optional_parts() {
        let tenant = TenantId("tenant-1".to_string());
        let region = RegionId("eu-west-1".to_string());
        let user = UserId(Uuid::nil());

        let key = RateLimitKeyBuilder::new("auth")
            .with_tenant(tenant.clone())
            .with_region(region.clone())
            .with_user(user)
            .build("login");

        assert!(key.contains("tenant=tenant-1"));
        assert!(key.contains("region=eu-west-1"));
        assert!(key.contains("user=00000000-0000-0000-0000-000000000000"));
    }
}
