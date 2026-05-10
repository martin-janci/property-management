//! Person month routes (Epic 3, Story 3.5).
//!
//! Implements person-month tracking for utility billing calculations.
//! Person-months represent the number of residents in a unit for each month,
//! used for calculating shared utility costs.

use api_core::extractors::{AuthUser, RlsConnection};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};
use common::errors::ErrorResponse;
use db::models::person_month::{
    person_month_source, BulkPersonMonthEntry, CreatePersonMonth, PersonMonth, PersonMonthWithUnit,
    UpdatePersonMonth, YearlyPersonMonthSummary,
};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::state::AppState;

/// Create person months router.
pub fn router() -> Router<AppState> {
    Router::new()
        // Unit-level person months (Story 3.5)
        .route("/", get(get_unit_person_months))
        .route("/", post(upsert_person_month))
        .route("/{id}", get(get_person_month))
        .route("/{id}", put(update_person_month))
        .route("/{id}", delete(delete_person_month))
        .route("/yearly", get(get_yearly_summary))
        .route("/calculate", post(calculate_from_residents))
}

/// Create building-level person months router.
pub fn building_router() -> Router<AppState> {
    Router::new()
        // Building-level person months (Story 3.5)
        .route("/", get(list_building_person_months))
        .route("/bulk", post(bulk_upsert_person_months))
        .route("/summary", get(get_building_summary))
}

// ==================== Request/Response Types ====================

/// Query parameters for getting unit person months.
#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct GetPersonMonthsQuery {
    /// Year to filter by
    pub year: i32,
    /// Month to filter by (optional, returns all months if not specified)
    pub month: Option<i32>,
}

/// Upsert person month request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpsertPersonMonthRequest {
    /// Year
    pub year: i32,
    /// Month (1-12)
    pub month: i32,
    /// Number of person-months
    pub count: i32,
    /// Source of the data (manual, calculated, imported)
    #[serde(default = "default_source")]
    pub source: String,
    /// Notes
    pub notes: Option<String>,
}

fn default_source() -> String {
    "manual".to_string()
}

/// Update person month request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdatePersonMonthRequest {
    /// Number of person-months
    pub count: Option<i32>,
    /// Source of the data
    pub source: Option<String>,
    /// Notes
    pub notes: Option<String>,
}

/// Bulk upsert request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct BulkUpsertRequest {
    /// Year
    pub year: i32,
    /// Month (1-12)
    pub month: i32,
    /// Entries for each unit
    pub entries: Vec<BulkEntry>,
}

/// Entry for bulk upsert.
#[derive(Debug, Deserialize, ToSchema)]
pub struct BulkEntry {
    /// Unit ID
    pub unit_id: Uuid,
    /// Number of person-months
    pub count: i32,
}

/// Query parameters for building-level operations.
#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct BuildingPersonMonthsQuery {
    /// Year
    pub year: i32,
    /// Month (1-12)
    pub month: i32,
}

