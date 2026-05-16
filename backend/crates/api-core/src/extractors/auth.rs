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
use uuid::Uuid;

/// JWT claims.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (user ID)
    pub sub: Uuid,
    /// Expiration time
    pub exp: i64,
    /// Issued at
    pub iat: i64,
    /// Current tenant ID
    pub tenant_id: Option<Uuid>,
    /// Role in current tenant
    pub role: Option<TenantRole>,
    /// Email
    pub email: String,
    /// Display name
    pub name: String,
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

        // Get JWT secret from environment - REQUIRED, no fallback for security
        let secret = std::env::var("JWT_SECRET").map_err(|_| {
            tracing::error!("JWT_SECRET environment variable not set");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Server configuration error",
            )
        })?;

        // Decode and validate JWT
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(secret.as_bytes()),
            &Validation::default(),
        )
        .map_err(|_| (StatusCode::UNAUTHORIZED, "Invalid or expired token"))?;

        let claims = token_data.claims;

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
