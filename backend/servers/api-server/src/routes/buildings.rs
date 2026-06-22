//! Building routes (UC-15, Epic 2B).
//!
//! Implements building and unit management including CRUD operations,
//! unit assignments, and statistics.

use api_core::extractors::RlsConnection;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};
use common::errors::ErrorResponse;
use common::TenantRole;
use db::models::{
    Building, BuildingStatistics, BuildingSummary, CreateBuilding, CreateUnit, Unit, UnitOwnerInfo,
    UnitSummary, UnitWithOwners, UpdateBuilding, UpdateUnit,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::IntoParams;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::state::AppState;

/// Default page size for building listing
const DEFAULT_LIST_LIMIT: i64 = 50;

/// Maximum page size
const MAX_LIST_LIMIT: i64 = 100;

/// Maximum buildings per bulk import
const MAX_BULK_IMPORT_SIZE: usize = 100;

/// Create buildings router.
pub fn router() -> Router<AppState> {
    Router::new()
        // Building CRUD (UC-15.1-3, 15.10)
        .route("/", post(create_building))
        .route("/", get(list_buildings))
        .route("/bulk", post(bulk_import_buildings))
        .route("/{id}", get(get_building))
        .route("/{id}", put(update_building))
        .route("/{id}", delete(archive_building))
        .route("/{id}/restore", post(restore_building))
        // Building statistics (UC-15.7)
        .route("/{id}/statistics", get(get_building_statistics))
        // Unit management (UC-15.4-5)
        .route("/{id}/units", get(list_units))
        .route("/{id}/units", post(create_unit))
        .route("/{building_id}/units/{unit_id}", get(get_unit))
        .route("/{building_id}/units/{unit_id}", put(update_unit))
        .route("/{building_id}/units/{unit_id}", delete(archive_unit))
        .route("/{building_id}/units/{unit_id}/restore", post(restore_unit))
        // Unit owner management (UC-15.6)
        .route(
            "/{building_id}/units/{unit_id}/owners",
            get(list_unit_owners),
        )
        .route(
            "/{building_id}/units/{unit_id}/owners",
            post(assign_unit_owner),
        )
        .route(
            "/{building_id}/units/{unit_id}/owners/{user_id}",
            put(update_unit_owner),
        )
        .route(
            "/{building_id}/units/{unit_id}/owners/{user_id}",
            delete(remove_unit_owner),
        )
        // Unit residents (Epic 3, Story 3.3)
        .nest(
            "/{building_id}/units/{unit_id}/residents",
            super::unit_residents::router(),
        )
        // Person months (Epic 3, Story 3.5) - unit level
        .nest(
            "/{building_id}/units/{unit_id}/person-months",
            super::person_months::router(),
        )
        // Person months (Epic 3, Story 3.5) - building level
        .nest(
            "/{building_id}/person-months",
            super::person_months::building_router(),
        )
}

// ==================== Request/Response Types ====================

/// Create building request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateBuildingRequest {
    /// Organization ID the building belongs to
    pub organization_id: Uuid,
    /// Street address
    pub street: String,
    /// City
    pub city: String,
    /// Postal code
    pub postal_code: String,
    /// Country (defaults to Slovakia)
    #[serde(default = "default_country")]
    pub country: String,
    /// Building name (optional)
    pub name: Option<String>,
    /// Description (optional)
    pub description: Option<String>,
    /// Year the building was constructed
    pub year_built: Option<i32>,
    /// Total number of floors
    #[serde(default = "default_one")]
    pub total_floors: i32,
    /// Total number of entrances
    #[serde(default = "default_one")]
    pub total_entrances: i32,
    /// Amenities list
    #[serde(default)]
    pub amenities: Vec<String>,
}

fn default_country() -> String {
    "Slovakia".to_string()
}

fn default_one() -> i32 {
    1
}

/// Building response.
#[derive(Debug, Serialize, ToSchema)]
pub struct BuildingResponse {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub street: String,
    pub city: String,
    pub postal_code: String,
    pub country: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub year_built: Option<i32>,
    pub total_floors: i32,
    pub total_entrances: i32,
    pub amenities: Vec<String>,
    /// Geocoded latitude (Story 3.1 AC3); null when unresolved.
    pub latitude: Option<Decimal>,
    /// Geocoded longitude (Story 3.1 AC3); null when unresolved.
    pub longitude: Option<Decimal>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<Building> for BuildingResponse {
    fn from(b: Building) -> Self {
        let amenities = b.amenity_list();
        Self {
            id: b.id,
            organization_id: b.organization_id,
            street: b.street,
            city: b.city,
            postal_code: b.postal_code,
            country: b.country,
            name: b.name,
            description: b.description,
            year_built: b.year_built,
            total_floors: b.total_floors,
            total_entrances: b.total_entrances,
            amenities,
            latitude: b.latitude,
            longitude: b.longitude,
            status: b.status,
            created_at: b.created_at.to_rfc3339(),
            updated_at: b.updated_at.to_rfc3339(),
        }
    }
}

/// Update building request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateBuildingRequest {
    pub street: Option<String>,
    pub city: Option<String>,
    pub postal_code: Option<String>,
    pub country: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub year_built: Option<i32>,
    pub total_floors: Option<i32>,
    pub total_entrances: Option<i32>,
    pub amenities: Option<Vec<String>>,
}

/// List buildings query parameters.
#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct ListBuildingsQuery {
    /// Organization ID to filter by
    pub organization_id: Uuid,
    /// Page offset
    #[serde(default)]
    pub offset: i64,
    /// Page size (max 100)
    pub limit: Option<i64>,
    /// Include archived buildings
    #[serde(default)]
    pub include_archived: bool,
    /// Search term
    pub search: Option<String>,
}