/// Person month response.
#[derive(Debug, Serialize, ToSchema)]
pub struct PersonMonthResponse {
    pub id: Uuid,
    pub unit_id: Uuid,
    pub year: i32,
    pub month: i32,
    pub count: i32,
    pub source: String,
    pub source_display: String,
    pub period: String,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<PersonMonth> for PersonMonthResponse {
    fn from(pm: PersonMonth) -> Self {
        Self {
            id: pm.id,
            unit_id: pm.unit_id,
            year: pm.year,
            month: pm.month,
            count: pm.count,
            source: pm.source.clone(),
            source_display: pm.source_display().to_string(),
            period: pm.period_string(),
            notes: pm.notes,
            created_at: pm.created_at.to_rfc3339(),
            updated_at: pm.updated_at.to_rfc3339(),
        }
    }
}

/// Person month with unit info response.
#[derive(Debug, Serialize, ToSchema)]
pub struct PersonMonthWithUnitResponse {
    pub id: Uuid,
    pub unit_id: Uuid,
    pub unit_designation: String,
    pub year: i32,
    pub month: i32,
    pub count: i32,
    pub source: String,
}

impl From<PersonMonthWithUnit> for PersonMonthWithUnitResponse {
    fn from(pm: PersonMonthWithUnit) -> Self {
        Self {
            id: pm.id,
            unit_id: pm.unit_id,
            unit_designation: pm.unit_designation,
            year: pm.year,
            month: pm.month,
            count: pm.count,
            source: pm.source,
        }
    }
}

/// Bulk upsert result.
#[derive(Debug, Serialize, ToSchema)]
pub struct BulkUpsertResult {
    /// Number of successful upserts
    pub successful: usize,
    /// Number of failed upserts
    pub failed: usize,
    /// Results for each entry
    pub results: Vec<BulkUpsertEntryResult>,
}

/// Result for a single bulk upsert entry.
#[derive(Debug, Serialize, ToSchema)]
pub struct BulkUpsertEntryResult {
    pub unit_id: Uuid,
    pub success: bool,
    pub person_month_id: Option<Uuid>,
    pub error: Option<String>,
}

/// Building summary response.
#[derive(Debug, Serialize, ToSchema)]
pub struct BuildingSummaryResponse {
    pub building_id: Uuid,
    pub year: i32,
    pub month: i32,
    pub total_count: i64,
    pub unit_count: i64,
}

// ==================== Unit-Level Handlers ====================

/// Get person months for a unit.
#[utoipa::path(
    get,
    path = "/api/v1/buildings/{building_id}/units/{unit_id}/person-months",
    tag = "Person Months",
    params(
        ("building_id" = Uuid, Path, description = "Building ID"),
        ("unit_id" = Uuid, Path, description = "Unit ID"),
        GetPersonMonthsQuery
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Person months for unit", body = Vec<PersonMonthResponse>),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Unit not found", body = ErrorResponse)
    )
)]
pub async fn get_unit_person_months(
    State(state): State<AppState>,
    auth: AuthUser,
    mut rls: RlsConnection,
    Path((building_id, unit_id)): Path<(Uuid, Uuid)>,
    Query(query): Query<GetPersonMonthsQuery>,
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
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "NOT_AUTHORIZED",
                "You are not a member of this organization",
            )),
        ));
    }

    // Verify unit belongs to building
    let belongs = state
        .unit_repo
        .belongs_to_building(unit_id, building_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to check unit building");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?;

    if !belongs {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "NOT_FOUND",
                "Unit not found in this building",
            )),
        ));
    }

    // Get person months
    let entries = if let Some(month) = query.month {
        // Get specific month
        let entry = state
            .person_month_repo
            .find_by_unit_period(unit_id, query.year, month)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to get person month");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new("DB_ERROR", "Database error")),
                )
            })?;
        entry.into_iter().collect()
    } else {
        // Get all months for the year
        state
            .person_month_repo
            .find_by_unit_year(unit_id, query.year)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to get person months");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new("DB_ERROR", "Database error")),
                )
            })?
    };

    let response: Vec<PersonMonthResponse> = entries.into_iter().map(Into::into).collect();

    Ok(Json(response))
}

/// Create or update a person month entry.
#[utoipa::path(
    post,
    path = "/api/v1/buildings/{building_id}/units/{unit_id}/person-months",
    tag = "Person Months",
    params(
        ("building_id" = Uuid, Path, description = "Building ID"),
        ("unit_id" = Uuid, Path, description = "Unit ID")
    ),
    request_body = UpsertPersonMonthRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Person month created/updated", body = PersonMonthResponse),
        (status = 400, description = "Invalid input", body = ErrorResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Unit not found", body = ErrorResponse)
    )
)]
pub async fn upsert_person_month(
    State(state): State<AppState>,
    auth: AuthUser,
    mut rls: RlsConnection,
    Path((building_id, unit_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpsertPersonMonthRequest>,
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
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "NOT_AUTHORIZED",
                "You are not a member of this organization",
            )),
        ));
    }

    // Verify unit belongs to building
    let belongs = state
        .unit_repo
        .belongs_to_building(unit_id, building_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to check unit building");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?;

    if !belongs {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "NOT_FOUND",
                "Unit not found in this building",
            )),
        ));
    }

    // Validate month
    if req.month < 1 || req.month > 12 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_MONTH",
                "Month must be between 1 and 12",
            )),
        ));
    }

    // Validate count
    if req.count < 0 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_COUNT",
                "Count cannot be negative",
            )),
        ));
    }

    // Validate source
    let valid_sources = [
        person_month_source::MANUAL,
        person_month_source::CALCULATED,
        person_month_source::IMPORTED,
    ];
    if !valid_sources.contains(&req.source.as_str()) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_SOURCE",
                "Source must be: manual, calculated, or imported",
            )),
        ));
    }

    let data = CreatePersonMonth {
        unit_id,
        year: req.year,
        month: req.month,
        count: req.count,
        source: req.source,
        notes: req.notes,
    };

    let entry = state
        .person_month_repo
        .upsert(data, auth.user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to upsert person month");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DB_ERROR",
                    "Failed to save person month",
                )),
            )
        })?;

    tracing::info!(
        unit_id = %unit_id,
        year = req.year,
        month = req.month,
        count = req.count,
        by_user_id = %auth.user_id,
        "Person month upserted"
    );

    Ok(Json(PersonMonthResponse::from(entry)))
}

