//! Authentication extractor.
//!
//! # Phase 2 deprecation note
//!
//! `AuthUser` is **deprecated** in favor of `crate::extractors::principal::RequestPrincipal`.
//! The `tenant_id` and `role` fields on `AuthUser` come from JWT claims and
//! must NOT be trusted as authority — they are kept only so existing callers
//! still compile during the one-phase Phase 2 transition window. The Phase 2
//! identity model re-derives both authority and the principal kind from
//! trusted server-side state on every request (see leak #10/#11 in the
//! brainstorming session). New extractors should be `RequestPrincipal`-based.

use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
};
use common::TenantRole;
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use uuid::Uuid;

/// JWT claims as seen by this extractor. Matches the on-the-wire shape
/// produced by `api_server::services::jwt::JwtService::Claims`:
/// `sub` is encoded as a JSON string (user_id.to_string()), which the
/// `Uuid` serde implementation will parse back into a UUID on decode.
///
/// `token_type` is **required**: an access-token-bearing endpoint must
/// reject any token whose `token_type != "access"`. Previously this
/// field was absent and the extractor happily decoded refresh tokens
/// as access tokens. (RUST-002)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (user ID)
    pub sub: Uuid,
    /// Expiration time
    pub exp: i64,
    /// Issued at
    pub iat: i64,
    /// Discriminator between access and refresh tokens. The JwtService
    /// always emits this; older fixtures that omit it will fail to
    /// deserialize, which is the safe direction.
    pub token_type: String,
    /// Current tenant ID
    pub tenant_id: Option<Uuid>,
    /// Role in current tenant
    pub role: Option<TenantRole>,
    /// Email
    pub email: String,
    /// Display name
    pub name: String,
}

/// Cached `DecodingKey` and `Validation`. Built once on first request
/// (via `OnceLock`) from the `JWT_SECRET` env var instead of re-reading
/// + re-allocating on every request. (RUST-002 hot-path concern.)
struct JwtVerifier {
    key: DecodingKey,
    validation: Validation,
}

static JWT_VERIFIER: OnceLock<Result<JwtVerifier, String>> = OnceLock::new();

fn jwt_verifier() -> Result<&'static JwtVerifier, &'static str> {
    let cell = JWT_VERIFIER.get_or_init(|| {
        let secret = std::env::var("JWT_SECRET")
            .map_err(|_| "JWT_SECRET environment variable not set".to_string())?;
        if secret.len() < 32 {
            return Err("JWT_SECRET must be at least 32 characters".to_string());
        }
        let mut validation = Validation::default();
        // Validation::default() already requires `exp` and checks signature.
        // We accept the JwtService's default HS256 alg.
        validation.leeway = 30; // 30s clock skew tolerance
                                // iss/aud are not currently emitted by JwtService — when they
                                // are added, set_audience / set_issuer here to bind them.
        Ok(JwtVerifier {
            key: DecodingKey::from_secret(secret.as_bytes()),
            validation,
        })
    });
    match cell {
        Ok(v) => Ok(v),
        Err(_) => Err("Server configuration error"),
    }
}

/// Trait implemented by every claims shape that carries the access-token
/// discriminator, so [`verify_access_token`] can apply the shared
/// `token_type` gate without owning a single concrete claims struct.
///
/// The two live callers deliberately differ in their `token_type` *shape*:
///
///   * `auth.rs`'s [`Claims`] makes it a **required** `String` — a token that
///     omits the claim fails to deserialize (the safe direction), so the gate
///     never even sees a missing value.
///   * `principal.rs`'s `PrincipalClaims` makes it `Option<String>` and is
///     **legacy-tolerant** — a token minted before the discriminator existed
///     omits the claim and is treated as access.
///
/// That difference is captured by the combination of this trait's return type
/// and the [`TokenTypePolicy`] passed to [`verify_access_token`], so the
/// security-critical decode + gate lives in exactly one place (GH #1675).
pub trait AccessTokenClaims {
    /// The `token_type` claim, if present. `None` means the claim was absent
    /// (only reachable for `Option`-shaped claims; a required `String` claim
    /// fails to deserialize before reaching here).
    fn token_type(&self) -> Option<&str>;
}

impl AccessTokenClaims for Claims {
    fn token_type(&self) -> Option<&str> {
        Some(self.token_type.as_str())
    }
}

/// How strictly the shared verifier treats the `token_type` discriminator.
///
/// This is the single explicit policy flag the two historical copies of the
/// validation path differed on — folded here so the decode + gate has ONE
/// implementation (GH #1675). Both policies reject an *explicit* non-`access`
/// value identically; they differ only on an absent claim:
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenTypePolicy {
    /// A missing `token_type` is rejected. (`auth.rs` semantics — though in
    /// practice [`Claims`] makes the field a required `String`, so a missing
    /// claim already fails deserialization upstream of the gate.)
    Required,
    /// A missing `token_type` is treated as `access`. (`principal.rs`
    /// semantics — backward-compat for legacy tokens minted before the
    /// discriminator existed.)
    LegacyTolerant,
}

