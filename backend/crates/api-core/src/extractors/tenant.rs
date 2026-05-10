//! Tenant context extractor.

use crate::AuthUser;
use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
};
use common::{TenantContext, TenantRole};
use db::repositories::OrganizationMemberRepository;
use db::DbPool;
use uuid::Uuid;

/// Extractor for tenant context from X-Tenant-ID header.
///
/// SECURITY: This extractor automatically validates JWT authentication by extracting
/// AuthUser first. Routes using TenantExtractor do NOT need to also extract AuthUser.
/// The user_id and role are populated from the JWT claims.
///
/// NOTE: This extractor does NOT validate database membership. For production routes
/// requiring strict tenant isolation, use `ValidatedTenantExtractor` instead.
#[derive(Debug, Clone)]
pub struct TenantExtractor(pub TenantContext);

impl std::ops::Deref for TenantExtractor {
    type Target = TenantContext;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<S> FromRequestParts<S> for TenantExtractor
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // SECURITY: Always extract and validate JWT authentication first
        // This ensures user_id and role are populated in extensions
        let auth_user = AuthUser::from_request_parts(parts, state).await?;

        // Get X-Tenant-ID header
        let tenant_id = parts
            .headers
            .get("X-Tenant-ID")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| Uuid::parse_str(s).ok())
            .ok_or((
                StatusCode::BAD_REQUEST,
                "Missing or invalid X-Tenant-ID header",
            ))?;

        // User ID comes from authenticated JWT (validated above)
        let user_id = auth_user.user_id;

        // Role comes from JWT claims, fall back to Guest (most restrictive)
        let role = auth_user.role.unwrap_or(TenantRole::Guest);

        Ok(TenantExtractor(TenantContext::new(
            tenant_id, user_id, role,
        )))
    }
}

/// Trait for providing tenant membership validation capability.
///
/// Implement this trait on your application state to enable `ValidatedTenantExtractor`,
/// which validates that users are active members of the tenant they're trying to access.
///
/// # Security Implications
///
/// - The `ValidatedTenantExtractor` uses this trait to query the database and verify
///   that the authenticated user has an active membership in the requested tenant.
/// - This prevents cross-tenant access attacks where a user might try to access
///   another tenant's data by manipulating the tenant ID header.
/// - The validated role from the database is used for authorization decisions,
///   not the role from the JWT, providing defense in depth.
///
/// # Example Implementation
///
/// ```rust,ignore
/// use api_core::extractors::TenantMembershipProvider;
/// use db::DbPool;
///
/// #[derive(Clone)]
/// pub struct AppState {
///     pub db_pool: DbPool,
///     // ... other fields
/// }
///
/// impl TenantMembershipProvider for AppState {
///     fn db_pool(&self) -> &DbPool {
///         &self.db_pool
///     }
/// }
/// ```
pub trait TenantMembershipProvider: Clone + Send + Sync + 'static {
    /// Get the database pool for membership queries.
    fn db_pool(&self) -> &DbPool;
}

/// Validated tenant extractor that verifies database membership.
///
/// SECURITY: This extractor validates that the user is an active member of the
/// requested tenant in the database, preventing cross-tenant access attacks.
///
/// Use this extractor for routes that access sensitive tenant data.
/// For public or less sensitive routes, `TenantExtractor` may be sufficient.
#[derive(Debug, Clone)]
pub struct ValidatedTenantExtractor(pub TenantContext);

impl std::ops::Deref for ValidatedTenantExtractor {
    type Target = TenantContext;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<S> FromRequestParts<S> for ValidatedTenantExtractor
where
    S: TenantMembershipProvider,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // SECURITY: Always extract and validate JWT authentication first
        let auth_user = AuthUser::from_request_parts(parts, state).await?;

