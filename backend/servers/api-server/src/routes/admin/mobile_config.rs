//! Admin mobile-config write surface.
//!
//! Backs the admin-console mobile-config Save flow. Exposes:
//!   * `GET   /api/v1/admin/mobile-config` — read the current config.
//!   * `PATCH /api/v1/admin/mobile-config` — partial update (force-update
//!     floors + React Native feature flags).
//!
//! Mounted under `/api/v1/admin/mobile-config`. Both routes are gated by
//! [`Capability::MobileConfigWrite`] via the `RequireCapability` tower layer —
//! the same platform-principal + MFA + active-grant triple enforced across the
//! Phase 5 admin tree. Persistence is the platform-global singleton
//! `mobile_config` row (migration 00230), accessed through
//! [`MobileConfigRepository`].

use admin_core::{require_capability, Capability, RequireCapability};
use api_core::extractors::principal::RequestPrincipal;
use axum::{extract::State, http::StatusCode, routing::get, Json, Router};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use db::repositories::{MobileConfig, MobileConfigPatch, MobileConfigRepository};

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route(
        "/",
        get(get_mobile_config)
            .patch(patch_mobile_config)
            .layer(require_capability(Capability::MobileConfigWrite)),
    )
}

/// Partial update to the mobile config. Only supplied fields are applied;
/// `flags` keys are merged into the stored object.
#[derive(Debug, Default, Deserialize, ToSchema)]
pub struct MobileConfigPatchRequest {
    /// Minimum iOS version (force-update floor). Empty string clears it.
    #[serde(default)]
    pub min_ios_version: Option<String>,
    /// Minimum Android version (force-update floor). Empty string clears it.
    #[serde(default)]
    pub min_android_version: Option<String>,
    /// React Native feature flags to merge, as a JSON object of `key -> value`.
    #[serde(default)]
    #[schema(value_type = Object)]
    pub flags: Option<serde_json::Value>,
}

/// The mobile config returned to the admin console.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MobileConfigResponse {
    pub min_ios_version: Option<String>,
    pub min_android_version: Option<String>,
    #[schema(value_type = Object)]
    pub flags: serde_json::Value,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub updated_by: Option<uuid::Uuid>,
}

impl From<MobileConfig> for MobileConfigResponse {
    fn from(c: MobileConfig) -> Self {
        Self {
            min_ios_version: c.min_ios_version,
            min_android_version: c.min_android_version,
            flags: c.flags,
            updated_at: c.updated_at,
            updated_by: c.updated_by,
        }
    }
}

/// GET /api/v1/admin/mobile-config
async fn get_mobile_config(
    _cap: RequireCapability,
    State(state): State<AppState>,
) -> Result<Json<MobileConfigResponse>, (StatusCode, String)> {
    let repo = MobileConfigRepository::new(state.db.clone());
    let config = repo
        .get()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        // The migration seeds the singleton; fall back to a patch-with-nothing
        // (which upserts the row) if it is somehow absent.
        .map(MobileConfigResponse::from);

    match config {
        Some(c) => Ok(Json(c)),
        None => {
            let seeded = repo
                .patch(MobileConfigPatch::default(), None)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            Ok(Json(seeded.into()))
        }
    }
}

/// PATCH /api/v1/admin/mobile-config
async fn patch_mobile_config(
    _cap: RequireCapability,
    principal: RequestPrincipal,
    State(state): State<AppState>,
    Json(req): Json<MobileConfigPatchRequest>,
) -> Result<Json<MobileConfigResponse>, (StatusCode, String)> {
    let repo = MobileConfigRepository::new(state.db.clone());
    let updated = repo
        .patch(
            MobileConfigPatch {
                min_ios_version: req.min_ios_version,
                min_android_version: req.min_android_version,
                flags: req.flags,
            },
            Some(principal.user_id),
        )
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(updated.into()))
}
