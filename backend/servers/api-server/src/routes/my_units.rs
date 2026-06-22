//! Resident-facing "My Unit" routes (Epic 3, Story 3.6).
//!
//! A read-only, resident-scoped view of the unit(s) the authenticated user is
//! a resident of. Unlike the manager-facing unit/resident endpoints under
//! `/buildings/{id}/units/...` (which require org membership and expose every
//! resident's and owner's PII), this surface is resolved strictly by the
//! caller's own `user_id` and never leaks other residents' or owners'
//! identities. The privacy filter is enforced server-side in the query layer
//! ([`db::repositories::UnitResidentRepository::find_my_units`]), not just
//! hidden in the UI.

use api_core::extractors::AuthUser;
use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::get, Json, Router};
use common::errors::ErrorResponse;
use db::models::unit_resident::MyUnitRow;
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::state::AppState;

/// Create the resident "My Unit" router (mounted at `/api/v1/users/me/units`).
pub fn router() -> Router<AppState> {
    Router::new().route("/", get(list_my_units))
}

// ==================== Response Types ====================

/// The caller's own association with a unit (their residency).
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MyUnitAssociation {
    /// Residency (unit_residents) id.
    pub resident_id: Uuid,
    /// owner | tenant | family_member | subtenant.
    pub resident_type: String,
    /// Human-readable resident type.
    pub resident_type_display: String,
    /// Whether this is the primary contact residency for the unit.
    pub is_primary: bool,
    /// Residency start date (ISO-8601).
    pub start_date: String,
    /// Residency end date, if ended (ISO-8601). `None` while active.
    pub end_date: Option<String>,
    /// Whether the residency is currently active.
    pub is_active: bool,
    /// Whether the resident receives notifications for this unit.
    pub receives_notifications: bool,
    /// Whether the resident receives mail for this unit.
    pub receives_mail: bool,
}

/// The unit's own (non-PII) details, as shown to its residents.
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MyUnitDetails {
    pub id: Uuid,
    pub building_id: Uuid,
    pub entrance: Option<String>,
    pub designation: String,
    pub floor: i32,
    /// Floor display (e.g. `G`, `B1`).
    pub floor_display: String,
    pub unit_type: String,
    pub size_sqm: Option<String>,
    pub rooms: Option<i32>,
    pub ownership_share: String,
    pub occupancy_status: String,
    pub description: Option<String>,
    pub status: String,
}

/// Minimal building address context for the unit.
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MyUnitBuilding {
    pub id: Uuid,
    pub name: Option<String>,
    pub street: String,
    pub city: String,
    pub postal_code: String,
}

/// One "My Unit" entry: the caller's association + the unit + its building.
#[derive(Debug, Serialize, ToSchema)]
pub struct MyUnitResponse {
    pub association: MyUnitAssociation,
    pub unit: MyUnitDetails,
    pub building: MyUnitBuilding,
}

fn resident_type_display(rtype: &str) -> &str {
    match rtype {
        "owner" => "Owner",
        "tenant" => "Tenant",
        "family_member" => "Family Member",
        "subtenant" => "Subtenant",
        other => other,
    }
}

fn floor_display(floor: i32) -> String {
    match floor {
        f if f < 0 => format!("B{}", f.abs()),
        0 => "G".to_string(),
        f => format!("{f}"),
    }
}

impl From<MyUnitRow> for MyUnitResponse {
    fn from(r: MyUnitRow) -> Self {
        let is_active = r.end_date.is_none();
        Self {
            association: MyUnitAssociation {
                resident_id: r.resident_id,
                resident_type_display: resident_type_display(&r.resident_type).to_string(),
                resident_type: r.resident_type,
                is_primary: r.is_primary,
                start_date: r.start_date.to_string(),
                end_date: r.end_date.map(|d| d.to_string()),
                is_active,
                receives_notifications: r.receives_notifications,
                receives_mail: r.receives_mail,
            },
            unit: MyUnitDetails {
                id: r.unit_id,
                building_id: r.building_id,
                entrance: r.entrance,
                designation: r.designation,
                floor: r.floor,
                floor_display: floor_display(r.floor),
                unit_type: r.unit_type,
                size_sqm: r.size_sqm.map(|d| d.to_string()),
                rooms: r.rooms,
                ownership_share: r.ownership_share.to_string(),
                occupancy_status: r.occupancy_status,
                description: r.description,
                status: r.unit_status,
            },
            building: MyUnitBuilding {
                id: r.building_id,
                name: r.building_name,
                street: r.building_street,
                city: r.building_city,
                postal_code: r.building_postal_code,
            },
        }
    }
}

// ==================== Handlers ====================

/// List the authenticated resident's units (Story 3.6).
///
/// Resolves the caller's active `unit_residents` associations and returns each
/// unit's details plus the caller's association status. Privacy-respecting:
/// other residents and owners are never included.
#[utoipa::path(
    get,
    path = "/api/v1/users/me/units",
    tag = "My Unit",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "The caller's units", body = Vec<MyUnitResponse>),
        (status = 401, description = "Not authenticated", body = ErrorResponse)
    )
)]
pub async fn list_my_units(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let rows = state
        .unit_resident_repo
        .find_my_units(auth.user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to load resident units");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to load your units")),
            )
        })?;

    let response: Vec<MyUnitResponse> = rows.into_iter().map(Into::into).collect();
    Ok(Json(response))
}
