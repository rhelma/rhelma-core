use rhelma_auth::{
    types::{Role, SessionId, UserPrincipal},
    AuthConfig, AuthService,
};
use rhelma_core::prelude::{TenantId, UserId};
use uuid::Uuid;

fn redis_url() -> String {
    std::env::var("RHELMA_AUTH_TEST_REDIS_URL")
        .or_else(|_| std::env::var("RHELMA_REDIS__URL"))
        .unwrap_or_else(|_| "redis://127.0.0.1:6379/0".to_string())
}

#[tokio::main]
async fn main() {
    // Use the exact same config as api-gateway
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

    let redis_url = redis_url();
    let cfg = AuthConfig::from_env(
        "api-gateway-asrnegar",
        "development",
        Some(redis_url.clone()),
    )
    .unwrap();
    println!("issuer: {}", cfg.issuer);
    println!("audience: {}", cfg.audience);

    let auth = AuthService::new(cfg.clone()).await.unwrap();

    let principal = UserPrincipal {
        user_id: UserId(Uuid::new_v4()),
        tenant_id: Some(TenantId("asrnegar".into())),
        session_id: SessionId::new(),
        roles: vec![Role("user".into())],
        permissions: vec![],
    };
    let pair = auth.issue_for_principal(&principal).await.unwrap();
    println!("issued token (first 50): {}", &pair.access_token[..50]);

    // Now verify with SOCIAL-SERVICE config
    let scfg = AuthConfig::from_env("social-asrnegar", "development", Some(redis_url)).unwrap();
    println!("social issuer: {}", scfg.issuer);
    println!("social audience: {}", scfg.audience);
    let social_auth = AuthService::new(scfg).await.unwrap();

    match social_auth.verify_access_token(&pair.access_token).await {
        Ok(p) => println!("VERIFY OK: user_id={}", p.user_id.0),
        Err(e) => println!("VERIFY FAILED: {:?}", e),
    }
}
