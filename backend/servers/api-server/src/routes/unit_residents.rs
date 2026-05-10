//! Unit residents routes (Epic 3, Story 3.3).
//!
//! Implements resident management for units including adding, updating,
//! and ending residencies.

use api_core::extractors::{AuthUser, RlsConnection};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};
use chrono::NaiveDate;
use common::errors::ErrorResponse;
use db::models::unit_resident::{
    resident_type, CreateUnitResident, UnitResident, UnitResidentWithUser, UpdateUnitResident,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::state::AppState;

/// Create unit residents router.
pub fn router() -> Router<AppState> {
    Router::new()
        // Resident management (Story 3.3)
        .route("/", get(list_residents))
        .route("/", post(add_resident))
        .route("/{resident_id}", get(get_resident))
        .route("/{resident_id}", put(update_resident))
        .route("/{resident_id}", delete(remove_resident))
        .route("/{resident_id}/end", post(end_residency))
        .route("/history", get(list_resident_history))
}

// ==================== Request/Response Types ====================

/// Add resident request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct AddResidentRequest {
    /// User ID to add as resident
    pub user_id: Uuid,
    /// Type of resident (owner, tenant, family_member, subtenant)
    pub resident_type: String,
    /// Is this the primary contact for the unit?
    #[serde(default)]
    pub is_primary: bool,
    /// When residency starts (defaults to today)
    pub start_date: Option<NaiveDate>,
    /// Whether resident receives notifications
    #[serde(default = "default_true")]
    pub receives_notifications: bool,
    /// Whether resident receives mail
    #[serde(default = "default_true")]
    pub receives_mail: bool,
    /// Notes about the resident
    pub notes: Option<String>,
}

fn default_true() -> bool {
    true
}

/// Update resident request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateResidentRequest {
    /// Type of resident
    pub resident_type: Option<String>,
    /// Is this the primary contact?
    pub is_primary: Option<bool>,
    /// Whether resident receives notifications
    pub receives_notifications: Option<bool>,
    /// Whether resident receives mail
    pub receives_mail: Option<bool>,
    /// Notes about the resident
    pub notes: Option<String>,
}

/// End residency request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct EndResidencyRequest {
    /// Date when residency ends
    pub end_date: NaiveDate,
}