/// Paginated buildings response.
#[derive(Debug, Serialize, ToSchema)]
pub struct BuildingsListResponse {
    pub buildings: Vec<BuildingSummary>,
    pub total: i64,
    pub offset: i64,
    pub limit: i64,
}

/// Create unit request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateUnitRequest {
    /// Unit designation (e.g., "3B", "101")
    pub designation: String,
    /// Entrance (optional, for multi-entrance buildings)
    pub entrance: Option<String>,
    /// Floor number (0 = ground, negative = basement)
    #[serde(default)]
    pub floor: i32,
    /// Unit type
    #[serde(default = "default_unit_type")]
    pub unit_type: String,
    /// Size in square meters
    pub size_sqm: Option<Decimal>,
    /// Number of rooms
    pub rooms: Option<i32>,
    /// Ownership share percentage (0-100)
    #[serde(default = "default_ownership_share")]
    pub ownership_share: Decimal,
    /// Description
    pub description: Option<String>,
}

fn default_unit_type() -> String {
    "apartment".to_string()
}

fn default_ownership_share() -> Decimal {
    Decimal::new(10000, 2) // 100.00
}

/// Unit response.
#[derive(Debug, Serialize, ToSchema)]
pub struct UnitResponse {
    pub id: Uuid,
    pub building_id: Uuid,
    pub entrance: Option<String>,
    pub designation: String,
    pub floor: i32,
    pub floor_display: String,
    pub unit_type: String,
    pub unit_type_display: String,
    pub size_sqm: Option<Decimal>,
    pub rooms: Option<i32>,
    pub ownership_share: Decimal,
    pub occupancy_status: String,
    pub description: Option<String>,
    pub notes: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<Unit> for UnitResponse {
    fn from(u: Unit) -> Self {
        Self {
            id: u.id,
            building_id: u.building_id,
            entrance: u.entrance.clone(),
            designation: u.designation.clone(),
            floor: u.floor,
            floor_display: u.floor_display(),
            unit_type: u.unit_type.clone(),
            unit_type_display: u.unit_type_display().to_string(),
            size_sqm: u.size_sqm,
            rooms: u.rooms,
            ownership_share: u.ownership_share,
            occupancy_status: u.occupancy_status,
            description: u.description,
            notes: u.notes,
            status: u.status,
            created_at: u.created_at.to_rfc3339(),
            updated_at: u.updated_at.to_rfc3339(),
        }
    }
}

/// Unit with owners response.
#[derive(Debug, Serialize, ToSchema)]
pub struct UnitWithOwnersResponse {
    #[serde(flatten)]
    pub unit: UnitResponse,
    pub owners: Vec<UnitOwnerInfo>,
}

impl From<UnitWithOwners> for UnitWithOwnersResponse {
    fn from(uw: UnitWithOwners) -> Self {
        Self {
            unit: uw.unit.into(),
            owners: uw.owners,
        }
    }
}

/// Update unit request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateUnitRequest {
    pub entrance: Option<String>,
    pub designation: Option<String>,
    pub floor: Option<i32>,
    pub unit_type: Option<String>,
    pub size_sqm: Option<Decimal>,
    pub rooms: Option<i32>,
    pub ownership_share: Option<Decimal>,
    pub occupancy_status: Option<String>,
    pub description: Option<String>,
    pub notes: Option<String>,
}

/// List units query parameters.
#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct ListUnitsQuery {
    /// Page offset
    #[serde(default)]
    pub offset: i64,
    /// Page size (max 100)
    pub limit: Option<i64>,
    /// Include archived units
    #[serde(default)]
    pub include_archived: bool,
    /// Filter by unit type
    pub unit_type: Option<String>,
    /// Filter by floor
    pub floor: Option<i32>,
}

/// Paginated units response.
#[derive(Debug, Serialize, ToSchema)]
pub struct UnitsListResponse {
    pub units: Vec<UnitSummary>,
    pub total: i64,
    pub offset: i64,
    pub limit: i64,
}

/// Assign unit owner request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct AssignOwnerRequest {
    /// User ID to assign as owner
    pub user_id: Uuid,
    /// Ownership percentage (0-100)
    #[serde(default = "default_ownership_share")]
    pub ownership_percentage: Decimal,
    /// Is this the primary contact for the unit?
    #[serde(default = "default_true")]
    pub is_primary: bool,
    /// When ownership starts (defaults to today)
    pub valid_from: Option<chrono::NaiveDate>,
}

fn default_true() -> bool {
    true
}

/// Update unit owner request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateOwnerRequest {
    pub ownership_percentage: Option<Decimal>,
    pub is_primary: Option<bool>,
}

/// Bulk import building entry.
#[derive(Debug, Deserialize, ToSchema)]
pub struct BulkBuildingEntry {
    /// Street address
    pub street: String,
    /// City
    pub city: String,
    /// Postal code
    pub postal_code: String,
    /// Country (defaults to Slovakia)
    #[serde(default = "default_country")]
    pub country: String,
    /// Building name (optional)
    pub name: Option<String>,
    /// Description (optional)
    pub description: Option<String>,
    /// Year the building was constructed
    pub year_built: Option<i32>,
    /// Total number of floors
    #[serde(default = "default_one")]
    pub total_floors: i32,
    /// Total number of entrances
    #[serde(default = "default_one")]
    pub total_entrances: i32,
    /// Amenities list
    #[serde(default)]
    pub amenities: Vec<String>,
}

