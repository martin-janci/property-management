//! Agency provisioning routes (Phase 1: Tenant Resolution).
//!
//! A minimal, platform-admin-gated "create agency" endpoint. Provisioning an
//! agency means creating the **organization** (the RLS tenant) and its first
//! `agency_domains` row in one transaction, then invalidating the
//! host-resolution cache so the new host resolves immediately.
//!
//! This router is merged into [`super::platform_admin::router`] so it inherits
//! the `/api/v1/platform-admin` nest prefix from `lib.rs` and the existing
//! platform-admin auth gate (`extract_super_admin_token`).

use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use common::errors::ErrorResponse;
use db::models::agency_domain::{AgencyDomain, AgencyDomainKind, CreateAgencyDomain};
use db::models::organization::{CreateOrganization, Organization};
use db::repositories::{AgencyDomainRepository, OrganizationRepository};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::routes::platform_admin::extract_super_admin_token;
use crate::state::AppState;

/// Default platform apex used to derive `{slug}.{apex}` when no explicit host
/// is supplied. Overridable via the `PLATFORM_APEX` env var.
const DEFAULT_PLATFORM_APEX: &str = "dev.rlt.sk";

/// Build the agency-provisioning sub-router.
///
/// Mounted with no extra prefix: paths here are relative to the
/// `/api/v1/platform-admin` nest that `platform_admin::router()` lives under.
pub fn router() -> Router<AppState> {
    Router::new().route("/agencies", post(create_agency))
}

// ==================== Types ====================

/// Request body for provisioning a new agency.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateAgencyRequest {
    /// Display name of the agency / organization.
    pub name: String,
    /// URL-safe slug. Must be unique across (non-deleted) organizations.
    pub slug: String,
    /// Primary contact email for the organization.
    pub contact_email: String,
    /// Optional inbound host. When omitted, defaults to
    /// `{slug}.{PLATFORM_APEX}`.
    #[serde(default)]
    pub host: Option<String>,
}

/// Response body for a successful agency provisioning.
#[derive(Debug, Serialize, ToSchema)]
pub struct CreateAgencyResponse {
    /// The newly created organization (tenant).
    pub organization: Organization,
    /// The primary `agency_domains` mapping created for it.
    pub domain: AgencyDomain,
}

// ==================== Validation ====================

/// Slug rule: matches the dev-mode `/a/{slug}` charset
/// (`[a-z0-9-]+`) so a provisioned agency is always reachable in dev mode.
fn is_valid_slug(slug: &str) -> bool {
    !slug.is_empty()
        && slug.len() <= 63
        && slug
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Very small sanity check for an email address — full validation is the
/// caller's responsibility; this only rejects obviously malformed input.
fn is_plausible_email(email: &str) -> bool {
    let email = email.trim();
    !email.is_empty()
        && email.len() <= 320
        && email.contains('@')
        && !email.starts_with('@')
        && !email.ends_with('@')
}

/// Host rule: lowercase DNS-ish label set. Keeps the stored host consistent
/// with what `normalize_host` in the resolution middleware would produce.
fn is_valid_host(host: &str) -> bool {
    !host.is_empty()
        && host.len() <= 253
        && host
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '.')
        && !host.starts_with('.')
        && !host.ends_with('.')
}

fn bad_request(code: &str, msg: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse::new(code, msg)),
    )
}

fn db_error(msg: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse::new("DATABASE_ERROR", msg)),
    )
}

// ==================== Handler ====================