/// Get a specific person month by ID.
#[utoipa::path(
    get,
    path = "/api/v1/buildings/{building_id}/units/{unit_id}/person-months/{id}",
    tag = "Person Months",
    params(
        ("building_id" = Uuid, Path, description = "Building ID"),
        ("unit_id" = Uuid, Path, description = "Unit ID"),
        ("id" = Uuid, Path, description = "Person month ID")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Person month found", body = PersonMonthResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Person month not found", body = ErrorResponse)
    )
)]
pub async fn get_person_month(
    State(state): State<AppState>,
    auth: AuthUser,
    mut rls: RlsConnection,
    Path((building_id, unit_id, id)): Path<(Uuid, Uuid, Uuid)>,
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
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "NOT_AUTHORIZED",
                "You are not a member of this organization",
            )),
        ));
    }

    let entry = state
        .person_month_repo
        .find_by_id(id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get person month");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Person month not found")),
            )
        })?;

    // Verify it belongs to the unit
    if entry.unit_id != unit_id {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "NOT_FOUND",
                "Person month not found for this unit",
            )),
        ));
    }

    Ok(Json(PersonMonthResponse::from(entry)))
}

/// Update a person month.
#[utoipa::path(
    put,
    path = "/api/v1/buildings/{building_id}/units/{unit_id}/person-months/{id}",
    tag = "Person Months",
    params(
        ("building_id" = Uuid, Path, description = "Building ID"),
        ("unit_id" = Uuid, Path, description = "Unit ID"),
        ("id" = Uuid, Path, description = "Person month ID")
    ),
    request_body = UpdatePersonMonthRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Person month updated", body = PersonMonthResponse),
        (status = 400, description = "Invalid input", body = ErrorResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Person month not found", body = ErrorResponse)
    )
)]
pub async fn update_person_month(
    State(state): State<AppState>,
    auth: AuthUser,
    mut rls: RlsConnection,
    Path((building_id, unit_id, id)): Path<(Uuid, Uuid, Uuid)>,
    Json(req): Json<UpdatePersonMonthRequest>,
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
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "NOT_AUTHORIZED",
                "You are not a member of this organization",
            )),
        ));
    }

    // Verify entry exists and belongs to unit
    let existing = state
        .person_month_repo
        .find_by_id(id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get person month");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Person month not found")),
            )
        })?;

    if existing.unit_id != unit_id {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "NOT_FOUND",
                "Person month not found for this unit",
            )),
        ));
    }

    // Validate count if provided
    if let Some(count) = req.count {
        if count < 0 {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "INVALID_COUNT",
                    "Count cannot be negative",
                )),
            ));
        }
    }

    // Validate source if provided
    if let Some(ref source) = req.source {
        let valid_sources = [
            person_month_source::MANUAL,
            person_month_source::CALCULATED,
            person_month_source::IMPORTED,
        ];
        if !valid_sources.contains(&source.as_str()) {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "INVALID_SOURCE",
                    "Source must be: manual, calculated, or imported",
                )),
            ));
        }
    }

    let data = UpdatePersonMonth {
        count: req.count,
        source: req.source,
        notes: req.notes,
    };

    let entry = state
        .person_month_repo
        .update(id, data, auth.user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to update person month");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DB_ERROR",
                    "Failed to update person month",
                )),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Person month not found")),
            )
        })?;

    tracing::info!(
        id = %id,
        by_user_id = %auth.user_id,
        "Person month updated"
    );

    Ok(Json(PersonMonthResponse::from(entry)))
}