/// Bulk import buildings request (UC-15.8).
#[derive(Debug, Deserialize, ToSchema)]
pub struct BulkImportBuildingsRequest {
    /// Organization ID for all buildings
    pub organization_id: Uuid,
    /// List of buildings to import
    pub buildings: Vec<BulkBuildingEntry>,
}

/// Bulk import result for a single building.
#[derive(Debug, Serialize, ToSchema)]
pub struct BulkImportResult {
    /// Index in the original request
    pub index: usize,
    /// Success or failure
    pub success: bool,
    /// Building ID if successful
    pub building_id: Option<Uuid>,
    /// Error message if failed
    pub error: Option<String>,
}

/// Bulk import buildings response.
#[derive(Debug, Serialize, ToSchema)]
pub struct BulkImportBuildingsResponse {
    /// Total buildings in request
    pub total: usize,
    /// Number of successful imports
    pub successful: usize,
    /// Number of failed imports
    pub failed: usize,
    /// Individual results
    pub results: Vec<BulkImportResult>,
}

/// Geocode an address on a building write path, never failing the request.
///
/// Returns `(None, None)` when geocoding is disabled, finds no match, or errors,
/// so a missing/flaky provider degrades gracefully (Story 3.1 AC3).
async fn geocode_address(state: &AppState, address: &str) -> (Option<Decimal>, Option<Decimal>) {
    match state.geocoding.geocode(address).await {
        Ok(Some(c)) => (Some(c.latitude), Some(c.longitude)),
        Ok(None) => (None, None),
        Err(e) => {
            tracing::warn!(error = %e, "Geocoding failed; storing building without coordinates");
            (None, None)
        }
    }
}

// ==================== Building Handlers ====================

/// Create a new building (UC-15.1).
#[utoipa::path(
    post,
    path = "/api/v1/buildings",
    tag = "Buildings",
    request_body = CreateBuildingRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 201, description = "Building created", body = BuildingResponse),
        (status = 400, description = "Invalid input", body = ErrorResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse)
    )
)]
pub async fn create_building(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Json(req): Json<CreateBuildingRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let user_id = rls.user_id();

    // RlsConnection already validates org membership via ValidatedTenantExtractor.
    // We just need to verify the request org_id matches the authenticated tenant.
    if req.organization_id != rls.tenant_id() {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "NOT_AUTHORIZED",
                "You are not a member of this organization",
            )),
        ));
    }

    // Creating a building is a manager action (closes #799). RLS enforces org
    // isolation but not role level, so gate explicitly.
    if !rls.is_super_admin() && !rls.has_role(TenantRole::Manager) {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "INSUFFICIENT_ROLE",
                "Manager role required to create buildings",
            )),
        ));
    }

    // Validate required fields
    if req.street.trim().is_empty() {
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new("INVALID_STREET", "Street is required")),
        ));
    }

    if req.city.trim().is_empty() {
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new("INVALID_CITY", "City is required")),
        ));
    }

    // Story 3.1 AC3: geocode on write. Degrades gracefully — an unconfigured
    // provider or a failed lookup simply leaves coordinates NULL.
    let address = format!(
        "{}, {} {}, {}",
        req.street, req.postal_code, req.city, req.country
    );
    let (latitude, longitude) = geocode_address(&state, &address).await;

    // Create building using RLS-enabled connection
    let create_data = CreateBuilding {
        organization_id: req.organization_id,
        street: req.street,
        city: req.city,
        postal_code: req.postal_code,
        country: req.country,
        name: req.name,
        description: req.description,
        year_built: req.year_built,
        total_floors: req.total_floors,
        total_entrances: req.total_entrances,
        amenities: req.amenities,
        latitude,
        longitude,
    };

    let building = state
        .building_repo
        .create_rls(&mut **rls.conn(), create_data)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to create building");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to create building")),
            )
        })?;

    tracing::info!(
        building_id = %building.id,
        org_id = %building.organization_id,
        user_id = %user_id,
        "Building created"
    );

    // Release RLS context before returning connection to pool
    rls.release().await;

    Ok((StatusCode::CREATED, Json(BuildingResponse::from(building))))
}

/// List buildings (with pagination).
#[utoipa::path(
    get,
    path = "/api/v1/buildings",
    tag = "Buildings",
    params(ListBuildingsQuery),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of buildings", body = BuildingsListResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse)
    )
)]
pub async fn list_buildings(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<ListBuildingsQuery>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // RLS policies filter by the resolved tenant, so a mismatched
    // `organization_id` would silently return an empty page. Reject it
    // explicitly instead — mirrors the guard in `create_building` so the
    // client-supplied org cannot diverge from the authenticated tenant
    // (closes the round-10 `list_buildings` finding in #789).
    if query.organization_id != rls.tenant_id() {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "NOT_AUTHORIZED",
                "You are not a member of this organization",
            )),
        ));
    }

    let limit = query
        .limit
        .unwrap_or(DEFAULT_LIST_LIMIT)
        .min(MAX_LIST_LIMIT);

    let (buildings, total) = state
        .building_repo
        .list_by_organization_with_count_rls(
            rls.conn(),
            query.organization_id,
            query.offset,
            limit,
            query.include_archived,
            query.search.as_deref(),
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to list buildings");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to list buildings")),
            )
        })?;

    rls.release().await;

    Ok(Json(BuildingsListResponse {
        buildings,
        total,
        offset: query.offset,
        limit,
    }))
}

