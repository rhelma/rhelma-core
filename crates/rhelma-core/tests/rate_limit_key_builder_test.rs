use rhelma_core::prelude::*;
use rhelma_core::types::UserId;

#[test]
fn rate_limit_key_sanitizes_and_constructs_correctly() {
    let tenant = TenantId::parse("tenant-1").unwrap();
    let user = UserId::new();

    let key = RateLimitKeyBuilder::new("login")
        .with_tenant(tenant)
        .with_user(user)
        .with_extra("region=eu-west-1")
        .build("auth:attempt");

    assert!(key.contains("tenant=tenant-1"));
    assert!(key.contains("user="));

    // '=' → '_' after sanitization
    assert!(key.contains("extra=region_eu-west-1"));

    // 'auth:attempt' becomes 'auth_attempt'
    assert!(key.contains("action=auth_attempt"));
}
