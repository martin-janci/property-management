//! JWT service (Epic 1, Story 1.2).

use api_core::extractors::auth::{JWT_AUDIENCE, JWT_ISSUER};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, TokenData, Validation};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

/// JWT service errors.
#[derive(Debug, Error)]
pub enum JwtError {
    #[error("Token creation failed: {0}")]
    CreationFailed(String),

    #[error("Token validation failed: {0}")]
    ValidationFailed(String),

    #[error("Token expired")]
    Expired,

    #[error("Invalid token")]
    Invalid,

    #[error("Missing secret key")]
    MissingSecret,
}

/// JWT claims structure.
/// Follows JWT standard claims with custom extensions for PPT.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (user ID)
    pub sub: String,
    /// User email
    pub email: String,
    /// User display name
    pub name: String,
    /// Expiration time (Unix timestamp)
    pub exp: i64,
    /// Issued at (Unix timestamp)
    pub iat: i64,
    /// JWT ID (unique identifier for this token)
    pub jti: String,
    /// Token type (access or refresh)
    pub token_type: String,
    /// Issuer (`iss`) — the service that minted this token. Stamped at issuance
    /// and bound leniently on verification (GH #3427). `Option` so tokens minted
    /// before this claim existed still deserialize (they decode to `None`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iss: Option<String>,
    /// Audience (`aud`) — the first-party API surface this token is minted for.
    /// Stamped at issuance and bound leniently on verification (GH #3427).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aud: Option<String>,

    // Extension points for future org/role claims
    /// Organization ID (optional, for org context)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub org_id: Option<String>,
    /// User roles in the organization (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roles: Option<Vec<String>>,
    /// Phase 6 C17: principal_kind of the user (public | staff | platform).
    /// Embedded at issuance so downstream middleware can read it cheaply.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
}

/// Token pair returned after successful authentication.
#[derive(Debug, Clone, Serialize)]
pub struct TokenPair {
    /// Access token (short-lived, 15 minutes)
    pub access_token: String,
    /// Refresh token (long-lived, 7 days)
    pub refresh_token: String,
    /// Access token expiration in seconds
    pub expires_in: i64,
    /// Token type (always "Bearer")
    pub token_type: String,
}

/// JWT service for token generation and validation.
#[derive(Clone)]
pub struct JwtService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    /// Access token lifetime in seconds (15 minutes = 900)
    access_token_lifetime: i64,
    /// Refresh token lifetime in seconds (7 days = 604800)
    refresh_token_lifetime: i64,
}

impl JwtService {
    /// Create a new JwtService.
    pub fn new(secret: &str) -> Result<Self, JwtError> {
        if secret.len() < 32 {
            return Err(JwtError::MissingSecret);
        }

        Ok(Self {
            encoding_key: EncodingKey::from_secret(secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(secret.as_bytes()),
            access_token_lifetime: 15 * 60,           // 15 minutes
            refresh_token_lifetime: 7 * 24 * 60 * 60, // 7 days
        })
    }

    /// Generate an access token for a user.
    pub fn generate_access_token(
        &self,
        user_id: Uuid,
        email: &str,
        name: &str,
        org_id: Option<Uuid>,
        roles: Option<Vec<String>>,
    ) -> Result<String, JwtError> {
        self.generate_access_token_with_kind(user_id, email, name, org_id, roles, None)
    }

    /// Generate an access token with an explicit `kind` (principal_kind) claim.
    /// Phase 6 C17: used when the kind is known at issuance time so downstream
    /// middleware can read it from the token without a DB round-trip.
    pub fn generate_access_token_with_kind(
        &self,
        user_id: Uuid,
        email: &str,
        name: &str,
        org_id: Option<Uuid>,
        roles: Option<Vec<String>>,
        kind: Option<String>,
    ) -> Result<String, JwtError> {
        let now = Utc::now();
        let exp = now + Duration::seconds(self.access_token_lifetime);

        let claims = Claims {
            sub: user_id.to_string(),
            email: email.to_string(),
            name: name.to_string(),
            exp: exp.timestamp(),
            iat: now.timestamp(),
            jti: Uuid::new_v4().to_string(),
            token_type: "access".to_string(),
            iss: Some(JWT_ISSUER.to_string()),
            aud: Some(JWT_AUDIENCE.to_string()),
            org_id: org_id.map(|id| id.to_string()),
            roles,
            kind,
        };

        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| JwtError::CreationFailed(e.to_string()))
    }