/// Bulk import buildings (UC-15.8).
#[utoipa::path(
    post,
    path = "/api/v1/buildings/bulk",
    tag = "Buildings",
    request_body = BulkImportBuildingsRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Bulk import completed", body = BulkImportBuildingsResponse),
        (status = 400, description = "Invalid input", body = ErrorResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse)
    )
)]
pub async fn bulk_import_buildings(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Json(req): Json<BulkImportBuildingsRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // RLS policies automatically filter by tenant - user can only import to
    // organizations they belong to

    let user_id = rls.user_id();

    // Bulk-importing buildings is a manager action (closes #799).
    if !rls.is_super_admin() && !rls.has_role(TenantRole::Manager) {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "INSUFFICIENT_ROLE",
                "Manager role required to bulk-import buildings",
            )),
        ));
    }

    // Validate bulk import size
    if req.buildings.is_empty() {
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "EMPTY_IMPORT",
                "No buildings provided for import",
            )),
        ));
    }

    if req.buildings.len() > MAX_BULK_IMPORT_SIZE {
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "IMPORT_TOO_LARGE",
                format!(
                    "Maximum {} buildings per import request",
                    MAX_BULK_IMPORT_SIZE
                ),
            )),
        ));
    }

    let mut results = Vec::with_capacity(req.buildings.len());
    let mut successful = 0;
    let mut failed = 0;

    for (index, entry) in req.buildings.into_iter().enumerate() {
        // Validate required fields
        if entry.street.trim().is_empty() {
            results.push(BulkImportResult {
                index,
                success: false,
                building_id: None,
                error: Some("Street is required".to_string()),
            });
            failed += 1;
            continue;
        }

        if entry.city.trim().is_empty() {
            results.push(BulkImportResult {
                index,
                success: false,
                building_id: None,
                error: Some("City is required".to_string()),
            });
            failed += 1;
            continue;
        }

        // Create building
        let create_data = CreateBuilding {
            organization_id: req.organization_id,
            street: entry.street,
            city: entry.city,
            postal_code: entry.postal_code,
            country: entry.country,
            name: entry.name,
            description: entry.description,
            year_built: entry.year_built,
            total_floors: entry.total_floors,
            total_entrances: entry.total_entrances,
            amenities: entry.amenities,
            // Bulk import skips live geocoding to avoid N sequential external
            // calls; coordinates are filled on a later edit/update.
            latitude: None,
            longitude: None,
        };

        match state
            .building_repo
            .create_rls(&mut **rls.conn(), create_data)
            .await
        {
            Ok(building) => {
                results.push(BulkImportResult {
                    index,
                    success: true,
                    building_id: Some(building.id),
                    error: None,
                });
                successful += 1;
            }
            Err(e) => {
                tracing::warn!(index = index, error = %e, "Failed to import building");
                results.push(BulkImportResult {
                    index,
                    success: false,
                    building_id: None,
                    error: Some(format!("Database error: {}", e)),
                });
                failed += 1;
            }
        }
    }

    tracing::info!(
        org_id = %req.organization_id,
        user_id = %user_id,
        total = results.len(),
        successful = successful,
        failed = failed,
        "Bulk import completed"
    );

    rls.release().await;

    Ok(Json(BulkImportBuildingsResponse {
        total: results.len(),
        successful,
        failed,
        results,
    }))
}

/// Get building by ID (UC-15.2).
#[utoipa::path(
    get,
    path = "/api/v1/buildings/{id}",
    tag = "Buildings",
    params(("id" = Uuid, Path, description = "Building ID")),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Building found", body = BuildingResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Building not found", body = ErrorResponse)
    )
)]
pub async fn get_building(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // RLS policies automatically filter by tenant - user can only see buildings
    // in organizations they belong to
    let building = state
        .building_repo
        .find_by_id_rls(&mut **rls.conn(), id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get building");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?
        .ok_or_else(|| {
            // RLS will return None if user doesn't have access OR building doesn't exist
            // This is intentional - we don't leak existence of buildings user can't access
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Building not found")),
            )
        })?;

    rls.release().await;
    Ok(Json(BuildingResponse::from(building)))
}

/// Update building (UC-15.3).
#[utoipa::path(
    put,
    path = "/api/v1/buildings/{id}",
    tag = "Buildings",
    params(("id" = Uuid, Path, description = "Building ID")),
    request_body = UpdateBuildingRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Building updated", body = BuildingResponse),
        (status = 400, description = "Invalid input", body = ErrorResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Building not found", body = ErrorResponse)
    )
)]
pub async fn update_building(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateBuildingRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // Story 3.1 AC3: re-geocode when any address component changes. We load the
    // current row to fill in unchanged components so the lookup uses the full
    // effective address. Failure leaves coordinates untouched (graceful).
    let address_changed = req.street.is_some()
        || req.city.is_some()
        || req.postal_code.is_some()
        || req.country.is_some();
    let (latitude, longitude) = if address_changed {
        match state
            .building_repo
            .find_by_id_rls(&mut **rls.conn(), id)
            .await
        {
            Ok(Some(current)) => {
                let street = req.street.clone().unwrap_or(current.street);
                let city = req.city.clone().unwrap_or(current.city);
                let postal_code = req.postal_code.clone().unwrap_or(current.postal_code);
                let country = req.country.clone().unwrap_or(current.country);
                let address = format!("{}, {} {}, {}", street, postal_code, city, country);
                geocode_address(&state, &address).await
            }
            _ => (None, None),
        }
    } else {
        (None, None)
    };

    let update_data = UpdateBuilding {
        street: req.street,
        city: req.city,
        postal_code: req.postal_code,
        country: req.country,
        name: req.name,
        description: req.description,
        year_built: req.year_built,
        total_floors: req.total_floors,
        total_entrances: req.total_entrances,
        amenities: req.amenities,
        contacts: None,
        settings: None,
        latitude,
        longitude,
    };

    // RLS policies automatically enforce tenant isolation and access control
    let building = state
        .building_repo
        .update_rls(&mut **rls.conn(), id, update_data)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to update building");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to update building")),
            )
        })?
        .ok_or_else(|| {
            // RLS returns None if user doesn't have access OR building doesn't exist
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Building not found")),
            )
        })?;

    tracing::info!(building_id = %id, user_id = %rls.user_id(), "Building updated");

    rls.release().await;
    Ok(Json(BuildingResponse::from(building)))
}