/// Resident response.
#[derive(Debug, Serialize, ToSchema)]
pub struct ResidentResponse {
    pub id: Uuid,
    pub unit_id: Uuid,
    pub user_id: Uuid,
    pub resident_type: String,
    pub resident_type_display: String,
    pub is_primary: bool,
    pub start_date: String,
    pub end_date: Option<String>,
    pub receives_notifications: bool,
    pub receives_mail: bool,
    pub notes: Option<String>,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl From<UnitResident> for ResidentResponse {
    fn from(r: UnitResident) -> Self {
        let is_active = r.is_active();
        let resident_type_display = r.resident_type_display().to_string();
        Self {
            id: r.id,
            unit_id: r.unit_id,
            user_id: r.user_id,
            resident_type: r.resident_type,
            resident_type_display,
            is_primary: r.is_primary,
            start_date: r.start_date.to_string(),
            end_date: r.end_date.map(|d| d.to_string()),
            receives_notifications: r.receives_notifications,
            receives_mail: r.receives_mail,
            notes: r.notes,
            is_active,
            created_at: r.created_at.to_rfc3339(),
            updated_at: r.updated_at.to_rfc3339(),
        }
    }
}

/// Resident with user info response.
#[derive(Debug, Serialize, ToSchema)]
pub struct ResidentWithUserResponse {
    pub id: Uuid,
    pub unit_id: Uuid,
    pub user_id: Uuid,
    pub user_name: String,
    pub user_email: String,
    pub resident_type: String,
    pub is_primary: bool,
    pub start_date: String,
    pub end_date: Option<String>,
    pub is_active: bool,
}

impl From<UnitResidentWithUser> for ResidentWithUserResponse {
    fn from(r: UnitResidentWithUser) -> Self {
        Self {
            id: r.id,
            unit_id: r.unit_id,
            user_id: r.user_id,
            user_name: r.user_name,
            user_email: r.user_email,
            resident_type: r.resident_type,
            is_primary: r.is_primary,
            start_date: r.start_date.to_string(),
            end_date: r.end_date.map(|d| d.to_string()),
            is_active: r.end_date.is_none(),
        }
    }
}

// ==================== Handlers ====================

/// List active residents for a unit.
#[utoipa::path(
    get,
    path = "/api/v1/buildings/{building_id}/units/{unit_id}/residents",
    tag = "Unit Residents",
    params(
        ("building_id" = Uuid, Path, description = "Building ID"),
        ("unit_id" = Uuid, Path, description = "Unit ID")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of active residents", body = Vec<ResidentWithUserResponse>),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Unit not found", body = ErrorResponse)
    )
)]
pub async fn list_residents(
    State(state): State<AppState>,
    auth: AuthUser,
    mut rls: RlsConnection,
    Path((building_id, unit_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // Verify building exists and user has access
    let building = state
        .building_repo
        .find_by_id_rls(&mut **rls.conn(), building_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get building");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Building not found")),
            )
        })?;

    // Validate user has access to the organization
    let is_member = state
        .org_member_repo
        .is_member(building.organization_id, auth.user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to check org membership");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?;

    if !is_member {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "NOT_AUTHORIZED",
                "You are not a member of this organization",
            )),
        ));
    }

    // Verify unit exists and belongs to building
    let unit = state
        .unit_repo
        .find_by_id_rls(&mut **rls.conn(), unit_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get unit");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?;

    let unit = unit.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Unit not found")),
        )
    })?;

    if unit.building_id != building_id {
        rls.release().await;
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "NOT_FOUND",
                "Unit not found in this building",
            )),
        ));
    }

    // Get active residents
    let residents = state
        .unit_resident_repo
        .find_by_unit(unit_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get residents");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to get residents")),
            )
        })?;

    let response: Vec<ResidentWithUserResponse> = residents.into_iter().map(Into::into).collect();

    rls.release().await;
    Ok(Json(response))
}

/// Add a resident to a unit.
#[utoipa::path(
    post,
    path = "/api/v1/buildings/{building_id}/units/{unit_id}/residents",
    tag = "Unit Residents",
    params(
        ("building_id" = Uuid, Path, description = "Building ID"),
        ("unit_id" = Uuid, Path, description = "Unit ID")
    ),
    request_body = AddResidentRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 201, description = "Resident added", body = ResidentResponse),
        (status = 400, description = "Invalid input", body = ErrorResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Unit not found", body = ErrorResponse),
        (status = 409, description = "User already resident of this unit", body = ErrorResponse)
    )
)]
pub async fn add_resident(
    State(state): State<AppState>,
    auth: AuthUser,
    mut rls: RlsConnection,
    Path((building_id, unit_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<AddResidentRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // Verify building exists and user has access
    let building = state
        .building_repo
        .find_by_id_rls(&mut **rls.conn(), building_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get building");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Building not found")),
            )
        })?;

    // Validate user has access to the organization
    let is_member = state
        .org_member_repo
        .is_member(building.organization_id, auth.user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to check org membership");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?;

    if !is_member {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "NOT_AUTHORIZED",
                "You are not a member of this organization",
            )),
        ));
    }

    // Verify unit exists and belongs to building
    let unit = state
        .unit_repo
        .find_by_id_rls(&mut **rls.conn(), unit_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get unit");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?;

    let unit = unit.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Unit not found")),
        )
    })?;

    if unit.building_id != building_id {
        rls.release().await;
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "NOT_FOUND",
                "Unit not found in this building",
            )),
        ));
    }

    // Validate resident type
    let valid_types = [
        resident_type::OWNER,
        resident_type::TENANT,
        resident_type::FAMILY_MEMBER,
        resident_type::SUBTENANT,
    ];
    if !valid_types.contains(&req.resident_type.as_str()) {
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_RESIDENT_TYPE",
                "Invalid resident type. Must be: owner, tenant, family_member, or subtenant",
            )),
        ));
    }

    // Verify user exists
    let user_exists = state
        .user_repo
        .find_by_id(req.user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to check user");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?
        .is_some();

    if !user_exists {
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new("USER_NOT_FOUND", "User not found")),
        ));
    }

    // Create resident
    let create_data = CreateUnitResident {
        unit_id,
        user_id: req.user_id,
        resident_type: req.resident_type,
        is_primary: req.is_primary,
        start_date: req.start_date,
        receives_notifications: req.receives_notifications,
        receives_mail: req.receives_mail,
        notes: req.notes,
    };

    let resident = state
        .unit_resident_repo
        .create(create_data, auth.user_id)
        .await
        .map_err(|e| {
            // Check for duplicate key violation
            if e.to_string().contains("unique") || e.to_string().contains("duplicate") {
                return (
                    StatusCode::CONFLICT,
                    Json(ErrorResponse::new(
                        "ALREADY_RESIDENT",
                        "This user is already a resident of this unit",
                    )),
                );
            }
            tracing::error!(error = %e, "Failed to add resident");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to add resident")),
            )
        })?;

    tracing::info!(
        unit_id = %unit_id,
        resident_id = %resident.id,
        user_id = %req.user_id,
        by_user_id = %auth.user_id,
        "Resident added to unit"
    );

    rls.release().await;
    Ok((StatusCode::CREATED, Json(ResidentResponse::from(resident))))
}

