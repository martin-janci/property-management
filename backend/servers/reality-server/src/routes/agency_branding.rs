//! Agency branding routes (UC-49: Agency Branding).
//!
//! GET/PUT branding settings for an agency.
//! Only agency members may update branding.
//! D1.2: handlers now use the unified `RequestPrincipal` extractor.

use crate::state::AppState;
use api_core::extractors::RequestPrincipal;
use axum::{
    extract::{Path, State},
    routing::{get, put},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use utoipa::ToSchema;
use uuid::Uuid;

// H7 — URL hygiene. `logo_url`, `banner_url`, `watermark_url` end up in
// `<img src=>` on the frontend; the shared `validate_image_or_link_url`
// rejects `javascript:`, `data:`, `file:` and any non-http(s) URL.
use crate::util::url::validate_image_or_link_url;

/// Validate an optional URL field. Returns a generic 400 message that does
/// NOT echo the rejected URL back (reflected-XSS guard).
fn check_optional_url_field(
    value: &Option<String>,
    field: &'static str,
) -> Result<(), (axum::http::StatusCode, String)> {
    if let Some(s) = value.as_deref() {
        if !s.is_empty() && validate_image_or_link_url(s).is_err() {
            return Err((
                axum::http::StatusCode::BAD_REQUEST,
                format!("{} must be a http(s) URL (max 2048 chars)", field),
            ));
        }
    }
    Ok(())
}

/// Create agency branding router.
/// Mounted at /api/v1/agencies/:id/branding to avoid prefix collision with
/// routes::agencies (which nests under /api/v1/agencies — Axum's .nest() with
/// the same prefix shadows the earlier router).
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(get_branding))
        .route("/", put(update_branding))
}

/// Watermark position enum.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum WatermarkPosition {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    Center,
}

impl std::fmt::Display for WatermarkPosition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            WatermarkPosition::TopLeft => "top_left",
            WatermarkPosition::TopRight => "top_right",
            WatermarkPosition::BottomLeft => "bottom_left",
            WatermarkPosition::BottomRight => "bottom_right",
            WatermarkPosition::Center => "center",
        };
        write!(f, "{}", s)
    }
}

/// Watermark style enum.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum WatermarkStyle {
    Logo,
    Text,
    LogoAndText,
}

impl std::fmt::Display for WatermarkStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            WatermarkStyle::Logo => "logo",
            WatermarkStyle::Text => "text",
            WatermarkStyle::LogoAndText => "logo_and_text",
        };
        write!(f, "{}", s)
    }
}

/// Agency branding settings.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AgencyBranding {
    pub agency_id: Uuid,
    pub logo_url: Option<String>,
    pub banner_url: Option<String>,
    pub primary_color: Option<String>,
    pub secondary_color: Option<String>,
    pub watermark_style: Option<String>,
    pub watermark_opacity: Option<f64>,
    pub watermark_position: Option<String>,
    pub watermark_url: Option<String>,
    pub updated_at: DateTime<Utc>,
}

/// Deserialize a field as a double `Option` so the handler can tell "absent"
/// (`None`) from "explicitly null" (`Some(None)`) and "set to a value"
/// (`Some(Some(v))`). Paired with `#[serde(default)]`: an absent key yields
/// `None`; a present key (null OR a value) runs this and wraps the inner
/// `Option` in `Some`. Without it, serde collapses an explicit `null` to
/// `None`, making a clear-to-null indistinguishable from an omitted field —
/// the exact reason the old COALESCE upsert could never clear a branding
/// field. (Mirrors `documents::core::double_option`, #1589.)
fn double_option<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}

/// Split a patch field into `(present, value)`:
/// - `None`          → `(false, None)` — field absent, leave the column unchanged.
/// - `Some(None)`    → `(true, None)`  — explicit null, clear the column.
/// - `Some(Some(v))` → `(true, Some(v))` — set the column.
fn patch_parts<T>(field: Option<Option<T>>) -> (bool, Option<T>) {
    match field {
        None => (false, None),
        Some(inner) => (true, inner),
    }
}

/// Like `patch_parts` but for string columns: a present-but-empty string is
/// treated as a clear (stored as NULL), so a client can clear a field with
/// either `null` or `""`.
fn patch_parts_str(field: Option<Option<String>>) -> (bool, Option<String>) {
    match field {
        None => (false, None),
        Some(None) => (true, None),
        Some(Some(s)) if s.is_empty() => (true, None),
        Some(Some(s)) => (true, Some(s)),
    }
}