/// Delete a person month entry.
#[utoipa::path(
    delete,
    path = "/api/v1/buildings/{building_id}/units/{unit_id}/person-months/{id}",
    tag = "Person Months",
    params(
        ("building_id" = Uuid, Path, description = "Building ID"),
        ("unit_id" = Uuid, Path, description = "Unit ID"),
        ("id" = Uuid, Path, description = "Person month ID")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Person month deleted"),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Person month not found", body = ErrorResponse)
    )
)]
pub async fn delete_person_month(
    State(state): State<AppState>,
    auth: AuthUser,
    mut rls: RlsConnection,
    Path((building_id, unit_id, id)): Path<(Uuid, Uuid, Uuid)>,
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
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "NOT_AUTHORIZED",
                "You are not a member of this organization",
            )),
        ));
    }

    // Verify entry exists and belongs to unit
    let existing = state.person_month_repo.find_by_id(id).await.map_err(|e| {
        tracing::error!(error = %e, "Failed to get person month");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("DB_ERROR", "Database error")),
        )
    })?;

    if let Some(e) = existing {
        if e.unit_id != unit_id {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    "Person month not found for this unit",
                )),
            ));
        }
    }

    let deleted = state.person_month_repo.delete(id).await.map_err(|e| {
        tracing::error!(error = %e, "Failed to delete person month");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "DB_ERROR",
                "Failed to delete person month",
            )),
        )
    })?;

    if deleted {
        tracing::info!(
            id = %id,
            by_user_id = %auth.user_id,
            "Person month deleted"
        );
        Ok(StatusCode::OK)
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Person month not found")),
        ))
    }
}

/// Get yearly summary for a unit.
#[utoipa::path(
    get,
    path = "/api/v1/buildings/{building_id}/units/{unit_id}/person-months/yearly",
    tag = "Person Months",
    params(
        ("building_id" = Uuid, Path, description = "Building ID"),
        ("unit_id" = Uuid, Path, description = "Unit ID"),
        ("year" = i32, Query, description = "Year")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Yearly summary", body = YearlyPersonMonthSummary),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Unit not found", body = ErrorResponse)
    )
)]
pub async fn get_yearly_summary(
    State(state): State<AppState>,
    auth: AuthUser,
    mut rls: RlsConnection,
    Path((building_id, unit_id)): Path<(Uuid, Uuid)>,
    Query(query): Query<GetPersonMonthsQuery>,
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
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "NOT_AUTHORIZED",
                "You are not a member of this organization",
            )),
        ));
    }

    // Verify unit belongs to building
    let belongs = state
        .unit_repo
        .belongs_to_building(unit_id, building_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to check unit building");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?;

    if !belongs {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "NOT_FOUND",
                "Unit not found in this building",
            )),
        ));
    }

    let summary = state
        .person_month_repo
        .get_yearly_summary(unit_id, query.year)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get yearly summary");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?;

    Ok(Json(summary))
}

/// Calculate person-months from residents for a period.
#[utoipa::path(
    post,
    path = "/api/v1/buildings/{building_id}/units/{unit_id}/person-months/calculate",
    tag = "Person Months",
    params(
        ("building_id" = Uuid, Path, description = "Building ID"),
        ("unit_id" = Uuid, Path, description = "Unit ID")
    ),
    request_body = GetPersonMonthsQuery,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Calculated count", body = i32),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Unit not found", body = ErrorResponse)
    )
)]
pub async fn calculate_from_residents(
    State(state): State<AppState>,
    auth: AuthUser,
    mut rls: RlsConnection,
    Path((building_id, unit_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<GetPersonMonthsQuery>,
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
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "NOT_AUTHORIZED",
                "You are not a member of this organization",
            )),
        ));
    }

    // Verify unit belongs to building
    let belongs = state
        .unit_repo
        .belongs_to_building(unit_id, building_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to check unit building");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?;

    if !belongs {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "NOT_FOUND",
                "Unit not found in this building",
            )),
        ));
    }

    let month = req.month.unwrap_or(1);

    let count = state
        .person_month_repo
        .calculate_from_residents(unit_id, req.year, month)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to calculate from residents");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?;

    Ok(Json(count))
}

