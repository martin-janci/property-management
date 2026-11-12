//! Agency branding repository (Phase 3: Hosting & Theming).
//!
//! # RLS integration
//!
//! Branding is read on TWO paths and the repository exposes both:
//!
//! 1. **System lookup** (`fetch_by_organization_system`) — used by the
//!    `/tenant-config` endpoint, which serves an unauthenticated request
//!    that has been resolved to an organization by the Phase 1 host
//!    middleware. There is no JWT, so `RlsConnection` cannot apply.
//!    Acquires a connection, sets a system context for ONE constrained
//!    `SELECT`, clears it. Mirrors `AgencyDomainRepository::resolve_host_system`.
//!
//! 2. **RLS-aware** (`upsert_rls`) — used by the platform-admin update
//!    endpoint which already holds a super-admin RLS context.
//!
//! # Why both
//!
//! The `/tenant-config` endpoint is intentionally pre-auth (called by every
//! browser hit on a tenant host). It must read branding without a JWT, so it
//! goes through the system path. The admin write path is JWT-gated and
//! goes through the RLS path.

use crate::models::agency_branding::{AgencyBranding, UpdateOrganizationBranding};
use crate::DbPool;
use serde_json::Value;
use sqlx::{Error as SqlxError, Executor, Postgres};
use uuid::Uuid;

#[derive(Clone)]
pub struct AgencyBrandingRepository {
    pool: DbPool,
}

impl AgencyBrandingRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Read branding for `organization_id` on a system connection.
    ///
    /// Used by `/tenant-config` after the host has been resolved by Phase 1
    /// middleware. Returns `Ok(None)` when the org has no branding row yet —
    /// callers fall back to the platform default.
    pub async fn fetch_by_organization_system(
        &self,
        organization_id: Uuid,
    ) -> Result<Option<AgencyBranding>, SqlxError> {
        let mut conn = self.pool.acquire().await?;

        crate::tenant_context::set_request_context(&mut *conn, None, None, true).await?;

        let result = sqlx::query_as::<_, AgencyBranding>(
            r#"
            SELECT id, organization_id, logo_url, banner_url, primary_color,
                   secondary_color, font_family, css_vars, updated_at
            FROM agency_branding
            WHERE organization_id = $1
            LIMIT 1
            "#,
        )
        .bind(organization_id)
        .fetch_optional(&mut *conn)
        .await;

        if let Err(e) = crate::tenant_context::clear_request_context(&mut *conn).await {
            tracing::warn!(
                error = %e,
                "Failed to clear system RLS context after fetch_by_organization_system"
            );
        }

        result
    }

    /// Upsert branding for `organization_id`. Caller must already hold an
    /// RLS context that authorizes writing this org's branding (super-admin
    /// for Phase 3; capability-gated in Phase 5).
    ///
    /// # Three-state partial update
    ///
    /// Each nullable field is applied with three-state semantics so a client
    /// can CLEAR a field back to NULL rather than only ever setting it:
    ///   * **absent** (`None`)            → leave the column unchanged,
    ///   * **explicit null / empty** (`Some(None)` or `Some(Some(""))`)
    ///                                     → clear the column to NULL,
    ///   * **a value** (`Some(Some(v))`)  → set the column.
    ///
    /// This replaces the previous `COALESCE(EXCLUDED.x, agency_branding.x)`
    /// upsert, under which an explicit null bound as SQL NULL and `COALESCE`
    /// kept the old value — making a clear indistinguishable from "no change".
    /// Each field now carries a `present` flag and the `ON CONFLICT` branch
    /// uses `CASE WHEN $present THEN EXCLUDED.x ELSE agency_branding.x END`.
    /// Mirrors the reality-server fix (#3430).
    pub async fn upsert_rls<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        update: UpdateOrganizationBranding,
    ) -> Result<AgencyBranding, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // Split each nullable patch field into (present, value). `present`
        // distinguishes an absent field (leave unchanged) from an explicit
        // null/empty (clear to NULL) — the fix for the COALESCE-can-never-clear
        // defect. A present-but-empty string clears the column, so a client can
        // clear via `""` as well as `null`.
        let (logo_present, logo_url) = patch_str(update.logo_url);
        let (banner_present, banner_url) = patch_str(update.banner_url);
        let (primary_present, primary_color) = patch_str(update.primary_color);
        let (secondary_present, secondary_color) = patch_str(update.secondary_color);
        let (font_present, font_family) = patch_str(update.font_family);

        // `css_vars` is `NOT NULL` (defaults to `{}`), so it has only two
        // states: present → set to the provided object, absent → leave the
        // existing value untouched (the old code wrote `{}` on an absent field,
        // silently wiping stored overrides).
        let css_present = update.css_vars.is_some();
        let css_vars: Value = update.css_vars.unwrap_or(Value::Object(Default::default()));

        sqlx::query_as::<_, AgencyBranding>(
            r#"
            INSERT INTO agency_branding (
                organization_id, logo_url, banner_url, primary_color,
                secondary_color, font_family, css_vars
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (organization_id)
                WHERE organization_id IS NOT NULL
            DO UPDATE SET
                logo_url        = CASE WHEN $8  THEN EXCLUDED.logo_url        ELSE agency_branding.logo_url        END,
                banner_url      = CASE WHEN $9  THEN EXCLUDED.banner_url      ELSE agency_branding.banner_url      END,
                primary_color   = CASE WHEN $10 THEN EXCLUDED.primary_color   ELSE agency_branding.primary_color   END,
                secondary_color = CASE WHEN $11 THEN EXCLUDED.secondary_color ELSE agency_branding.secondary_color END,
                font_family     = CASE WHEN $12 THEN EXCLUDED.font_family     ELSE agency_branding.font_family     END,
                css_vars        = CASE WHEN $13 THEN EXCLUDED.css_vars        ELSE agency_branding.css_vars        END,
                updated_at      = NOW()
            RETURNING id, organization_id, logo_url, banner_url, primary_color,
                      secondary_color, font_family, css_vars, updated_at
            "#,
        )
        .bind(organization_id)
        .bind(&logo_url)
        .bind(&banner_url)
        .bind(&primary_color)
        .bind(&secondary_color)
        .bind(&font_family)
        .bind(&css_vars)
        .bind(logo_present)
        .bind(banner_present)
        .bind(primary_present)
        .bind(secondary_present)
        .bind(font_present)
        .bind(css_present)
        .fetch_one(executor)
        .await
    }
}

/// Split a nullable string patch field into `(present, value)` for the
/// three-state upsert:
/// - `None`                    → `(false, None)` — absent, leave unchanged.
/// - `Some(None)`              → `(true, None)`  — explicit null, clear to NULL.
/// - `Some(Some(s))` where `s` is empty → `(true, None)` — empty string clears.
/// - `Some(Some(s))`           → `(true, Some(s))` — set the column.
fn patch_str(field: Option<Option<String>>) -> (bool, Option<String>) {
    match field {
        None => (false, None),
        Some(None) => (true, None),
        Some(Some(s)) if s.is_empty() => (true, None),
        Some(Some(s)) => (true, Some(s)),
    }
}
