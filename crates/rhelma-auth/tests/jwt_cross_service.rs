use rhelma_auth::types::{Role, SessionId, UserPrincipal};
/// Test that a token issued by "api-gateway" can be verified by "social-service"
/// when both share the same RHELMA_AUTH_ISSUER, RHELMA_AUTH_AUDIENCE and keys.
use rhelma_auth::{AuthConfig, AuthService};
use rhelma_core::prelude::{TenantId, UserId};
use uuid::Uuid;

fn gw_cfg(redis: &str) -> AuthConfig {
    // Simulate: api-gateway with RHELMA_AUTH_ISSUER/AUDIENCE set
    std::env::set_var("RHELMA_AUTH_ISSUER", "rhelma-asrnegar-auth");
    std::env::set_var("RHELMA_AUTH_AUDIENCE", "rhelma-asrnegar");
    std::env::set_var("RHELMA_AUTH_REDIS_PREFIX", "rhelma:asrnegar:auth");
    std::env::set_var(
        "RHELMA_AUTH_JWT_PRIVATE_KEY_B64",
        "MC4CAQAwBQYDK2VwBCIEIG5eoevHK/2EMu/bMMNuSOA5O9lLAGkE6aEb3RLDOOT0",
    );
    std::env::set_var(
        "RHELMA_AUTH_JWT_PUBLIC_KEY_B64",
        "MCowBQYDK2VwAyEALg/WjM8sx3qkZyfNXL3De7Z3wTQNOfKwgOyRiCMhGmU=",
    );
    AuthConfig::from_env("api-gateway-asrnegar", "development", Some(redis.into())).unwrap()
}

fn svc_cfg(redis: &str) -> AuthConfig {
    // Simulate: social-service with same RHELMA_AUTH_ISSUER/AUDIENCE
    // env vars already set by gw_cfg above since they share the process
    AuthConfig::from_env("social-asrnegar", "development", Some(redis.into())).unwrap()
}

fn redis_url() -> String {
    std::env::var("RHELMA_AUTH_TEST_REDIS_URL")
        .or_else(|_| std::env::var("RHELMA_REDIS__URL"))
        .unwrap_or_else(|_| "redis://127.0.0.1:6379/0".to_string())
}

