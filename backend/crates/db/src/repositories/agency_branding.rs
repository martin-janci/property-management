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
    pub async fn upsert_rls<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        update: UpdateOrganizationBranding,
    ) -> Result<AgencyBranding, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // `css_vars` defaults to `{}` at the column level; we only override
        // when an explicit value is passed.
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
                logo_url        = COALESCE(EXCLUDED.logo_url,        agency_branding.logo_url),
                banner_url      = COALESCE(EXCLUDED.banner_url,      agency_branding.banner_url),
                primary_color   = COALESCE(EXCLUDED.primary_color,   agency_branding.primary_color),
                secondary_color = COALESCE(EXCLUDED.secondary_color, agency_branding.secondary_color),
                font_family     = COALESCE(EXCLUDED.font_family,     agency_branding.font_family),
                css_vars        = EXCLUDED.css_vars,
                updated_at      = NOW()
            RETURNING id, organization_id, logo_url, banner_url, primary_color,
                      secondary_color, font_family, css_vars, updated_at
            "#,
        )
        .bind(organization_id)
        .bind(&update.logo_url)
        .bind(&update.banner_url)
        .bind(&update.primary_color)
        .bind(&update.secondary_color)
        .bind(&update.font_family)
        .bind(&css_vars)
        .fetch_one(executor)
        .await
    }
}