    /// Generate a refresh token for a user.
    /// Returns the token string and the hashed version for storage.
    pub fn generate_refresh_token(
        &self,
        user_id: Uuid,
        email: &str,
        name: &str,
    ) -> Result<(String, String, chrono::DateTime<Utc>), JwtError> {
        let now = Utc::now();
        let exp = now + Duration::seconds(self.refresh_token_lifetime);

        let claims = Claims {
            sub: user_id.to_string(),
            email: email.to_string(),
            name: name.to_string(),
            exp: exp.timestamp(),
            iat: now.timestamp(),
            jti: Uuid::new_v4().to_string(),
            token_type: "refresh".to_string(),
            iss: Some(JWT_ISSUER.to_string()),
            aud: Some(JWT_AUDIENCE.to_string()),
            org_id: None,
            roles: None,
            kind: None,
        };

        let token = encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| JwtError::CreationFailed(e.to_string()))?;

        // Hash the token for storage
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        let token_hash = hex::encode(hasher.finalize());

        Ok((token, token_hash, exp))
    }

    /// Validate and decode an access token.
    ///
    /// # Relationship to the shared verifier (GH #1675 / #1782)
    ///
    /// This is a **deliberate third copy** of the `decode` + `token_type ==
    /// "access"` gate that `api_core::extractors::auth::verify_access_token`
    /// unified for the extractor paths. It is intentionally *not* delegated to
    /// the shared verifier because this is the **issuance-service** validator: it
    /// owns its own `decoding_key` and returns the richer [`JwtError`] (notably
    /// distinguishing `Expired` from `Invalid`/`ValidationFailed`), which ~40
    /// route call-sites depend on for their responses — a granularity the shared
    /// verifier's `&'static str` does not carry.
    ///
    /// **Maintenance contract:** any future hardening of the access-token gate
    /// (e.g. adding `aud`/`iss` validation, or tightening the `token_type`
    /// check) MUST be applied here as well as in `verify_access_token`, or this
    /// most-used path silently diverges — exactly the failure mode #1675 set out
    /// to prevent. GH #3427 did exactly that: the `iss`/`aud` binding below
    /// mirrors the one in `api_core::extractors::auth`'s cached verifier and in
    /// `principal.rs`'s `decode_bearer_subject`. The
    /// `refresh_token_rejected_by_validate_access_token` test below pins the
    /// `token_type` gate so a regression fails CI.
    pub fn validate_access_token(&self, token: &str) -> Result<Claims, JwtError> {
        let mut validation = Validation::default();
        validation.validate_exp = true;
        // Bind first-party iss/aud leniently (GH #3427): a token that carries
        // them must match, a token that omits them is still accepted (legacy
        // tokens). jsonwebtoken v10 would otherwise reject our own aud-stamped
        // tokens here, since an unexpected `aud` is rejected by default.
        validation.set_issuer(&[JWT_ISSUER]);
        validation.set_audience(&[JWT_AUDIENCE]);

        let token_data: TokenData<Claims> = decode(token, &self.decoding_key, &validation)
            .map_err(|e| match e.kind() {
                jsonwebtoken::errors::ErrorKind::ExpiredSignature => JwtError::Expired,
                _ => JwtError::ValidationFailed(e.to_string()),
            })?;

        if token_data.claims.token_type != "access" {
            return Err(JwtError::Invalid);
        }

        Ok(token_data.claims)
    }

    /// Validate and decode a refresh token.
    pub fn validate_refresh_token(&self, token: &str) -> Result<Claims, JwtError> {
        let mut validation = Validation::default();
        validation.validate_exp = true;
        // Same lenient iss/aud binding as `validate_access_token` (GH #3427).
        validation.set_issuer(&[JWT_ISSUER]);
        validation.set_audience(&[JWT_AUDIENCE]);

        let token_data: TokenData<Claims> = decode(token, &self.decoding_key, &validation)
            .map_err(|e| match e.kind() {
                jsonwebtoken::errors::ErrorKind::ExpiredSignature => JwtError::Expired,
                _ => JwtError::ValidationFailed(e.to_string()),
            })?;

        if token_data.claims.token_type != "refresh" {
            return Err(JwtError::Invalid);
        }

        Ok(token_data.claims)
    }

    /// Get access token lifetime in seconds.
    pub fn access_token_lifetime(&self) -> i64 {
        self.access_token_lifetime
    }

    /// Get refresh token lifetime in seconds.
    pub fn refresh_token_lifetime(&self) -> i64 {
        self.refresh_token_lifetime
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_service() -> JwtService {
        JwtService::new("test-secret-key-that-is-at-least-32-characters-long").unwrap()
    }

    #[test]
    fn test_access_token_generation_and_validation() {
        let service = create_test_service();
        let user_id = Uuid::new_v4();

        let token = service
            .generate_access_token(user_id, "test@example.com", "Test User", None, None)
            .unwrap();

        let claims = service.validate_access_token(&token).unwrap();

        assert_eq!(claims.sub, user_id.to_string());
        assert_eq!(claims.email, "test@example.com");
        assert_eq!(claims.name, "Test User");
        assert_eq!(claims.token_type, "access");
    }

    #[test]
    fn test_refresh_token_generation_and_validation() {
        let service = create_test_service();
        let user_id = Uuid::new_v4();

        let (token, hash, _exp) = service
            .generate_refresh_token(user_id, "test@example.com", "Test User")
            .unwrap();

        assert!(!hash.is_empty());

        let claims = service.validate_refresh_token(&token).unwrap();

        assert_eq!(claims.sub, user_id.to_string());
        assert_eq!(claims.token_type, "refresh");
    }

    #[test]
    fn test_access_token_rejected_as_refresh() {
        let service = create_test_service();
        let user_id = Uuid::new_v4();

        let token = service
            .generate_access_token(user_id, "test@example.com", "Test User", None, None)
            .unwrap();

        let result = service.validate_refresh_token(&token);
        assert!(matches!(result, Err(JwtError::Invalid)));
    }

    /// Pins the `token_type` access-gate in `JwtService::validate_access_token`
    /// — the deliberate third copy of the gate (GH #1675 / #1782). A refresh
    /// token passes signature + exp validation but must be rejected here, the
    /// same way the shared `verify_access_token` rejects it on the extractor
    /// paths. If a future edit drops or weakens this gate, this test fails CI,
    /// keeping the three access-token paths in sync.
    #[test]
    fn refresh_token_rejected_by_validate_access_token() {
        let service = create_test_service();
        let user_id = Uuid::new_v4();

        let (token, _hash, _exp) = service
            .generate_refresh_token(user_id, "test@example.com", "Test User")
            .unwrap();

        let result = service.validate_access_token(&token);
        assert!(
            matches!(result, Err(JwtError::Invalid)),
            "refresh token must be rejected as an access token, got {result:?}"
        );
    }

    #[test]
    fn test_short_secret_rejected() {
        let result = JwtService::new("short");
        assert!(matches!(result, Err(JwtError::MissingSecret)));
    }

    // --- GH #3427: first-party `iss`/`aud` binding -------------------------
    //
    // These forge tokens with an ad-hoc claims struct (independent of the
    // `Claims` type) so they compile against both the pre-fix and post-fix
    // code. The first one is the IG3 "fails-on-dev, passes-with-fix" proof.

    const TEST_SECRET: &str = "test-secret-key-that-is-at-least-32-characters-long";
    const API_ISSUER: &str = "three.two.bit.ppt.api-server";
    const API_AUDIENCE: &str = "three.two.bit.ppt.api";

    #[derive(serde::Serialize)]
    struct ForgedClaims {
        sub: String,
        email: String,
        name: String,
        exp: i64,
        iat: i64,
        jti: String,
        token_type: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        iss: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        aud: Option<String>,
    }

    fn forge(token_type: &str, iss: Option<&str>, aud: Option<&str>) -> String {
        let now = Utc::now();
        let claims = ForgedClaims {
            sub: Uuid::new_v4().to_string(),
            email: "user@example.com".to_string(),
            name: "User".to_string(),
            exp: (now + Duration::minutes(5)).timestamp(),
            iat: now.timestamp(),
            jti: Uuid::new_v4().to_string(),
            token_type: token_type.to_string(),
            iss: iss.map(String::from),
            aud: aud.map(String::from),
        };
        jsonwebtoken::encode(
            &jsonwebtoken::Header::default(),
            &claims,
            &jsonwebtoken::EncodingKey::from_secret(TEST_SECRET.as_bytes()),
        )
        .unwrap()
    }

    /// IG3 (GH #3427): a token stamped with the first-party audience + issuer
    /// MUST be accepted by `validate_access_token`. Before the fix the
    /// validator used a bare `Validation::default()`, and jsonwebtoken v10
    /// rejects any token carrying an `aud` the validator does not expect
    /// (`InvalidAudience`) — so this token was rejected. After the fix the
    /// validator binds the first-party iss/aud, so it validates. This test
    /// therefore FAILS on `dev` and PASSES with the fix.
    #[test]
    fn first_party_aud_iss_access_token_is_accepted() {
        let service = JwtService::new(TEST_SECRET).unwrap();
        let token = forge("access", Some(API_ISSUER), Some(API_AUDIENCE));
        let claims = service
            .validate_access_token(&token)
            .expect("a token carrying the first-party iss+aud must validate");
        assert_eq!(claims.token_type, "access");
    }

    /// Backward-compat (GH #3427): a token minted before the iss/aud claims
    /// existed (carrying neither) MUST still validate — the binding is lenient
    /// on *absent* claims so already-issued tokens are not broken.
    #[test]
    fn legacy_access_token_without_iss_aud_still_validates() {
        let service = JwtService::new(TEST_SECRET).unwrap();
        let token = forge("access", None, None);
        assert!(
            service.validate_access_token(&token).is_ok(),
            "a legacy token without iss/aud must remain valid"
        );
    }

    /// Security property (GH #3427): a token stamped with a FOREIGN audience is
    /// rejected — it cannot be replayed against this context.
    #[test]
    fn foreign_audience_access_token_is_rejected() {
        let service = JwtService::new(TEST_SECRET).unwrap();
        let token = forge("access", Some(API_ISSUER), Some("some-other-service"));
        assert!(
            service.validate_access_token(&token).is_err(),
            "a token minted for a foreign audience must be rejected"
        );
    }

    /// GH #3427: tokens the service mints carry the first-party iss + aud.
    #[test]
    fn minted_access_token_carries_first_party_iss_aud() {
        let service = create_test_service();
        let token = service
            .generate_access_token(Uuid::new_v4(), "user@example.com", "User", None, None)
            .unwrap();
        let claims = service.validate_access_token(&token).unwrap();
        assert_eq!(claims.iss.as_deref(), Some(API_ISSUER));
        assert_eq!(claims.aud.as_deref(), Some(API_AUDIENCE));
    }

    /// GH #3427: refresh tokens are stamped with the first-party iss + aud too.
    #[test]
    fn minted_refresh_token_carries_first_party_iss_aud() {
        let service = create_test_service();
        let (token, _hash, _exp) = service
            .generate_refresh_token(Uuid::new_v4(), "user@example.com", "User")
            .unwrap();
        let claims = service.validate_refresh_token(&token).unwrap();
        assert_eq!(claims.iss.as_deref(), Some(API_ISSUER));
        assert_eq!(claims.aud.as_deref(), Some(API_AUDIENCE));
    }
}