/// Update branding request.
///
/// Every field is a double `Option` (via `double_option`) so the handler can
/// distinguish three client intents per field: **absent** (leave unchanged),
/// **explicit null / empty string** (clear the field back to NULL), and **a
/// value** (set it). The wire shape is unchanged — each field is still an
/// optional, nullable value.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdateBrandingRequest {
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
    #[schema(value_type = Option<WatermarkStyle>, nullable)]
    pub watermark_style: Option<Option<WatermarkStyle>>,
    /// Opacity between 0.0 and 1.0.
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<f64>, nullable)]
    pub watermark_opacity: Option<Option<f64>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<WatermarkPosition>, nullable)]
    pub watermark_position: Option<Option<WatermarkPosition>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>, nullable)]
    pub watermark_url: Option<Option<String>>,
}

/// Branding response.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BrandingResponse {
    pub branding: AgencyBranding,
}

/// Get agency branding settings.
#[utoipa::path(
    get,
    path = "/api/v1/agencies/{id}/branding",
    tag = "AgencyBranding",
    params(("id" = Uuid, Path, description = "Agency ID")),
    responses(
        (status = 200, description = "Agency branding settings", body = BrandingResponse),
        (status = 404, description = "Agency not found")
    )
)]
pub async fn get_branding(
    State(state): State<AppState>,
    Path(agency_id): Path<Uuid>,
) -> Result<Json<BrandingResponse>, (axum::http::StatusCode, String)> {
    let mut conn = state
        .acquire_public_conn()
        .await
        .map_err(|e| crate::util::errors::db_error("database error", e))?;

    let row = sqlx::query(
        r#"
        SELECT logo_url, banner_url, primary_color, secondary_color,
               watermark_style, watermark_opacity, watermark_position, watermark_url,
               updated_at
        FROM agency_branding
        WHERE agency_id = $1
        "#,
    )
    .bind(agency_id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(|e| crate::util::errors::db_error("get branding", e))?;

    match row {
        Some(r) => {
            let branding = AgencyBranding {
                agency_id,
                logo_url: r.get("logo_url"),
                banner_url: r.get("banner_url"),
                primary_color: r.get("primary_color"),
                secondary_color: r.get("secondary_color"),
                watermark_style: r.get("watermark_style"),
                watermark_opacity: r.get("watermark_opacity"),
                watermark_position: r.get("watermark_position"),
                watermark_url: r.get("watermark_url"),
                updated_at: r.get("updated_at"),
            };
            Ok(Json(BrandingResponse { branding }))
        }
        None => {
            // Check agency exists; if so, return empty branding row
            let agency_exists: bool =
                sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM reality_agencies WHERE id = $1)")
                    .bind(agency_id)
                    .fetch_one(&mut *conn)
                    .await
                    .map_err(|e| crate::util::errors::db_error("check agency", e))?;

            if !agency_exists {
                return Err((
                    axum::http::StatusCode::NOT_FOUND,
                    "Agency not found".to_string(),
                ));
            }

            // Return default empty branding (agency exists but has no branding record yet)
            let branding = AgencyBranding {
                agency_id,
                logo_url: None,
                banner_url: None,
                primary_color: None,
                secondary_color: None,
                watermark_style: None,
                watermark_opacity: None,
                watermark_position: None,
                watermark_url: None,
                updated_at: chrono::Utc::now(),
            };
            Ok(Json(BrandingResponse { branding }))
        }
    }
}

