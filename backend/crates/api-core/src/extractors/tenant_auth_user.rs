//! `AuthUser` with its organization and role resolved from the database.

use crate::extractors::{AuthUser, TenantMembershipProvider, ValidatedTenantExtractor};
use crate::middleware::host_tenant::ResolvedTenant;
use axum::{
    extract::FromRequestParts,
    http::{request::Parts, HeaderValue, StatusCode},
};
use common::TenantRole;

const TENANT_HEADER: &str = "X-Tenant-ID";

/// An [`AuthUser`] whose `tenant_id` and `role` come from the caller's
/// validated organization membership instead of the JWT.
///
/// Login access tokens carry no tenant or role claim (the organization is
/// chosen per request via the Host / `X-Tenant-ID` header), so handlers that
/// read `AuthUser.tenant_id` / `AuthUser.role` directly saw `None` for every
/// real user and answered 403 ("No organization context", "Only managers ...").
///
/// This extractor runs [`ValidatedTenantExtractor`] (tenant from Host /
/// `X-Tenant-ID`, active membership required, role from
/// `organization_members`) and overwrites both fields with the result, so
/// existing `AuthUser`-based helpers keep working unchanged:
///
/// - `tenant_id` is always `Some(<validated org>)`;
/// - `role` is always `Some(<membership role>)` (platform admins keep their
///   JWT role, as in `ValidatedTenantExtractor`); a membership never grants
///   the platform tier, see [`cap_platform_tier`].
///
/// The tenant comes from the Host-resolved org or the `X-Tenant-ID` header,
/// falling back to a signed JWT `tenant_id` claim when neither is present
/// (service / test tokens). With no tenant at all, platform admins pass
/// through unchanged (platform-wide endpoints) and everyone else gets 403
/// "No organization context".
///
/// Other rejections are those of `ValidatedTenantExtractor`: 401 without a
/// valid token, 403 when the caller is not an active member of the org.
#[derive(Debug, Clone)]
pub struct TenantAuthUser(pub AuthUser);

impl std::ops::Deref for TenantAuthUser {
    type Target = AuthUser;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<S> FromRequestParts<S> for TenantAuthUser
where
    S: TenantMembershipProvider,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let mut user = AuthUser::from_request_parts(parts, state).await?;

        let has_request_tenant = parts.extensions.get::<ResolvedTenant>().is_some()
            || parts.headers.contains_key(TENANT_HEADER);
        if !has_request_tenant {
            match user.tenant_id {
                // Feed the signed claim through the same membership check.
                Some(jwt_tenant) => {
                    let value = HeaderValue::from_str(&jwt_tenant.to_string())
                        .expect("a UUID is a valid header value");
                    parts.headers.insert(TENANT_HEADER, value);
                }
                None if user.is_platform_admin() => return Ok(TenantAuthUser(user)),
                None => return Err((StatusCode::FORBIDDEN, "No organization context")),
            }
        }

        let ValidatedTenantExtractor(ctx) =
            ValidatedTenantExtractor::from_request_parts(parts, state).await?;
        user.tenant_id = Some(ctx.tenant_id);
        user.role = Some(cap_platform_tier(ctx.role, user.is_platform_admin()));
        Ok(TenantAuthUser(user))
    }
}

/// Platform tier (`SuperAdmin` / `PlatformAdmin`) is never granted by an org
/// membership row: org admins can write any `role_type` on members of their own
/// org, so trusting it here would let them unlock platform-wide endpoints
/// (compliance audit logs, migration, AML/EDD). A membership-derived platform
/// role is capped at `OrgAdmin`; platform tier still comes only from a platform
/// admin's token, exactly as before.
fn cap_platform_tier(role: TenantRole, caller_is_platform_admin: bool) -> TenantRole {
    match role {
        TenantRole::SuperAdmin | TenantRole::PlatformAdmin if !caller_is_platform_admin => {
            TenantRole::OrgAdmin
        }
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::cap_platform_tier;
    use common::TenantRole;

    #[test]
    fn membership_never_grants_platform_tier() {
        assert_eq!(
            cap_platform_tier(TenantRole::SuperAdmin, false),
            TenantRole::OrgAdmin
        );
        assert_eq!(
            cap_platform_tier(TenantRole::PlatformAdmin, false),
            TenantRole::OrgAdmin
        );
        assert_eq!(
            cap_platform_tier(TenantRole::SuperAdmin, true),
            TenantRole::SuperAdmin
        );
        assert_eq!(
            cap_platform_tier(TenantRole::Manager, false),
            TenantRole::Manager
        );
    }
}
