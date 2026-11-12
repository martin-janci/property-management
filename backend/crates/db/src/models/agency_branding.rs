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

/// Deserialize a field as a double `Option` so the caller can tell "absent"
/// (`None`) from "explicitly null" (`Some(None)`) and "set to a value"
/// (`Some(Some(v))`). Paired with `#[serde(default)]`: an absent key yields
/// `None`; a present key (null OR a value) runs this and wraps the inner
/// `Option` in `Some`. Without it, serde collapses an explicit `null` to
/// `None`, making a clear-to-null indistinguishable from an omitted field —
/// the exact reason the old COALESCE upsert in `AgencyBrandingRepository::
/// upsert_rls` could never clear a branding field. Mirrors the reality-server
/// fix (#3430) and `documents::core::double_option` (#1589).
fn double_option<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}

/// Update payload for the per-tenant branding admin endpoint
/// (`PUT /admin/tenants/{org_id}/branding`).
///
/// Every nullable field is a double `Option` (via `double_option`) so the
/// upsert can distinguish three client intents per field: **absent** (leave
/// the column unchanged), **explicit null / empty string** (clear the column
/// back to NULL), and **a value** (set it). The wire shape is unchanged — each
/// field is still an optional, nullable value. `css_vars` is `NOT NULL`
/// (defaults to `{}`), so it has only two states: present → set, absent →
/// leave unchanged.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct UpdateOrganizationBranding {
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>, nullable)]
    pub logo_url: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>, nullable)]
    pub banner_url: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>, nullable)]
    pub primary_color: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>, nullable)]
    pub secondary_color: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>, nullable)]
    pub font_family: Option<Option<String>>,
    #[serde(default)]
    pub css_vars: Option<Value>,
}