/// Shared access-token verifier (GH #1675).
///
/// Owns the security-critical sequence that used to be copy-pasted across
/// `auth.rs` (the `validate_access_token*` family + the `AuthUser` extractor)
/// and `principal.rs` (`decode_bearer_subject`): `jsonwebtoken::decode` with
/// the caller-supplied [`DecodingKey`] + [`Validation`] (so `exp`/`nbf` and the
/// signature are enforced), followed by the `token_type` access-gate.
///
/// The caller supplies its own key + `Validation` so each path keeps its exact
/// historical behavior — notably `auth.rs` uses the cached `JWT_VERIFIER`
/// (`leeway = 30`) while `principal.rs` uses a fresh `Validation::default()`
/// (`leeway = 0`); unifying those would *loosen* or *tighten* one path, which
/// this refactor must not do. The strictness difference on a *missing*
/// `token_type` is captured by [`TokenTypePolicy`].
///
/// Returns the decoded claims on success so each caller can read the
/// additional fields it needs (`email`/`name`/`role` for `auth.rs`, `kind` for
/// `principal.rs`) from the same single decode.
pub fn verify_access_token<C>(
    token: &str,
    key: &DecodingKey,
    validation: &Validation,
    policy: TokenTypePolicy,
) -> Result<C, &'static str>
where
    C: AccessTokenClaims + serde::de::DeserializeOwned,
{
    let token_data = decode::<C>(token, key, validation).map_err(|_| "Invalid or expired token")?;
    let claims = token_data.claims;
    match claims.token_type() {
        Some("access") => Ok(claims),
        // An explicit non-access discriminator (e.g. a refresh token) is
        // rejected by BOTH policies — this is the RUST-002 / #822 gate.
        Some(_) => Err("Invalid token type for this endpoint"),
        // A missing discriminator: rejected only under `Required`.
        None => match policy {
            TokenTypePolicy::Required => Err("Invalid token type for this endpoint"),
            TokenTypePolicy::LegacyTolerant => Ok(claims),
        },
    }
}

/// Validate a raw JWT access token and return the subject (user ID).
///
/// Reuses the shared `JWT_VERIFIER` so the HMAC key is loaded and cached once,
/// shared across all callers (HTTP extractors, WebSocket upgrade, etc.).
/// Any future algorithm rotation or `iss`/`aud` hardening in `jwt_verifier()`
/// automatically applies here.
///
/// Returns an error string if the token is invalid/expired, the JWT_SECRET is
/// misconfigured, or the `token_type` claim is not `"access"`.
pub fn validate_access_token(token: &str) -> Result<Uuid, &'static str> {
    validate_access_token_with_exp(token).map(|(uid, _)| uid)
}

/// Like [`validate_access_token`] but also returns the `exp` Unix timestamp.
///
/// Issue #480: long-lived WebSocket sessions must re-check the JWT exp so
/// a logged-out user cannot retain a live channel for hours after the
/// access token has expired.
pub fn validate_access_token_with_exp(token: &str) -> Result<(Uuid, i64), &'static str> {
    let verifier = jwt_verifier()?;
    let claims: Claims = verify_access_token(
        token,
        &verifier.key,
        &verifier.validation,
        TokenTypePolicy::Required,
    )?;
    Ok((claims.sub, claims.exp))
}

/// Authenticated user extractor.
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: Uuid,
    pub email: String,
    pub name: String,
    pub tenant_id: Option<Uuid>,
    pub role: Option<TenantRole>,
}

impl AuthUser {
    /// Check if user is a platform administrator.
    pub fn is_platform_admin(&self) -> bool {
        matches!(
            self.role,
            Some(TenantRole::SuperAdmin) | Some(TenantRole::PlatformAdmin)
        )
    }
}

impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Get Authorization header
        let auth_header = parts
            .headers
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or((StatusCode::UNAUTHORIZED, "Missing Authorization header"))?;

        // Extract Bearer token
        let token = auth_header.strip_prefix("Bearer ").ok_or((
            StatusCode::UNAUTHORIZED,
            "Invalid Authorization header format",
        ))?;

        // Use cached verifier; reads JWT_SECRET only on the first request.
        let verifier = jwt_verifier().map_err(|msg| {
            tracing::error!("{}", msg);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Server configuration error",
            )
        })?;

        // Decode + enforce the token-type discriminator via the shared
        // verifier (GH #1675). P0-03: refresh tokens issued by JwtService also
        // pass signature+exp validation, but must not be accepted as access
        // tokens. `Required` policy mirrors the historical `auth.rs` behavior
        // exactly (the `Claims` shape already makes `token_type` mandatory).
        // The shared verifier returns `&'static str`; map it to this
        // extractor's `(StatusCode, &'static str)` rejection unchanged.
        let claims: Claims = verify_access_token(
            token,
            &verifier.key,
            &verifier.validation,
            TokenTypePolicy::Required,
        )
        .map_err(|msg| (StatusCode::UNAUTHORIZED, msg))?;

        // Store user_id and role in extensions for TenantExtractor
        parts.extensions.insert(claims.sub);
        if let Some(role) = claims.role {
            parts.extensions.insert(role);
        }

        Ok(AuthUser {
            user_id: claims.sub,
            email: claims.email,
            name: claims.name,
            tenant_id: claims.tenant_id,
            role: claims.role,
        })
    }
}

/// Optional authentication (for public endpoints that benefit from auth).
#[derive(Debug, Clone)]
pub struct OptionalAuth(pub Option<AuthUser>);

impl<S> FromRequestParts<S> for OptionalAuth
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        match AuthUser::from_request_parts(parts, state).await {
            Ok(user) => Ok(OptionalAuth(Some(user))),
            Err(_) => Ok(OptionalAuth(None)),
        }
    }
}
