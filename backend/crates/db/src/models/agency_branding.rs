//! Agency branding model (Phase 3: Hosting & Theming).
//!
//! Branding is keyed by `organization_id` for the runtime-theming pipeline
//! (the Phase 1 keystone middleware resolves Host -> organization_id, and
//! `/tenant-config` reads branding via that id). A single row per org carries
//! logo, colors, font and an open-ended JSONB blob of CSS-variable overrides
//! consumed by `reality-web`'s `--ppt-*` design-token system.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/// One branding record per organization.
///
/// Matches the columns added in migration `00133_extend_agency_branding.sql`.
/// Uses `organization_id` (the Phase 1 RLS tenant) — the legacy
/// `agency_id`-keyed rows from migration 00108 are still supported (the
/// column is now nullable) but are not produced by Phase 3 endpoints.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct AgencyBranding {
    pub id: Uuid,
    pub organization_id: Option<Uuid>,
    pub logo_url: Option<String>,
    pub banner_url: Option<String>,
    pub primary_color: Option<String>,
    pub secondary_color: Option<String>,
    pub font_family: Option<String>,
    /// Free-form CSS variable overrides — e.g. `{ "--ppt-radius-md": "8px" }`.
    /// Sent verbatim into the HTML root style attribute by the reality-web
    /// layout. Defaults to `{}`.
    pub css_vars: Value,
    pub updated_at: DateTime<Utc>,
}

/// Update payload for the per-tenant branding admin endpoint
/// (`PUT /admin/tenants/{org_id}/branding`).
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct UpdateOrganizationBranding {
    #[serde(default)]
    pub logo_url: Option<String>,
    #[serde(default)]
    pub banner_url: Option<String>,
    #[serde(default)]
    pub primary_color: Option<String>,
    #[serde(default)]
    pub secondary_color: Option<String>,
    #[serde(default)]
    pub font_family: Option<String>,
    #[serde(default)]
    pub css_vars: Option<Value>,
}