/// Get resident by ID.
#[utoipa::path(
    get,
    path = "/api/v1/buildings/{building_id}/units/{unit_id}/residents/{resident_id}",
    tag = "Unit Residents",
    params(
        ("building_id" = Uuid, Path, description = "Building ID"),
        ("unit_id" = Uuid, Path, description = "Unit ID"),
        ("resident_id" = Uuid, Path, description = "Resident ID")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Resident found", body = ResidentResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Resident not found", body = ErrorResponse)
    )
)]
pub async fn get_resident(
    State(state): State<AppState>,
    auth: AuthUser,
    mut rls: RlsConnection,
    Path((building_id, unit_id, resident_id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // Verify building exists and user has access
    let building = state
        .building_repo
        .find_by_id_rls(&mut **rls.conn(), building_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get building");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Building not found")),
            )
        })?;

    // Validate user has access to the organization
    let is_member = state
        .org_member_repo
        .is_member(building.organization_id, auth.user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to check org membership");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?;

    if !is_member {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "NOT_AUTHORIZED",
                "You are not a member of this organization",
            )),
        ));
    }

    // Get resident
    let resident = state
        .unit_resident_repo
        .find_by_id(resident_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get resident");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Resident not found")),
            )
        })?;

    // Verify resident belongs to the unit
    if resident.unit_id != unit_id {
        rls.release().await;
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "NOT_FOUND",
                "Resident not found for this unit",
            )),
        ));
    }

    rls.release().await;
    Ok(Json(ResidentResponse::from(resident)))
}