/// Update agency branding (agency members only).
#[utoipa::path(
    put,
    path = "/api/v1/agencies/{id}/branding",
    tag = "AgencyBranding",
    params(("id" = Uuid, Path, description = "Agency ID")),
    request_body = UpdateBrandingRequest,
    responses(
        (status = 200, description = "Branding updated", body = BrandingResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Not a member of this agency"),
        (status = 404, description = "Agency not found")
    ),
    security(("session_token" = []))
)]
pub async fn update_branding(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(agency_id): Path<Uuid>,
    Json(data): Json<UpdateBrandingRequest>,
) -> Result<Json<BrandingResponse>, (axum::http::StatusCode, String)> {
    // Split each patch field into (present, value). `present` distinguishes an
    // absent field (leave the column unchanged) from an explicit null/empty
    // (clear it) — the fix for the COALESCE-can-never-clear defect.
    let (logo_present, logo_url) = patch_parts_str(data.logo_url);
    let (banner_present, banner_url) = patch_parts_str(data.banner_url);
    let (primary_present, primary_color) = patch_parts_str(data.primary_color);
    let (secondary_present, secondary_color) = patch_parts_str(data.secondary_color);
    let (wstyle_present, watermark_style) = patch_parts(data.watermark_style);
    let (wopacity_present, watermark_opacity) = patch_parts(data.watermark_opacity);
    let (wposition_present, watermark_position) = patch_parts(data.watermark_position);
    let (wurl_present, watermark_url) = patch_parts_str(data.watermark_url);

    // H7: validate URL fields before doing any DB work. These end up in
    // `<img src=>` on the frontend — reject `javascript:`, `data:`, `file:`.
    // Cleared/empty values are normalized to `None` above and skipped.
    check_optional_url_field(&logo_url, "logo_url")?;
    check_optional_url_field(&banner_url, "banner_url")?;
    check_optional_url_field(&watermark_url, "watermark_url")?;

    let mut conn = state
        .acquire_public_conn()
        .await
        .map_err(|e| crate::util::errors::db_error("database error", e))?;

    // Check agency exists
    let agency_exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM reality_agencies WHERE id = $1)")
            .bind(agency_id)
            .fetch_one(&mut *conn)
            .await
            .map_err(|e| crate::util::errors::db_error("check agency", e))?;

    if !agency_exists {
        return Err((
            axum::http::StatusCode::NOT_FOUND,
            "Agency not found".to_string(),
        ));
    }

    // Check user is an active member
    let is_member: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM reality_agency_members
            WHERE agency_id = $1 AND user_id = $2 AND is_active = TRUE
        )
        "#,
    )
    .bind(agency_id)
    .bind(principal.user_id)
    .fetch_one(&mut *conn)
    .await
    .map_err(|e| crate::util::errors::db_error("check membership", e))?;

    if !is_member {
        return Err((
            axum::http::StatusCode::FORBIDDEN,
            "Not a member of this agency".to_string(),
        ));
    }

    let watermark_style_str = watermark_style.as_ref().map(|s| s.to_string());
    let watermark_position_str = watermark_position.as_ref().map(|p| p.to_string());

    // Per-field `CASE WHEN $present` instead of `COALESCE(EXCLUDED.x, old)`:
    // when a field is present (`$10..=$17` true) we take the new value — even
    // when it is NULL — so a client can clear it; when a field is absent we
    // keep the existing value. On the pure-INSERT path the VALUES clause writes
    // the bound value directly (absent → NULL), which is correct for a new row.
    let row = sqlx::query(
        r#"
        INSERT INTO agency_branding
            (agency_id, logo_url, banner_url, primary_color, secondary_color,
             watermark_style, watermark_opacity, watermark_position, watermark_url)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        ON CONFLICT (agency_id) DO UPDATE SET
            logo_url           = CASE WHEN $10 THEN EXCLUDED.logo_url           ELSE agency_branding.logo_url           END,
            banner_url         = CASE WHEN $11 THEN EXCLUDED.banner_url         ELSE agency_branding.banner_url         END,
            primary_color      = CASE WHEN $12 THEN EXCLUDED.primary_color      ELSE agency_branding.primary_color      END,
            secondary_color    = CASE WHEN $13 THEN EXCLUDED.secondary_color    ELSE agency_branding.secondary_color    END,
            watermark_style    = CASE WHEN $14 THEN EXCLUDED.watermark_style    ELSE agency_branding.watermark_style    END,
            watermark_opacity  = CASE WHEN $15 THEN EXCLUDED.watermark_opacity  ELSE agency_branding.watermark_opacity  END,
            watermark_position = CASE WHEN $16 THEN EXCLUDED.watermark_position ELSE agency_branding.watermark_position END,
            watermark_url      = CASE WHEN $17 THEN EXCLUDED.watermark_url      ELSE agency_branding.watermark_url      END,
            updated_at         = NOW()
        RETURNING logo_url, banner_url, primary_color, secondary_color,
                  watermark_style, watermark_opacity, watermark_position, watermark_url, updated_at
        "#,
    )
    .bind(agency_id)
    .bind(&logo_url)
    .bind(&banner_url)
    .bind(&primary_color)
    .bind(&secondary_color)
    .bind(&watermark_style_str)
    .bind(watermark_opacity)
    .bind(&watermark_position_str)
    .bind(&watermark_url)
    .bind(logo_present)
    .bind(banner_present)
    .bind(primary_present)
    .bind(secondary_present)
    .bind(wstyle_present)
    .bind(wopacity_present)
    .bind(wposition_present)
    .bind(wurl_present)
    .fetch_one(&mut *conn)
    .await
    .map_err(|e| crate::util::errors::db_error("update branding", e))?;

    let branding = AgencyBranding {
        agency_id,
        logo_url: row.get("logo_url"),
        banner_url: row.get("banner_url"),
        primary_color: row.get("primary_color"),
        secondary_color: row.get("secondary_color"),
        watermark_style: row.get("watermark_style"),
        watermark_opacity: row.get("watermark_opacity"),
        watermark_position: row.get("watermark_position"),
        watermark_url: row.get("watermark_url"),
        updated_at: row.get("updated_at"),
    };

    Ok(Json(BrandingResponse { branding }))
}