#[tokio::test]
async fn raw_jwt_decode_error() {
    let redis_url = redis_url();
    let gw = AuthService::new(gw_cfg(&redis_url)).await.expect("gw init");

    let principal = UserPrincipal {
        user_id: UserId(Uuid::new_v4()),
        tenant_id: Some(TenantId("asrnegar".into())),
        session_id: SessionId::new(),
        roles: vec![Role("user".into())],
        permissions: vec![],
    };
    let pair = gw.issue_for_principal(&principal).await.expect("issue");
    let token = &pair.access_token;

    // Decode directly via jsonwebtoken to get the raw error
    use base64::Engine as _;
    use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};

    let pub_b64 = "MCowBQYDK2VwAyEALg/WjM8sx3qkZyfNXL3De7Z3wTQNOfKwgOyRiCMhGmU=";
    let pub_der = base64::engine::general_purpose::STANDARD
        .decode(pub_b64)
        .unwrap();
    let key = DecodingKey::from_ed_der(&pub_der);

    // Test 1: validate aud + iss (same as social-service does)
    let mut v = Validation::new(Algorithm::EdDSA);
    v.set_audience(&["rhelma-asrnegar"]);
    v.set_issuer(&["rhelma-asrnegar-auth"]);
    v.validate_exp = true;
    let r1 = decode::<serde_json::Value>(token, &key, &v);
    println!(
        "With aud+iss validation: {:?}",
        r1.err().map(|e| format!("{:?}", e.kind()))
    );

    // Test 2: disable ALL validation
    let mut v2 = Validation::new(Algorithm::EdDSA);
    v2.insecure_disable_signature_validation();
    v2.required_spec_claims = std::collections::HashSet::new();
    v2.validate_exp = false;
    let r2 = decode::<serde_json::Value>(token, &key, &v2);
    println!(
        "With no validation (signature disabled): {:?}",
        r2.map(|t| t.claims.to_string())
    );

    // Test 3: only signature (no iss/aud check)
    let mut v3 = Validation::new(Algorithm::EdDSA);
    v3.required_spec_claims = std::collections::HashSet::new();
    v3.validate_exp = false;
    let r3 = decode::<serde_json::Value>(token, &key, &v3);
    println!(
        "Signature only (no aud/iss): {:?}",
        r3.err().map(|e| format!("{:?}", e.kind()))
    );

    // Test 4: use raw 32-byte key (strip SubjectPublicKeyInfo 12-byte header)
    let raw_pub = &pub_der[12..];
    let key_raw = DecodingKey::from_ed_der(raw_pub);
    let mut v4 = Validation::new(Algorithm::EdDSA);
    v4.required_spec_claims = std::collections::HashSet::new();
    v4.validate_exp = false;
    let r4 = decode::<serde_json::Value>(token, &key_raw, &v4);
    println!(
        "Raw 32-byte key (no aud/iss): {:?}",
        r4.err().map(|e| format!("{:?}", e.kind()))
    );
    println!(
        "Raw key success: {}",
        decode::<serde_json::Value>(token, &key_raw, &v4).is_ok()
    );

    // Test 5: raw key + aud+iss set
    let mut v5 = Validation::new(Algorithm::EdDSA);
    v5.set_audience(&["rhelma-asrnegar"]);
    v5.set_issuer(&["rhelma-asrnegar-auth"]);
    v5.validate_exp = false;
    let r5 = decode::<serde_json::Value>(token, &key_raw, &v5);
    println!(
        "Raw key + aud+iss: {:?}",
        r5.err().map(|e| format!("{:?}", e.kind()))
    );
    println!(
        "Raw key + aud+iss SUCCESS: {}",
        decode::<serde_json::Value>(token, &key_raw, &v5).is_ok()
    );

    // What the actual token aud value is
    let parts: Vec<&str> = token.split('.').collect();
    let payload_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(parts[1])
        .unwrap();
    let raw_claims: serde_json::Value = serde_json::from_slice(&payload_bytes).unwrap();
    println!("token aud raw value: {:?}", raw_claims.get("aud"));
    println!("token iss raw value: {:?}", raw_claims.get("iss"));
}

#[tokio::test]
async fn token_issued_by_gateway_accepted_by_social_service() {
    let redis_url = redis_url();

    let gw = AuthService::new(gw_cfg(&redis_url))
        .await
        .expect("gateway auth init");
    let ss = AuthService::new(svc_cfg(&redis_url))
        .await
        .expect("social auth init");

    // Issue token as gateway would
    let principal = UserPrincipal {
        user_id: UserId(Uuid::new_v4()),
        tenant_id: Some(TenantId("asrnegar".into())),
        session_id: SessionId::new(),
        roles: vec![Role("user".into())],
        permissions: vec![],
    };
    let pair = gw
        .issue_for_principal(&principal)
        .await
        .expect("issue token");
    println!("access_token issued: {}...", &pair.access_token[..40]);

    // Verify as social-service would
    let result = ss.verify_access_token(&pair.access_token).await;
    println!("verify result: {:?}", result);
    assert!(result.is_ok(), "social-service should accept gateway token");
    println!("verified principal user_id: {}", result.unwrap().user_id.0);
}

#[tokio::test]
async fn token_issuer_audience_logged() {
    let redis_url = redis_url();
    let gw_config = gw_cfg(&redis_url);
    println!("gateway issuer: {}", gw_config.issuer);
    println!("gateway audience: {}", gw_config.audience);
    let ss_config = svc_cfg(&redis_url);
    println!("social issuer: {}", ss_config.issuer);
    println!("social audience: {}", ss_config.audience);
    assert_eq!(gw_config.issuer, ss_config.issuer, "issuers must match");
    assert_eq!(
        gw_config.audience, ss_config.audience,
        "audiences must match"
    );
}