/// Archive building (soft delete) (UC-15.10).
#[utoipa::path(
    delete,
    path = "/api/v1/buildings/{id}",
    tag = "Buildings",
    params(("id" = Uuid, Path, description = "Building ID")),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Building archived", body = BuildingResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Building not found", body = ErrorResponse)
    )
)]
pub async fn archive_building(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // Archiving a building is a manager action (closes #799).
    if !rls.is_super_admin() && !rls.has_role(TenantRole::Manager) {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "INSUFFICIENT_ROLE",
                "Manager role required to archive buildings",
            )),
        ));
    }

    // RLS policies automatically enforce tenant isolation and access control
    let building = state
        .building_repo
        .archive_rls(&mut **rls.conn(), id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to archive building");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to archive building")),
            )
        })?;

    let user_id = rls.user_id();
    rls.release().await;

    match building {
        Some(b) => {
            tracing::info!(building_id = %id, user_id = %user_id, "Building archived");
            Ok(Json(BuildingResponse::from(b)))
        }
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "NOT_FOUND",
                "Building not found or already archived",
            )),
        )),
    }
}

/// Restore archived building.
#[utoipa::path(
    post,
    path = "/api/v1/buildings/{id}/restore",
    tag = "Buildings",
    params(("id" = Uuid, Path, description = "Building ID")),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Building restored", body = BuildingResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Building not found", body = ErrorResponse)
    )
)]
pub async fn restore_building(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // RLS policies handle organization access.
    let building = state
        .building_repo
        .restore_rls(&mut **rls.conn(), id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to restore building");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to restore building")),
            )
        })?;

    let user_id = rls.user_id();
    rls.release().await;

    match building {
        Some(b) => {
            tracing::info!(building_id = %id, user_id = %user_id, "Building restored");
            Ok(Json(BuildingResponse::from(b)))
        }
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "NOT_FOUND",
                "Building not found or not archived",
            )),
        )),
    }
}

/// Get building statistics (UC-15.7).
#[utoipa::path(
    get,
    path = "/api/v1/buildings/{id}/statistics",
    tag = "Buildings",
    params(("id" = Uuid, Path, description = "Building ID")),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Building statistics", body = BuildingStatistics),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Building not found", body = ErrorResponse)
    )
)]
pub async fn get_building_statistics(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // RLS policies automatically filter by tenant - verify building exists in scope
    let _building = state
        .building_repo
        .find_by_id_rls(&mut **rls.conn(), id)
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

    let stats = state
        .building_repo
        .get_statistics_rls(&mut **rls.conn(), id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get building statistics");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DB_ERROR",
                    "Failed to get building statistics",
                )),
            )
        })?;

    rls.release().await;

    Ok(Json(stats))
}

// ==================== Unit Handlers ====================

/// List units in a building.
#[utoipa::path(
    get,
    path = "/api/v1/buildings/{id}/units",
    tag = "Units",
    params(
        ("id" = Uuid, Path, description = "Building ID"),
        ListUnitsQuery
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of units", body = UnitsListResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Building not found", body = ErrorResponse)
    )
)]
pub async fn list_units(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(building_id): Path<Uuid>,
    Query(query): Query<ListUnitsQuery>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // RLS policies automatically filter by tenant - verify building exists in scope
    let _building = state
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

    let limit = query
        .limit
        .unwrap_or(DEFAULT_LIST_LIMIT)
        .min(MAX_LIST_LIMIT);

    let (units, total) = state
        .unit_repo
        .list_by_building_with_count_rls(
            rls.conn(),
            building_id,
            query.offset,
            limit,
            query.include_archived,
            query.unit_type.as_deref(),
            query.floor,
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to list units");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to list units")),
            )
        })?;

    rls.release().await;

    Ok(Json(UnitsListResponse {
        units,
        total,
        offset: query.offset,
        limit,
    }))
}

/// Create a new unit (UC-15.4).
#[utoipa::path(
    post,
    path = "/api/v1/buildings/{id}/units",
    tag = "Units",
    params(("id" = Uuid, Path, description = "Building ID")),
    request_body = CreateUnitRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 201, description = "Unit created", body = UnitResponse),
        (status = 400, description = "Invalid input", body = ErrorResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Building not found", body = ErrorResponse),
        (status = 409, description = "Unit with this designation already exists", body = ErrorResponse)
    )
)]
pub async fn create_unit(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(building_id): Path<Uuid>,
    Json(req): Json<CreateUnitRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let user_id = rls.user_id();

    // RLS policies automatically filter by tenant - verify building exists in scope
    let _building = state
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

    // Validate required fields
    if req.designation.trim().is_empty() {
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_DESIGNATION",
                "Unit designation is required",
            )),
        ));
    }

    // Check for duplicate designation
    let exists = state
        .unit_repo
        .designation_exists_rls(&mut **rls.conn(), building_id, &req.designation, None)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to check designation");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?;

    if exists {
        rls.release().await;
        return Err((
            StatusCode::CONFLICT,
            Json(ErrorResponse::new(
                "DUPLICATE_DESIGNATION",
                "A unit with this designation already exists in this building",
            )),
        ));
    }

    // Validate unit type
    let valid_types = ["apartment", "commercial", "parking", "storage", "other"];
    if !valid_types.contains(&req.unit_type.as_str()) {
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_UNIT_TYPE",
                "Invalid unit type. Must be: apartment, commercial, parking, storage, or other",
            )),
        ));
    }

    // Create unit
    let create_data = CreateUnit {
        building_id,
        entrance: req.entrance,
        designation: req.designation,
        floor: req.floor,
        unit_type: req.unit_type,
        size_sqm: req.size_sqm,
        rooms: req.rooms,
        ownership_share: req.ownership_share,
        description: req.description,
    };

    let unit = state
        .unit_repo
        .create_rls(&mut **rls.conn(), create_data)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to create unit");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to create unit")),
            )
        })?;

    tracing::info!(
        unit_id = %unit.id,
        building_id = %building_id,
        user_id = %user_id,
        "Unit created"
    );

    rls.release().await;

    Ok((StatusCode::CREATED, Json(UnitResponse::from(unit))))
}