/// Provision a new agency: create the organization (tenant) and its primary
/// `agency_domains` mapping, then invalidate the host-resolution cache.
#[utoipa::path(
    post,
    path = "/api/v1/platform-admin/agencies",
    tag = "Platform Admin",
    security(("bearer_auth" = [])),
    request_body = CreateAgencyRequest,
    responses(
        (status = 201, description = "Agency provisioned", body = CreateAgencyResponse),
        (status = 400, description = "Invalid input", body = ErrorResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 409, description = "Slug already in use", body = ErrorResponse)
    )
)]
pub async fn create_agency(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<CreateAgencyRequest>,
) -> Result<(StatusCode, Json<CreateAgencyResponse>), (StatusCode, Json<ErrorResponse>)> {
    // Platform-admin gate — identical pattern to every other platform_admin route.
    let (admin_id, admin_email) = extract_super_admin_token(&headers, &state)?;

    // ---- 1. Validate input ------------------------------------------------
    let name = req.name.trim().to_string();
    if name.is_empty() || name.len() > 255 {
        return Err(bad_request(
            "INVALID_NAME",
            "Agency name must be 1-255 characters",
        ));
    }

    let slug = req.slug.trim().to_lowercase();
    if !is_valid_slug(&slug) {
        return Err(bad_request(
            "INVALID_SLUG",
            "Slug must be 1-63 chars of lowercase letters, digits, or hyphens",
        ));
    }

    let contact_email = req.contact_email.trim().to_string();
    if !is_plausible_email(&contact_email) {
        return Err(bad_request(
            "INVALID_CONTACT_EMAIL",
            "contact_email must be a plausible email address",
        ));
    }

    // Host: explicit, or derived from `{slug}.{PLATFORM_APEX}`.
    let host = match req.host {
        Some(h) => h.trim().to_lowercase(),
        None => {
            let apex = std::env::var("PLATFORM_APEX")
                .unwrap_or_else(|_| DEFAULT_PLATFORM_APEX.to_string());
            format!("{slug}.{apex}")
        }
    };
    if !is_valid_host(&host) {
        return Err(bad_request(
            "INVALID_HOST",
            "host must be a valid lowercase DNS host name",
        ));
    }

    // Repositories are thin pool wrappers; their `*_rls` methods take the
    // executor explicitly, so we build them from `state.db` here rather than
    // depending on an `AppState` field (`AppState` is owned by sibling agent
    // 3X and not modified in this lane).
    let org_repo = OrganizationRepository::new(state.db.clone());
    // N6: wire the host-resolution cache so `create_rls` (and other write
    // methods) drop any cached entry for the touched host eagerly. The
    // explicit `state.tenant_resolution_cache.invalidate(&host)` call below
    // remains as a belt-and-braces — it covers the post-commit `verified`
    // promotion which still hits agency_domains via raw SQL.
    let agency_domain_repo = AgencyDomainRepository::new(state.db.clone())
        .with_cache(state.tenant_resolution_cache.clone());

    // ---- 2. Open a transaction on a super-admin RLS connection -----------
    // platform_admin handlers operate without an `RlsConnection` extractor, so
    // we acquire the connection here and set a super-admin context on the
    // transaction itself. The context is bound to this connection only and is
    // dropped with the transaction.
    let mut tx = state.db.begin().await.map_err(|e| {
        tracing::error!(error = %e, "Failed to begin agency-provisioning transaction");
        db_error("Failed to begin transaction")
    })?;

    db::tenant_context::set_request_context(&mut *tx, None, Some(admin_id), true)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to set super-admin RLS context");
            db_error("Failed to set security context")
        })?;

    // Pre-check slug uniqueness inside the transaction.
    let exists = org_repo
        .slug_exists_rls(&mut *tx, &slug)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "slug_exists_rls failed");
            db_error("Failed to check slug availability")
        })?;
    if exists {
        return Err((
            StatusCode::CONFLICT,
            Json(ErrorResponse::new(
                "SLUG_TAKEN",
                "An organization with this slug already exists",
            )),
        ));
    }

    // ---- 3. Create the organization (the tenant) -------------------------
    let organization = org_repo
        .create_rls(
            &mut *tx,
            CreateOrganization {
                name: name.clone(),
                slug: slug.clone(),
                contact_email: contact_email.clone(),
                logo_url: None,
                primary_color: None,
            },
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "create_rls(organization) failed");
            db_error("Failed to create organization")
        })?;

    // ---- 4. Create the primary agency_domains row ------------------------
    // `AgencyDomainRepository::create_rls` inserts kind + is_primary but leaves
    // `verification_state` at its column default (`pending`). The provisioning
    // contract requires the seeded domain to be `verified` so the host resolves
    // immediately, so we promote it within the same transaction.
    let domain = agency_domain_repo
        .create_rls(
            &mut *tx,
            CreateAgencyDomain {
                organization_id: organization.id,
                host: host.clone(),
                kind: Some(AgencyDomainKind::Subdomain),
                is_primary: true,
            },
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "create_rls(agency_domain) failed");
            db_error("Failed to create agency domain")
        })?;

    let domain = sqlx::query_as::<_, AgencyDomain>(
        r#"
        UPDATE agency_domains
        SET verification_state = 'verified',
            verified_at = NOW(),
            updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(domain.id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "Failed to mark agency domain verified");
        db_error("Failed to verify agency domain")
    })?;

    // Clear the RLS context before the connection returns to the pool, then
    // commit. (Committing first would also release the connection, but clearing
    // explicitly keeps the same discipline as the RlsConnection extractors.)
    if let Err(e) = db::tenant_context::clear_request_context(&mut *tx).await {
        tracing::warn!(error = %e, "Failed to clear RLS context before commit");
    }

    tx.commit().await.map_err(|e| {
        tracing::error!(error = %e, "Failed to commit agency-provisioning transaction");
        db_error("Failed to commit transaction")
    })?;

    // ---- 5. Invalidate the host-resolution cache -------------------------
    // So the freshly-verified host resolves on the very next request instead of
    // waiting out a stale negative-cache entry.
    //
    // NOTE (Wave 4 integration): `tenant_resolution_cache` is added to
    // `AppState` by sibling agent 3X on branch `phase1/devmode-sliver`'s
    // counterpart. In THIS isolated worktree that field does not yet exist, so
    // this is the single expected `cargo check` error. It resolves on merge.
    state.tenant_resolution_cache.invalidate(&host).await;

    tracing::info!(
        admin_id = %admin_id,
        admin_email = %admin_email,
        organization_id = %organization.id,
        slug = %slug,
        host = %host,
        "Agency provisioned"
    );

    Ok((
        StatusCode::CREATED,
        Json(CreateAgencyResponse {
            organization,
            domain,
        }),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_slug_accepts_lowercase_digits_hyphens() {
        assert!(is_valid_slug("acme"));
        assert!(is_valid_slug("acme-2"));
        assert!(is_valid_slug("a1-b2-c3"));
    }

    #[test]
    fn valid_slug_rejects_bad_input() {
        assert!(!is_valid_slug(""));
        assert!(!is_valid_slug("Acme"));
        assert!(!is_valid_slug("ac_me"));
        assert!(!is_valid_slug("ac.me"));
        assert!(!is_valid_slug("ac me"));
        assert!(!is_valid_slug(&"a".repeat(64)));
    }

    #[test]
    fn plausible_email_basic() {
        assert!(is_plausible_email("ops@acme.example"));
        assert!(!is_plausible_email(""));
        assert!(!is_plausible_email("no-at-sign"));
        assert!(!is_plausible_email("@acme.example"));
        assert!(!is_plausible_email("ops@"));
    }

    #[test]
    fn valid_host_basic() {
        assert!(is_valid_host("acme.dev.rlt.sk"));
        assert!(is_valid_host("acme-2.dev.rlt.sk"));
        assert!(!is_valid_host(""));
        assert!(!is_valid_host("Acme.dev.rlt.sk"));
        assert!(!is_valid_host(".acme.dev.rlt.sk"));
        assert!(!is_valid_host("acme.dev.rlt.sk."));
        assert!(!is_valid_host("acme_dev.rlt.sk"));
    }
}