        // Get X-Tenant-ID header
        let tenant_id = parts
            .headers
            .get("X-Tenant-ID")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| Uuid::parse_str(s).ok())
            .ok_or((
                StatusCode::BAD_REQUEST,
                "Missing or invalid X-Tenant-ID header",
            ))?;

        let user_id = auth_user.user_id;

        // SECURITY: Platform admins bypass tenant membership check
        // They can access any tenant for administrative purposes
        if auth_user.is_platform_admin() {
            let role = auth_user.role.unwrap_or(TenantRole::SuperAdmin);
            // Insert role into extensions for authorization middleware
            parts.extensions.insert(role);
            return Ok(ValidatedTenantExtractor(TenantContext::new(
                tenant_id, user_id, role,
            )));
        }

        // SECURITY: Validate user membership in the requested tenant
        let member_repo = OrganizationMemberRepository::new(state.db_pool().clone());

        // Check if user is an active member of this tenant
        let is_member = member_repo
            .is_member(tenant_id, user_id)
            .await
            .map_err(|e| {
                tracing::error!(
                    user_id = %user_id,
                    tenant_id = %tenant_id,
                    error = %e,
                    "Failed to verify tenant membership"
                );
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to verify tenant access",
                )
            })?;

        if !is_member {
            tracing::warn!(
                user_id = %user_id,
                tenant_id = %tenant_id,
                "Unauthorized tenant access attempt - user not a member"
            );
            return Err((StatusCode::FORBIDDEN, "User is not a member of this tenant"));
        }

        // Get actual role from database (don't trust JWT for authorization)
        let role_type = member_repo
            .get_user_role_type(tenant_id, user_id)
            .await
            .map_err(|e| {
                tracing::error!(
                    user_id = %user_id,
                    tenant_id = %tenant_id,
                    error = %e,
                    "Failed to get user role in tenant"
                );
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to verify user role",
                )
            })?;

        // Parse role from database, fall back to Guest if unknown.
        // Uses case-insensitive matching to handle variations in database storage.
        let role = role_type
            .as_deref()
            .and_then(|r| {
                let r_lower = r.to_lowercase();
                match r_lower.as_str() {
                    "super_admin" | "superadmin" => Some(TenantRole::SuperAdmin),
                    "platform_admin" | "platformadmin" => Some(TenantRole::PlatformAdmin),
                    "org_admin" | "orgadmin" => Some(TenantRole::OrgAdmin),
                    "manager" => Some(TenantRole::Manager),
                    "technical_manager" | "technicalmanager" => Some(TenantRole::TechnicalManager),
                    "owner" => Some(TenantRole::Owner),
                    "owner_delegate" | "ownerdelegate" => Some(TenantRole::OwnerDelegate),
                    "tenant" => Some(TenantRole::Tenant),
                    "resident" => Some(TenantRole::Resident),
                    "property_manager" | "propertymanager" => Some(TenantRole::PropertyManager),
                    "real_estate_agent" | "realestateagent" => Some(TenantRole::RealEstateAgent),
                    "guest" => Some(TenantRole::Guest),
                    other => {
                        tracing::warn!(
                            user_id = %user_id,
                            tenant_id = %tenant_id,
                            role = %other,
                            "Unknown role type in database, defaulting to Guest"
                        );
                        None
                    }
                }
            })
            .unwrap_or(TenantRole::Guest);

        tracing::debug!(
            user_id = %user_id,
            tenant_id = %tenant_id,
            role = ?role,
            "Validated tenant membership"
        );

        // SECURITY: Insert database-validated role into extensions for authorization middleware.
        // This overwrites any role from the JWT, ensuring authorization uses the current
        // database role rather than potentially stale JWT claims.
        parts.extensions.insert(role);

        Ok(ValidatedTenantExtractor(TenantContext::new(
            tenant_id, user_id, role,
        )))
    }
}

/// Optional tenant extractor (for endpoints that work with or without tenant context).
///
/// Returns None if authentication fails or tenant header is missing.
/// Useful for public endpoints that benefit from tenant context when available.
#[derive(Debug, Clone)]
pub struct OptionalTenant(pub Option<TenantContext>);

impl<S> FromRequestParts<S> for OptionalTenant
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        match TenantExtractor::from_request_parts(parts, state).await {
            Ok(TenantExtractor(ctx)) => Ok(OptionalTenant(Some(ctx))),
            Err(_) => Ok(OptionalTenant(None)),
        }
    }
}