/// Update a resident.
#[utoipa::path(
    put,
    path = "/api/v1/buildings/{building_id}/units/{unit_id}/residents/{resident_id}",
    tag = "Unit Residents",
    params(
        ("building_id" = Uuid, Path, description = "Building ID"),
        ("unit_id" = Uuid, Path, description = "Unit ID"),
        ("resident_id" = Uuid, Path, description = "Resident ID")
    ),
    request_body = UpdateResidentRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Resident updated", body = ResidentResponse),
        (status = 400, description = "Invalid input", body = ErrorResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Resident not found", body = ErrorResponse)
    )
)]
pub async fn update_resident(
    State(state): State<AppState>,
    auth: AuthUser,
    mut rls: RlsConnection,
    Path((building_id, unit_id, resident_id)): Path<(Uuid, Uuid, Uuid)>,
    Json(req): Json<UpdateResidentRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // Verify building exists and user has access
    let building = state
        .building_repo
        .find_by_id_rls(&mut **rls.conn(), building_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get building");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Building not found")),
            )
        })?;

    // Validate user has access to the organization
    let is_member = state
        .org_member_repo
        .is_member(building.organization_id, auth.user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to check org membership");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?;

    if !is_member {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "NOT_AUTHORIZED",
                "You are not a member of this organization",
            )),
        ));
    }

    // Verify resident exists and belongs to unit
    let existing = state
        .unit_resident_repo
        .find_by_id(resident_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get resident");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Resident not found")),
            )
        })?;

    if existing.unit_id != unit_id {
        rls.release().await;
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "NOT_FOUND",
                "Resident not found for this unit",
            )),
        ));
    }

    // Validate resident type if provided
    if let Some(ref rtype) = req.resident_type {
        let valid_types = [
            resident_type::OWNER,
            resident_type::TENANT,
            resident_type::FAMILY_MEMBER,
            resident_type::SUBTENANT,
        ];
        if !valid_types.contains(&rtype.as_str()) {
            rls.release().await;
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "INVALID_RESIDENT_TYPE",
                    "Invalid resident type. Must be: owner, tenant, family_member, or subtenant",
                )),
            ));
        }
    }

    let update_data = UpdateUnitResident {
        resident_type: req.resident_type,
        is_primary: req.is_primary,
        end_date: None,
        receives_notifications: req.receives_notifications,
        receives_mail: req.receives_mail,
        notes: req.notes,
    };

    let resident = state
        .unit_resident_repo
        .update(resident_id, update_data)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to update resident");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to update resident")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Resident not found")),
            )
        })?;

    tracing::info!(
        resident_id = %resident_id,
        by_user_id = %auth.user_id,
        "Resident updated"
    );

    rls.release().await;
    Ok(Json(ResidentResponse::from(resident)))
}

/// Remove a resident from unit (hard delete).
#[utoipa::path(
    delete,
    path = "/api/v1/buildings/{building_id}/units/{unit_id}/residents/{resident_id}",
    tag = "Unit Residents",
    params(
        ("building_id" = Uuid, Path, description = "Building ID"),
        ("unit_id" = Uuid, Path, description = "Unit ID"),
        ("resident_id" = Uuid, Path, description = "Resident ID")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Resident removed"),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Resident not found", body = ErrorResponse)
    )
)]
pub async fn remove_resident(
    State(state): State<AppState>,
    auth: AuthUser,
    mut rls: RlsConnection,
    Path((building_id, unit_id, resident_id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // Verify building exists and user has access
    let building = state
        .building_repo
        .find_by_id_rls(&mut **rls.conn(), building_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get building");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Building not found")),
            )
        })?;

    // Validate user has access to the organization
    let is_member = state
        .org_member_repo
        .is_member(building.organization_id, auth.user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to check org membership");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?;

    if !is_member {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "NOT_AUTHORIZED",
                "You are not a member of this organization",
            )),
        ));
    }

    // Verify resident exists and belongs to unit
    let existing = state
        .unit_resident_repo
        .find_by_id(resident_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get resident");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?;

    if let Some(r) = existing {
        if r.unit_id != unit_id {
            rls.release().await;
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    "Resident not found for this unit",
                )),
            ));
        }
    }

    let deleted = state
        .unit_resident_repo
        .delete(resident_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to remove resident");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to remove resident")),
            )
        })?;

    rls.release().await;
    if deleted {
        tracing::info!(
            resident_id = %resident_id,
            by_user_id = %auth.user_id,
            "Resident removed from unit"
        );
        Ok(StatusCode::OK)
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Resident not found")),
        ))
    }
}