/// Get unit by ID (UC-15.5).
#[utoipa::path(
    get,
    path = "/api/v1/buildings/{building_id}/units/{unit_id}",
    tag = "Units",
    params(
        ("building_id" = Uuid, Path, description = "Building ID"),
        ("unit_id" = Uuid, Path, description = "Unit ID")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Unit found", body = UnitWithOwnersResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Unit not found", body = ErrorResponse)
    )
)]
pub async fn get_unit(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path((building_id, unit_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // RLS policies automatically filter by tenant - verify building exists in scope
    let _building = state
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

    // Get unit with RLS context
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
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Unit not found")),
            )
        })?;

    // Verify unit belongs to building
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

    // Get owners with RLS context
    let owners = state
        .unit_repo
        .get_owners_rls(&mut **rls.conn(), unit_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get unit owners");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?;

    rls.release().await;

    let unit_with_owners = UnitWithOwners { unit, owners };
    Ok(Json(UnitWithOwnersResponse::from(unit_with_owners)))
}

/// Update unit.
#[utoipa::path(
    put,
    path = "/api/v1/buildings/{building_id}/units/{unit_id}",
    tag = "Units",
    params(
        ("building_id" = Uuid, Path, description = "Building ID"),
        ("unit_id" = Uuid, Path, description = "Unit ID")
    ),
    request_body = UpdateUnitRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Unit updated", body = UnitResponse),
        (status = 400, description = "Invalid input", body = ErrorResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Unit not found", body = ErrorResponse),
        (status = 409, description = "Unit with this designation already exists", body = ErrorResponse)
    )
)]
pub async fn update_unit(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path((building_id, unit_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdateUnitRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let user_id = rls.user_id();

    // RLS policies automatically filter by tenant - verify building exists in scope
    let _building = state
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

    // Verify unit exists and belongs to building
    let existing = state
        .unit_repo
        .find_by_id_rls(&mut **rls.conn(), unit_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get unit");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Unit not found")),
            )
        })?;

    if existing.building_id != building_id {
        rls.release().await;
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "NOT_FOUND",
                "Unit not found in this building",
            )),
        ));
    }

    // Check for duplicate designation if being updated
    if let Some(ref designation) = req.designation {
        let exists = state
            .unit_repo
            .designation_exists_rls(&mut **rls.conn(), building_id, designation, Some(unit_id))
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to check designation");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new("DB_ERROR", "Database error")),
                )
            })?;

        if exists {
            rls.release().await;
            return Err((
                StatusCode::CONFLICT,
                Json(ErrorResponse::new(
                    "DUPLICATE_DESIGNATION",
                    "A unit with this designation already exists in this building",
                )),
            ));
        }
    }

    // Validate unit type if provided
    if let Some(ref unit_type) = req.unit_type {
        let valid_types = ["apartment", "commercial", "parking", "storage", "other"];
        if !valid_types.contains(&unit_type.as_str()) {
            rls.release().await;
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "INVALID_UNIT_TYPE",
                    "Invalid unit type. Must be: apartment, commercial, parking, storage, or other",
                )),
            ));
        }
    }

    // Validate occupancy status if provided
    if let Some(ref status) = req.occupancy_status {
        let valid_statuses = ["owner_occupied", "rented", "vacant", "unknown"];
        if !valid_statuses.contains(&status.as_str()) {
            rls.release().await;
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "INVALID_OCCUPANCY_STATUS",
                    "Invalid occupancy status. Must be: owner_occupied, rented, vacant, or unknown",
                )),
            ));
        }
    }

    let update_data = UpdateUnit {
        entrance: req.entrance,
        designation: req.designation,
        floor: req.floor,
        unit_type: req.unit_type,
        size_sqm: req.size_sqm,
        rooms: req.rooms,
        ownership_share: req.ownership_share,
        occupancy_status: req.occupancy_status,
        description: req.description,
        notes: req.notes,
        settings: None,
    };

    let unit = state
        .unit_repo
        .update_rls(&mut **rls.conn(), unit_id, update_data)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to update unit");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to update unit")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Unit not found")),
            )
        })?;

    tracing::info!(unit_id = %unit_id, user_id = %user_id, "Unit updated");

    rls.release().await;

    Ok(Json(UnitResponse::from(unit)))
}

