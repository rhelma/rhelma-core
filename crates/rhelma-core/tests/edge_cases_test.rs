use chrono::{Duration, Utc};
use rhelma_core::prelude::*;
use rhelma_core::{Email, PageRequest, Paginated, RegionId, RhelmaError, TenantId, UserId};

//
// ------------------------------
// EMAIL TESTS
// ------------------------------
//

#[test]
fn email_redaction_with_plus_tag() {
    let email = Email::parse("alice+tag@example.co.uk").unwrap();
    assert_eq!(email.redacted(), "a***@example.co.uk");
}

#[test]
fn email_redaction_single_char_local_part() {
    let email = Email::parse("a@example.com").unwrap();
    assert_eq!(email.redacted(), "a***@example.com");
}

//
// ------------------------------
// PAGINATION TESTS
// ------------------------------
//

#[test]
fn pagination_offset_overflow_does_not_panic() {
    let page = PageRequest::new(u64::MAX - 10, 100);
    let next = page.next_offset();

    assert!(next.is_none());
}

#[test]
fn pagination_zero_limit_normalized_to_default() {
    let page = PageRequest::new(0, 0);
    let normalized = page.normalized();
    assert_eq!(normalized.offset, 0);
    assert_eq!(normalized.limit, 20);
}

#[test]
fn pagination_total_pages_on_huge_total_is_safe() {
    let paginated = Paginated {
        items: vec![1u32; 50],
        total: u64::MAX,
        offset: 0,
        limit: 50,
    };

    let pages = paginated.total_pages();
    assert!(pages > 0);
}

//
// ------------------------------
// STRONG-ID TESTS
// ------------------------------
//

#[test]
fn tenant_id_min_length_and_charset_rules() {
    assert!(TenantId::parse("abc").is_ok());
    assert!(TenantId::parse("a-b").is_ok());
    assert!(TenantId::parse("ab").is_err());
    assert!(TenantId::parse("Aaa").is_err());
    assert!(TenantId::parse("acme corp").is_err());
}

#[test]
fn region_id_common_formats() {
    assert!(RegionId::parse("us-east-1").is_ok());
    assert!(RegionId::parse("eu-west-1").is_ok());
    assert!(RegionId::parse("ap-southeast-2").is_ok());
    assert!(RegionId::parse("local").is_ok());

    assert!(RegionId::parse("US-EAST-1").is_err());
    assert!(RegionId::parse("us_east_1").is_err());
}

//
// ------------------------------
// ERROR CONTEXT TESTS
// ------------------------------
//

#[test]
fn error_context_preserves_base_message_and_adds_context() {
    let res: Result<(), RhelmaError> = Err(RhelmaError::NotFound("Invoice 123".into()));

    let err = res
        .rhelma_context("while fetching")
        .rhelma_context("while handling request")
        .unwrap_err();

    let msg = err.to_string();
    assert!(msg.contains("Invoice 123"));
    assert!(msg.contains("while fetching"));
    assert!(msg.contains("while handling request"));
}

//
// ------------------------------
// REQUEST CONTEXT TEST
// ------------------------------
//

#[test]
fn request_context_builder_chaining_works() {
    let tenant = TenantId::parse("acme").unwrap();
    let region = RegionId::parse("eu-west-1").unwrap();

    let ctx = RequestContext::empty()
        .with_tenant(tenant)
        .with_region(region)
        .add_scope("read:data")
        .add_scope("write:data")
        .add_role("admin")
        .add_role("user")
        .with_locale("en-US");

    assert!(ctx.has_tenant());
    assert!(ctx.has_region());
    assert_eq!(ctx.locale(), Some("en-US"));
}

//
// ------------------------------
// PASSWORD POLICY
// ------------------------------
//

#[test]
fn password_policy_accepts_reasonable_default_password() {
    let policy = PasswordPolicy::default();
    assert!(policy.validate("Aa1!bcde").is_ok());
}

#[test]
fn password_policy_rejects_too_short_password() {
    let policy = PasswordPolicy::default();
    assert!(policy.validate("Aa1!").is_err());
}

//
// ------------------------------
// RATE LIMIT KEY
// ------------------------------
//

#[test]
fn rate_limit_key_differs_for_different_users() {
    let user1 = UserId::new();
    let user2 = UserId::new();

    let key1 = RateLimitKeyBuilder::new("api")
        .with_user(user1)
        .build("login");

    let key2 = RateLimitKeyBuilder::new("api")
        .with_user(user2)
        .build("login");

    assert_ne!(key1, key2);
}

#[test]
fn rate_limit_key_sanitizes_dangerous_chars() {
    let key = RateLimitKeyBuilder::new("api:core")
        .with_extra("path=/v1/resource?id=123;drop")
        .build("login");

    assert!(!key.contains(':'));
    assert!(!key.contains(';'));
    assert!(!key.contains('/'));
    assert!(key.contains("api_core"));
}

//
// ------------------------------
// REALTIME METADATA TESTS
// ------------------------------
//

#[test]
fn connection_metadata_stale_detection_behaves_as_expected() {
    use rhelma_core::realtime_types::{ConnectionMetadata, RealtimeSessionId};

    let now = Utc::now();
    let meta = ConnectionMetadata {
        session_id: RealtimeSessionId::new(),
        user_id: UserId::new(),
        tenant_id: Some(TenantId::parse("acme").unwrap()),
        region: Some(RegionId::parse("local").unwrap()),
        connected_at: now - Duration::seconds(400),
        last_seen_at: now - Duration::seconds(400),
        user_agent: None,
        ip: None,
    };

    assert!(meta.is_stale(now, 300));
    assert!(!meta.is_stale(now, 500));
}

#[test]
fn connection_metadata_stale_detection_default_timeout() {
    use rhelma_core::realtime_types::{ConnectionMetadata, RealtimeSessionId};

    let now = Utc::now();
    let meta = ConnectionMetadata {
        session_id: RealtimeSessionId::new(),
        user_id: UserId::new(),
        tenant_id: Some(TenantId::parse("acme").unwrap()),
        region: Some(RegionId::parse("local").unwrap()),
        connected_at: now - Duration::seconds(400),
        last_seen_at: now - Duration::seconds(400),
        user_agent: None,
        ip: None,
    };

    assert!(meta.is_stale_default(now));
}