/// End a residency (soft delete by setting end_date).
#[utoipa::path(
    post,
    path = "/api/v1/buildings/{building_id}/units/{unit_id}/residents/{resident_id}/end",
    tag = "Unit Residents",
    params(
        ("building_id" = Uuid, Path, description = "Building ID"),
        ("unit_id" = Uuid, Path, description = "Unit ID"),
        ("resident_id" = Uuid, Path, description = "Resident ID")
    ),
    request_body = EndResidencyRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Residency ended", body = ResidentResponse),
        (status = 400, description = "Invalid input", body = ErrorResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Resident not found", body = ErrorResponse)
    )
)]
pub async fn end_residency(
    State(state): State<AppState>,
    auth: AuthUser,
    mut rls: RlsConnection,
    Path((building_id, unit_id, resident_id)): Path<(Uuid, Uuid, Uuid)>,
    Json(req): Json<EndResidencyRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // Verify building exists and user has access
    let building = state
        .building_repo
        .find_by_id_rls(&mut **rls.conn(), building_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get building");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Building not found")),
            )
        })?;

    // Validate user has access to the organization
    let is_member = state
        .org_member_repo
        .is_member(building.organization_id, auth.user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to check org membership");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?;

    if !is_member {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "NOT_AUTHORIZED",
                "You are not a member of this organization",
            )),
        ));
    }

    // Verify resident exists and belongs to unit
    let existing = state
        .unit_resident_repo
        .find_by_id(resident_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get resident");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Resident not found")),
            )
        })?;

    if existing.unit_id != unit_id {
        rls.release().await;
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "NOT_FOUND",
                "Resident not found for this unit",
            )),
        ));
    }

    // Validate end date is after start date
    if req.end_date < existing.start_date {
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_END_DATE",
                "End date must be after start date",
            )),
        ));
    }

    let resident = state
        .unit_resident_repo
        .end_residency(resident_id, req.end_date)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to end residency");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to end residency")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Resident not found")),
            )
        })?;

    tracing::info!(
        resident_id = %resident_id,
        end_date = %req.end_date,
        by_user_id = %auth.user_id,
        "Residency ended"
    );

    rls.release().await;
    Ok(Json(ResidentResponse::from(resident)))
}

/// List resident history for a unit (including ended residencies).
#[utoipa::path(
    get,
    path = "/api/v1/buildings/{building_id}/units/{unit_id}/residents/history",
    tag = "Unit Residents",
    params(
        ("building_id" = Uuid, Path, description = "Building ID"),
        ("unit_id" = Uuid, Path, description = "Unit ID")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Resident history", body = Vec<ResidentWithUserResponse>),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Unit not found", body = ErrorResponse)
    )
)]
pub async fn list_resident_history(
    State(state): State<AppState>,
    auth: AuthUser,
    mut rls: RlsConnection,
    Path((building_id, unit_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // Verify building exists and user has access
    let building = state
        .building_repo
        .find_by_id_rls(&mut **rls.conn(), building_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get building");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Building not found")),
            )
        })?;

    // Validate user has access to the organization
    let is_member = state
        .org_member_repo
        .is_member(building.organization_id, auth.user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to check org membership");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?;

    if !is_member {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "NOT_AUTHORIZED",
                "You are not a member of this organization",
            )),
        ));
    }

    // Verify unit exists and belongs to building
    let unit = state
        .unit_repo
        .find_by_id_rls(&mut **rls.conn(), unit_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get unit");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?;

    let unit = unit.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Unit not found")),
        )
    })?;

    if unit.building_id != building_id {
        rls.release().await;
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "NOT_FOUND",
                "Unit not found in this building",
            )),
        ));
    }

    // Get all residents including ended
    let residents = state
        .unit_resident_repo
        .find_by_unit_all(unit_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get resident history");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DB_ERROR",
                    "Failed to get resident history",
                )),
            )
        })?;

    let response: Vec<ResidentWithUserResponse> = residents.into_iter().map(Into::into).collect();

    rls.release().await;
    Ok(Json(response))
}
