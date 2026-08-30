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

// ---------------------------------------------------------------------------
// H7 — URL hygiene
// ---------------------------------------------------------------------------
// `logo_url`, `banner_url`, `watermark_url` end up in `<img src=>` on the
// frontend. Reject `javascript:`, `data:`, `file:` and any non-http(s) URL.
// Inlined locally until the auth-hardening branch lands a shared validator.
const MAX_URL_LEN: usize = 2048;

fn validate_image_or_link_url(s: &str) -> Result<(), String> {
    if s.chars().count() > MAX_URL_LEN {
        return Err(format!("URL must be at most {} characters", MAX_URL_LEN));
    }
    let parsed = url::Url::parse(s).map_err(|_| "Invalid URL".to_string())?;
    match parsed.scheme() {
        "https" | "http" => Ok(()),
        _ => Err("URL scheme must be http or https".to_string()),
    }
}

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

/// Update branding request.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdateBrandingRequest {
    pub logo_url: Option<String>,
    pub banner_url: Option<String>,
    pub primary_color: Option<String>,
    pub secondary_color: Option<String>,
    pub watermark_style: Option<WatermarkStyle>,
    /// Opacity between 0.0 and 1.0.
    pub watermark_opacity: Option<f64>,
    pub watermark_position: Option<WatermarkPosition>,
    pub watermark_url: Option<String>,
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
    // H7: validate URL fields before doing any DB work. These end up in
    // `<img src=>` on the frontend — reject `javascript:`, `data:`, `file:`.
    check_optional_url_field(&data.logo_url, "logo_url")?;
    check_optional_url_field(&data.banner_url, "banner_url")?;
    check_optional_url_field(&data.watermark_url, "watermark_url")?;

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

    let watermark_style_str = data.watermark_style.as_ref().map(|s| s.to_string());
    let watermark_position_str = data.watermark_position.as_ref().map(|p| p.to_string());

    let row = sqlx::query(
        r#"
        INSERT INTO agency_branding
            (agency_id, logo_url, banner_url, primary_color, secondary_color,
             watermark_style, watermark_opacity, watermark_position, watermark_url)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        ON CONFLICT (agency_id) DO UPDATE SET
            logo_url           = COALESCE(EXCLUDED.logo_url, agency_branding.logo_url),
            banner_url         = COALESCE(EXCLUDED.banner_url, agency_branding.banner_url),
            primary_color      = COALESCE(EXCLUDED.primary_color, agency_branding.primary_color),
            secondary_color    = COALESCE(EXCLUDED.secondary_color, agency_branding.secondary_color),
            watermark_style    = COALESCE(EXCLUDED.watermark_style, agency_branding.watermark_style),
            watermark_opacity  = COALESCE(EXCLUDED.watermark_opacity, agency_branding.watermark_opacity),
            watermark_position = COALESCE(EXCLUDED.watermark_position, agency_branding.watermark_position),
            watermark_url      = COALESCE(EXCLUDED.watermark_url, agency_branding.watermark_url),
            updated_at         = NOW()
        RETURNING logo_url, banner_url, primary_color, secondary_color,
                  watermark_style, watermark_opacity, watermark_position, watermark_url, updated_at
        "#,
    )
    .bind(agency_id)
    .bind(&data.logo_url)
    .bind(&data.banner_url)
    .bind(&data.primary_color)
    .bind(&data.secondary_color)
    .bind(&watermark_style_str)
    .bind(data.watermark_opacity)
    .bind(&watermark_position_str)
    .bind(&data.watermark_url)
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