/// Archive unit (soft delete).
#[utoipa::path(
    delete,
    path = "/api/v1/buildings/{building_id}/units/{unit_id}",
    tag = "Units",
    params(
        ("building_id" = Uuid, Path, description = "Building ID"),
        ("unit_id" = Uuid, Path, description = "Unit ID")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Unit archived", body = UnitResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Unit not found", body = ErrorResponse)
    )
)]
pub async fn archive_unit(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path((building_id, unit_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // RLS policies automatically filter by tenant - verify building exists in scope
    let _building = state
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

    // Verify unit exists and belongs to building
    let existing = state
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

    if let Some(u) = existing {
        if u.building_id != building_id {
            rls.release().await;
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    "Unit not found in this building",
                )),
            ));
        }
    }

    let unit = state
        .unit_repo
        .archive_rls(&mut **rls.conn(), unit_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to archive unit");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to archive unit")),
            )
        })?;

    let user_id = rls.user_id();
    rls.release().await;

    match unit {
        Some(u) => {
            tracing::info!(unit_id = %unit_id, user_id = %user_id, "Unit archived");
            Ok(Json(UnitResponse::from(u)))
        }
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "NOT_FOUND",
                "Unit not found or already archived",
            )),
        )),
    }
}

/// Restore archived unit.
#[utoipa::path(
    post,
    path = "/api/v1/buildings/{building_id}/units/{unit_id}/restore",
    tag = "Units",
    params(
        ("building_id" = Uuid, Path, description = "Building ID"),
        ("unit_id" = Uuid, Path, description = "Unit ID")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Unit restored", body = UnitResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Unit not found", body = ErrorResponse)
    )
)]
pub async fn restore_unit(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path((building_id, unit_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // RLS policies automatically filter by tenant - verify building exists in scope
    let _building = state
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

    // Verify unit belongs to building
    let belongs = state
        .unit_repo
        .belongs_to_building_rls(&mut **rls.conn(), unit_id, building_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to check unit building");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?;

    if !belongs {
        rls.release().await;
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "NOT_FOUND",
                "Unit not found in this building",
            )),
        ));
    }

    let unit = state
        .unit_repo
        .restore_rls(&mut **rls.conn(), unit_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to restore unit");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to restore unit")),
            )
        })?;

    let user_id = rls.user_id();
    rls.release().await;

    match unit {
        Some(u) => {
            tracing::info!(unit_id = %unit_id, user_id = %user_id, "Unit restored");
            Ok(Json(UnitResponse::from(u)))
        }
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "NOT_FOUND",
                "Unit not found or not archived",
            )),
        )),
    }
}

// ==================== Unit Owner Handlers ====================

/// List owners for a unit.
#[utoipa::path(
    get,
    path = "/api/v1/buildings/{building_id}/units/{unit_id}/owners",
    tag = "Unit Owners",
    params(
        ("building_id" = Uuid, Path, description = "Building ID"),
        ("unit_id" = Uuid, Path, description = "Unit ID")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of unit owners", body = Vec<UnitOwnerInfo>),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Unit not found", body = ErrorResponse)
    )
)]
pub async fn list_unit_owners(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path((building_id, unit_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // RLS policies automatically filter by tenant - verify building exists in scope
    let _building = state
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
        })?
        .ok_or_else(|| {
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

    let owners = state
        .unit_repo
        .get_owners_rls(&mut **rls.conn(), unit_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get unit owners");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to get unit owners")),
            )
        })?;

    rls.release().await;

    Ok(Json(owners))
}

/// Assign owner to unit (UC-15.6).
#[utoipa::path(
    post,
    path = "/api/v1/buildings/{building_id}/units/{unit_id}/owners",
    tag = "Unit Owners",
    params(
        ("building_id" = Uuid, Path, description = "Building ID"),
        ("unit_id" = Uuid, Path, description = "Unit ID")
    ),
    request_body = AssignOwnerRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 201, description = "Owner assigned", body = UnitOwnerInfo),
        (status = 400, description = "Invalid input", body = ErrorResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Unit not found", body = ErrorResponse)
    )
)]
pub async fn assign_unit_owner(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path((building_id, unit_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<AssignOwnerRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let user_id = rls.user_id();

    // RLS policies automatically filter by tenant - verify building exists in scope
    let _building = state
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
        })?
        .ok_or_else(|| {
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

    // Validate ownership percentage
    if req.ownership_percentage <= Decimal::ZERO
        || req.ownership_percentage > Decimal::new(10000, 2)
    {
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_PERCENTAGE",
                "Ownership percentage must be between 0 and 100",
            )),
        ));
    }

    // Check that total ownership won't exceed 100%
    let current_total = state
        .unit_repo
        .get_total_ownership_rls(&mut **rls.conn(), unit_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get total ownership");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?;

    if current_total + req.ownership_percentage > Decimal::new(10000, 2) {
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "EXCEEDS_100_PERCENT",
                "Total ownership would exceed 100%",
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

    // Assign owner
    let assign_data = db::models::AssignUnitOwner {
        unit_id,
        user_id: req.user_id,
        ownership_percentage: req.ownership_percentage,
        is_primary: req.is_primary,
        valid_from: req.valid_from,
    };

    let owner = state
        .unit_repo
        .assign_owner_rls(&mut **rls.conn(), assign_data)
        .await
        .map_err(|e| {
            // Check for duplicate key violation
            if e.to_string().contains("unique_owner_per_unit") {
                return (
                    StatusCode::CONFLICT,
                    Json(ErrorResponse::new(
                        "ALREADY_OWNER",
                        "This user is already an owner of this unit",
                    )),
                );
            }
            tracing::error!(error = %e, "Failed to assign owner");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to assign owner")),
            )
        })?;

    tracing::info!(
        unit_id = %unit_id,
        owner_user_id = %req.user_id,
        by_user_id = %user_id,
        "Unit owner assigned"
    );

    // Return owner info. The owner row was just written above so the user
    // must exist; if `find_by_id` returns `None` we have an invariant break
    // (race with delete, foreign-key drift) and surface 500 rather than panic.
    let user = state
        .user_repo
        .find_by_id(req.user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get user");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?
        .ok_or_else(|| {
            tracing::error!(
                user_id = %req.user_id,
                "Owner row was created but user lookup returned None — invariant violation"
            );
            // Distinct from client-facing `USER_NOT_FOUND` 400s earlier in
            // this handler: the owner row was just written successfully, so
            // the user MUST exist. Reaching this arm is a server-side
            // invariant violation, not a missing-input error — surface it
            // as `INTERNAL_ERROR` so log/consumer triage can tell the two
            // apart (PR #300 review).
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "User row missing after successful write",
                )),
            )
        })?;

    rls.release().await;

    let owner_info = UnitOwnerInfo {
        user_id: owner.user_id,
        user_name: user.name,
        user_email: user.email,
        ownership_percentage: owner.ownership_percentage,
        is_primary: owner.is_primary,
    };

    Ok((StatusCode::CREATED, Json(owner_info)))
}