// ==================== Building-Level Handlers ====================

/// List person months for a building for a specific month.
#[utoipa::path(
    get,
    path = "/api/v1/buildings/{building_id}/person-months",
    tag = "Person Months",
    params(
        ("building_id" = Uuid, Path, description = "Building ID"),
        BuildingPersonMonthsQuery
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Person months for building", body = Vec<PersonMonthWithUnitResponse>),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Building not found", body = ErrorResponse)
    )
)]
pub async fn list_building_person_months(
    State(state): State<AppState>,
    auth: AuthUser,
    mut rls: RlsConnection,
    Path(building_id): Path<Uuid>,
    Query(query): Query<BuildingPersonMonthsQuery>,
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
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "NOT_AUTHORIZED",
                "You are not a member of this organization",
            )),
        ));
    }

    let entries = state
        .person_month_repo
        .find_by_building_period(building_id, query.year, query.month)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get building person months");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?;

    let response: Vec<PersonMonthWithUnitResponse> = entries.into_iter().map(Into::into).collect();

    Ok(Json(response))
}

/// Bulk upsert person months for a building.
#[utoipa::path(
    post,
    path = "/api/v1/buildings/{building_id}/person-months/bulk",
    tag = "Person Months",
    params(("building_id" = Uuid, Path, description = "Building ID")),
    request_body = BulkUpsertRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Bulk upsert completed", body = BulkUpsertResult),
        (status = 400, description = "Invalid input", body = ErrorResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Building not found", body = ErrorResponse)
    )
)]
pub async fn bulk_upsert_person_months(
    State(state): State<AppState>,
    auth: AuthUser,
    mut rls: RlsConnection,
    Path(building_id): Path<Uuid>,
    Json(req): Json<BulkUpsertRequest>,
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
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "NOT_AUTHORIZED",
                "You are not a member of this organization",
            )),
        ));
    }

    // Validate month
    if req.month < 1 || req.month > 12 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_MONTH",
                "Month must be between 1 and 12",
            )),
        ));
    }

    let entries: Vec<BulkPersonMonthEntry> = req
        .entries
        .into_iter()
        .map(|e| BulkPersonMonthEntry {
            unit_id: e.unit_id,
            count: e.count,
        })
        .collect();

    let results = state
        .person_month_repo
        .bulk_upsert(req.year, req.month, entries, auth.user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to bulk upsert person months");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to bulk upsert")),
            )
        })?;

    let entry_results: Vec<BulkUpsertEntryResult> = results
        .iter()
        .map(|pm| BulkUpsertEntryResult {
            unit_id: pm.unit_id,
            success: true,
            person_month_id: Some(pm.id),
            error: None,
        })
        .collect();

    let successful = entry_results.len();

    tracing::info!(
        building_id = %building_id,
        year = req.year,
        month = req.month,
        count = successful,
        by_user_id = %auth.user_id,
        "Bulk person months upserted"
    );

    Ok(Json(BulkUpsertResult {
        successful,
        failed: 0,
        results: entry_results,
    }))
}

/// Get building-level summary for a month.
#[utoipa::path(
    get,
    path = "/api/v1/buildings/{building_id}/person-months/summary",
    tag = "Person Months",
    params(
        ("building_id" = Uuid, Path, description = "Building ID"),
        BuildingPersonMonthsQuery
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Building summary", body = BuildingSummaryResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Building not found", body = ErrorResponse)
    )
)]
pub async fn get_building_summary(
    State(state): State<AppState>,
    auth: AuthUser,
    mut rls: RlsConnection,
    Path(building_id): Path<Uuid>,
    Query(query): Query<BuildingPersonMonthsQuery>,
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
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "NOT_AUTHORIZED",
                "You are not a member of this organization",
            )),
        ));
    }

    let summary = state
        .person_month_repo
        .get_building_summary(building_id, query.year, query.month)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get building summary");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?;

    match summary {
        Some(s) => Ok(Json(BuildingSummaryResponse {
            building_id: s.building_id,
            year: s.year,
            month: s.month,
            total_count: s.total_count,
            unit_count: s.unit_count,
        })),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "No data found")),
        )),
    }
}
