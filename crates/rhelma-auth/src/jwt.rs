//! JWT service (Ed25519 / EdDSA) for Rhelma Auth.

use base64::Engine as _;
use chrono::Utc;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation};
use secrecy::ExposeSecret;

use crate::config::AuthConfig;
use crate::crypto::keys::load_ed25519_keys;
use crate::error::{AuthError, AuthResult};
use crate::metrics;
use crate::tracing_ext::auth_span;
use crate::types::{AuthSubject, JwtClaims, SessionId, UserPrincipal};

/// Pair of access + refresh tokens (refresh stored/validated via Redis).
#[derive(Debug, Clone)]
/// struct (documented for contract compliance).
pub struct JwtTokenPair {
    /// Access token (JWT).
    pub access_token: String,
    /// Refresh token (opaque, random).
    pub refresh_token: String,
    /// Access token expiry timestamp (unix seconds).
    pub access_exp: i64,
}

#[derive(Clone)]
/// struct (documented for contract compliance).
pub struct JwtService {
    encoding: EncodingKey,
    decoding: DecodingKey,
    issuer: String,
    audience: String,
    access_ttl_secs: u64,
}

impl JwtService {
    /// Build JWT service from config (loads Ed25519 keys).
    pub fn new(cfg: &AuthConfig) -> AuthResult<Self> {
        cfg.validate()?;

        let (enc, dec) = load_ed25519_keys(
            cfg.jwt_private_key_b64.expose_secret(),
            cfg.jwt_public_key_b64.expose_secret(),
        )?;

        Ok(Self {
            encoding: enc,
            decoding: dec,
            issuer: cfg.issuer.clone(),
            audience: cfg.audience.clone(),
            access_ttl_secs: cfg.access_token_ttl_secs,
        })
    }

    /// Encode an access token for a principal.
    ///
    /// Returns (jwt, jti, exp_unix).
    pub fn encode_access(&self, principal: &UserPrincipal) -> AuthResult<(String, String, i64)> {
        let _span = auth_span("jwt.encode_access");
        let now = Utc::now();
        let exp = now + chrono::Duration::seconds(self.access_ttl_secs as i64);

        // Use UUIDv7 when available for time-sortable IDs.
        let jti = uuid::Uuid::now_v7().to_string();

        let claims = JwtClaims {
            sub: principal.user_id,
            tenant_id: principal.tenant_id.clone(),
            session_id: principal.session_id,
            iat: now.timestamp(),
            exp: exp.timestamp(),
            jti: jti.clone(),
            subject: AuthSubject::User,
            iss: self.issuer.clone(),
            aud: self.audience.clone(),
            roles: principal.roles.clone(),
            permissions: principal.permissions.clone(),
        };

        let mut header = Header::new(Algorithm::EdDSA);
        header.typ = Some("JWT".to_string());

        let token = jsonwebtoken::encode(&header, &claims, &self.encoding)?;
        Ok((token, jti, exp.timestamp()))
    }

    /// Verify JWT and return claims.
    pub fn verify(&self, token: &str) -> AuthResult<JwtClaims> {
        let _span = auth_span("jwt.verify");

        let mut v = Validation::new(Algorithm::EdDSA);
        v.set_audience(std::slice::from_ref(&self.audience));
        v.set_issuer(std::slice::from_ref(&self.issuer));
        v.validate_exp = true;

        let data = jsonwebtoken::decode::<JwtClaims>(token, &self.decoding, &v)?;
        metrics::record_token_verify("ok");
        Ok(data.claims)
    }

    /// Convert verified claims to a principal.
    pub fn claims_to_principal(&self, claims: JwtClaims) -> AuthResult<UserPrincipal> {
        if claims.subject != AuthSubject::User {
            return Err(AuthError::Unauthorized);
        }

        Ok(UserPrincipal {
            user_id: claims.sub,
            tenant_id: claims.tenant_id,
            session_id: claims.session_id,
            roles: claims.roles,
            permissions: claims.permissions,
        })
    }

    /// Extract session id without verifying signature (ONLY for logging/metrics).
    pub fn unsafe_session_id(token: &str) -> Option<SessionId> {
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() < 2 {
            return None;
        }
        let payload_b64 = parts[1];
        let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(payload_b64.as_bytes())
            .ok()?;
        let v: serde_json::Value = serde_json::from_slice(&payload).ok()?;
        let sid = v.get("session_id")?.as_str()?;
        SessionId::parse(sid).ok()
    }
}