/// Update unit owner.
#[utoipa::path(
    put,
    path = "/api/v1/buildings/{building_id}/units/{unit_id}/owners/{user_id}",
    tag = "Unit Owners",
    params(
        ("building_id" = Uuid, Path, description = "Building ID"),
        ("unit_id" = Uuid, Path, description = "Unit ID"),
        ("user_id" = Uuid, Path, description = "Owner user ID")
    ),
    request_body = UpdateOwnerRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Owner updated", body = UnitOwnerInfo),
        (status = 400, description = "Invalid input", body = ErrorResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Owner not found", body = ErrorResponse)
    )
)]
pub async fn update_unit_owner(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path((building_id, unit_id, owner_user_id)): Path<(Uuid, Uuid, Uuid)>,
    Json(req): Json<UpdateOwnerRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let auth_user_id = rls.user_id();

    // RLS policies automatically filter by tenant - verify building exists in scope
    let _building = state
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

    // Verify unit exists and belongs to building
    let belongs = state
        .unit_repo
        .belongs_to_building_rls(&mut **rls.conn(), unit_id, building_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to check unit building");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?;

    if !belongs {
        rls.release().await;
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "NOT_FOUND",
                "Unit not found in this building",
            )),
        ));
    }

    // Validate ownership percentage if provided
    if let Some(pct) = req.ownership_percentage {
        if pct <= Decimal::ZERO || pct > Decimal::new(10000, 2) {
            rls.release().await;
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "INVALID_PERCENTAGE",
                    "Ownership percentage must be between 0 and 100",
                )),
            ));
        }
    }

    let owner = state
        .unit_repo
        .update_owner_rls(
            &mut **rls.conn(),
            unit_id,
            owner_user_id,
            req.ownership_percentage,
            req.is_primary,
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to update owner");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to update owner")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    "Owner not found for this unit",
                )),
            )
        })?;

    tracing::info!(
        unit_id = %unit_id,
        owner_user_id = %owner_user_id,
        by_user_id = %auth_user_id,
        "Unit owner updated"
    );

    // Get user info. The owner update succeeded above, so the user must
    // exist; if not we have an invariant break and surface 500 rather than
    // panic on `.unwrap()`.
    let user = state
        .user_repo
        .find_by_id(owner_user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get user");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?
        .ok_or_else(|| {
            tracing::error!(
                user_id = %owner_user_id,
                "Owner row was updated but user lookup returned None — invariant violation"
            );
            // See `assign_unit_owner` — `INTERNAL_ERROR` instead of
            // `USER_NOT_FOUND` so the invariant-violation path is
            // distinguishable from the client-input 400s earlier in this
            // handler (PR #300 review).
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "User row missing after successful write",
                )),
            )
        })?;

    rls.release().await;

    let owner_info = UnitOwnerInfo {
        user_id: owner.user_id,
        user_name: user.name,
        user_email: user.email,
        ownership_percentage: owner.ownership_percentage,
        is_primary: owner.is_primary,
    };

    Ok(Json(owner_info))
}

/// Remove owner from unit.
#[utoipa::path(
    delete,
    path = "/api/v1/buildings/{building_id}/units/{unit_id}/owners/{user_id}",
    tag = "Unit Owners",
    params(
        ("building_id" = Uuid, Path, description = "Building ID"),
        ("unit_id" = Uuid, Path, description = "Unit ID"),
        ("user_id" = Uuid, Path, description = "Owner user ID")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Owner removed"),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Owner not found", body = ErrorResponse)
    )
)]
pub async fn remove_unit_owner(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path((building_id, unit_id, owner_user_id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let auth_user_id = rls.user_id();

    // RLS policies automatically filter by tenant - verify building exists in scope
    let _building = state
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

    // Verify unit belongs to building
    let belongs = state
        .unit_repo
        .belongs_to_building_rls(&mut **rls.conn(), unit_id, building_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to check unit building");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?;

    if !belongs {
        rls.release().await;
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "NOT_FOUND",
                "Unit not found in this building",
            )),
        ));
    }

    let removed = state
        .unit_repo
        .remove_owner_rls(&mut **rls.conn(), unit_id, owner_user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to remove owner");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to remove owner")),
            )
        })?;

    rls.release().await;

    if removed {
        tracing::info!(
            unit_id = %unit_id,
            owner_user_id = %owner_user_id,
            by_user_id = %auth_user_id,
            "Unit owner removed"
        );
        Ok(StatusCode::OK)
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "NOT_FOUND",
                "Owner not found for this unit",
            )),
        ))
    }
}
