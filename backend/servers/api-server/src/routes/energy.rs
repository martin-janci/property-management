//! Energy & Sustainability routes (Epic 65).
//!
//! Implements energy performance certificates, carbon footprint tracking,
//! sustainability scores, and utility benchmarking.

use api_core::extractors::AuthUser;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post, put},
    Json, Router,
};
use chrono::Datelike;
use common::errors::ErrorResponse;
use db::models::energy::{
    BenchmarkAlertsQuery, BenchmarkQuery, CalculateBenchmark, CreateCarbonEmission,
    CreateCarbonTarget, CreateEnergyPerformanceCertificate, CreateSustainabilityScore,
    SustainabilityFilter, UpdateEnergyPerformanceCertificate, UpdateSustainabilityScore,
};
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::state::AppState;

/// Default page size for listings
const DEFAULT_LIST_LIMIT: i64 = 50;

/// Maximum page size
const MAX_LIST_LIMIT: i64 = 100;

type ApiResult<T> = Result<T, (StatusCode, Json<ErrorResponse>)>;

/// Create energy routes router.
pub fn router() -> Router<AppState> {
    Router::new()
        // Energy Performance Certificates (Story 65.1)
        .route("/units/{unit_id}/epc", get(get_unit_epc))
        .route("/units/{unit_id}/epc", post(create_unit_epc))
        .route("/units/{unit_id}/epc", put(update_unit_epc))
        .route("/buildings/{building_id}/epcs", get(list_building_epcs))
        .route("/epc/{id}", get(get_epc))
        .route("/epc/{id}", delete(delete_epc))
        // Carbon Footprint (Story 65.2)
        .route("/buildings/{building_id}/carbon", get(get_carbon_dashboard))
        .route("/buildings/{building_id}/emissions", post(record_emission))
        .route("/buildings/{building_id}/emissions", get(list_emissions))
        .route(
            "/buildings/{building_id}/carbon/target",
            post(set_carbon_target),
        )
        .route(
            "/buildings/{building_id}/carbon/export",
            get(export_carbon_report),
        )
        // Sustainability Scores (Story 65.3)
        .route(
            "/listings/{listing_id}/sustainability",
            get(get_sustainability_score),
        )
        .route(
            "/listings/{listing_id}/sustainability",
            post(create_sustainability_score),
        )
        .route(
            "/listings/{listing_id}/sustainability",
            put(update_sustainability_score),
        )
        .route(
            "/listings/sustainability/search",
            get(search_sustainable_listings),
        )
        // Benchmarking (Story 65.4)
        .route(
            "/buildings/{building_id}/benchmark",
            get(get_benchmark_dashboard),
        )
        .route(
            "/buildings/{building_id}/benchmark/calculate",
            post(calculate_benchmark),
        )
        .route(
            "/buildings/{building_id}/benchmark/alerts",
            get(list_benchmark_alerts),
        )
        .route(
            "/benchmark/alerts/{id}/resolve",
            post(resolve_benchmark_alert),
        )
}

// ==================== Request/Response Types ====================

/// Create EPC request with organization context.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateEpcRequest {
    /// Organization ID
    pub organization_id: Uuid,
    /// EPC data
    #[serde(flatten)]
    pub data: CreateEnergyPerformanceCertificate,
}

/// Update EPC request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateEpcRequest {
    #[serde(flatten)]
    pub data: UpdateEnergyPerformanceCertificate,
}

/// Create carbon emission request with organization context.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateEmissionRequest {
    /// Organization ID
    pub organization_id: Uuid,
    /// Emission data
    #[serde(flatten)]
    pub data: CreateCarbonEmission,
}

/// Set carbon target request with organization context.
#[derive(Debug, Deserialize, ToSchema)]
pub struct SetCarbonTargetRequest {
    /// Organization ID
    pub organization_id: Uuid,
    /// Target data
    #[serde(flatten)]
    pub data: CreateCarbonTarget,
}

/// Create sustainability score request with organization context.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateSustainabilityScoreRequest {
    /// Organization ID
    pub organization_id: Uuid,
    /// Score data
    #[serde(flatten)]
    pub data: CreateSustainabilityScore,
}

/// Update sustainability score request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateSustainabilityScoreRequest {
    #[serde(flatten)]
    pub data: UpdateSustainabilityScore,
}

/// Calculate benchmark request with organization context.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CalculateBenchmarkRequest {
    /// Organization ID
    pub organization_id: Uuid,
    /// Benchmark calculation data
    #[serde(flatten)]
    pub data: CalculateBenchmark,
}

/// List query parameters.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListQuery {
    /// Page limit
    #[serde(default = "default_limit")]
    pub limit: i64,
    /// Page offset
    #[serde(default)]
    pub offset: i64,
}

/// Carbon dashboard query parameters.
#[derive(Debug, Deserialize, IntoParams)]
pub struct CarbonDashboardQuery {
    /// Year to display (defaults to current year)
    pub year: Option<i32>,
}

/// Emissions list query parameters.
#[derive(Debug, Deserialize, IntoParams)]
pub struct EmissionsQuery {
    /// Year filter
    pub year: Option<i32>,
    /// Page limit
    #[serde(default = "default_limit")]
    pub limit: i64,
    /// Page offset
    #[serde(default)]
    pub offset: i64,
}

/// Sustainability search query parameters.
#[derive(Debug, Deserialize, IntoParams)]
pub struct SustainabilitySearchQuery {
    /// Organization ID
    pub organization_id: Uuid,
    /// Minimum sustainability score (1-100)
    pub min_score: Option<i32>,
    /// Has solar panels
    pub has_solar: Option<bool>,
    /// Has heat pump
    pub has_heat_pump: Option<bool>,
    /// Has EV charging
    pub has_ev_charging: Option<bool>,
    /// Page limit
    #[serde(default = "default_limit")]
    pub limit: i64,
    /// Page offset
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    DEFAULT_LIST_LIMIT
}

fn internal_error(msg: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse::new("DB_ERROR", msg)),
    )
}

fn not_found_error(msg: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse::new("NOT_FOUND", msg)),
    )
}

// ==================== EPC Handlers (Story 65.1) ====================

/// Get EPC for a unit.
#[utoipa::path(
    get,
    path = "/api/v1/energy/units/{unit_id}/epc",
    params(("unit_id" = Uuid, Path, description = "Unit ID")),
    responses(
        (status = 200, description = "EPC details", body = db::models::energy::EnergyPerformanceCertificate),
        (status = 404, description = "Not found", body = ErrorResponse),
    ),
    tag = "energy"
)]
async fn get_unit_epc(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(unit_id): Path<Uuid>,
) -> ApiResult<Json<db::models::energy::EnergyPerformanceCertificate>> {
    let repo = &state.energy_repo;

    match repo.get_epc_for_unit(unit_id).await {
        Ok(Some(epc)) => Ok(Json(epc)),
        Ok(None) => Err(not_found_error("EPC not found for unit")),
        Err(e) => {
            tracing::error!("Failed to get EPC: {:?}", e);
            Err(internal_error("Failed to get EPC"))
        }
    }
}

/// Create EPC for a unit.
#[utoipa::path(
    post,
    path = "/api/v1/energy/units/{unit_id}/epc",
    params(("unit_id" = Uuid, Path, description = "Unit ID")),
    request_body = CreateEpcRequest,
    responses(
        (status = 201, description = "EPC created", body = db::models::energy::EnergyPerformanceCertificate),
        (status = 400, description = "Invalid request", body = ErrorResponse),
    ),
    tag = "energy"
)]
async fn create_unit_epc(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(_unit_id): Path<Uuid>,
    Json(payload): Json<CreateEpcRequest>,
) -> ApiResult<(
    StatusCode,
    Json<db::models::energy::EnergyPerformanceCertificate>,
)> {
    let repo = &state.energy_repo;

    repo.create_epc(payload.organization_id, auth.user_id, payload.data)
        .await
        .map(|epc| (StatusCode::CREATED, Json(epc)))
        .map_err(|e| {
            tracing::error!("Failed to create EPC: {:?}", e);
            internal_error("Failed to create EPC")
        })
}

/// Update EPC for a unit.
#[utoipa::path(
    put,
    path = "/api/v1/energy/units/{unit_id}/epc",
    params(("unit_id" = Uuid, Path, description = "Unit ID")),
    request_body = UpdateEpcRequest,
    responses(
        (status = 200, description = "EPC updated", body = db::models::energy::EnergyPerformanceCertificate),
        (status = 404, description = "Not found", body = ErrorResponse),
    ),
    tag = "energy"
)]
async fn update_unit_epc(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(unit_id): Path<Uuid>,
    Json(payload): Json<UpdateEpcRequest>,
) -> ApiResult<Json<db::models::energy::EnergyPerformanceCertificate>> {
    let repo = &state.energy_repo;

    // First get the EPC for this unit
    let epc = match repo.get_epc_for_unit(unit_id).await {
        Ok(Some(epc)) => epc,
        Ok(None) => return Err(not_found_error("EPC not found for unit")),
        Err(e) => {
            tracing::error!("Failed to get EPC: {:?}", e);
            return Err(internal_error("Failed to get EPC"));
        }
    };

    repo.update_epc(epc.id, payload.data)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to update EPC: {:?}", e);
            internal_error("Failed to update EPC")
        })
}

/// List EPCs for a building.
#[utoipa::path(
    get,
    path = "/api/v1/energy/buildings/{building_id}/epcs",
    params(("building_id" = Uuid, Path, description = "Building ID"), ListQuery),
    responses(
        (status = 200, description = "EPCs list", body = db::models::energy::ListEpcsResponse),
    ),
    tag = "energy"
)]
async fn list_building_epcs(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(building_id): Path<Uuid>,
    Query(query): Query<ListQuery>,
) -> ApiResult<Json<db::models::energy::ListEpcsResponse>> {
    let repo = &state.energy_repo;
    let limit = query.limit.min(MAX_LIST_LIMIT);

    repo.list_epcs_for_building(building_id, limit, query.offset)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to list EPCs: {:?}", e);
            internal_error("Failed to list EPCs")
        })
}

/// Get EPC by ID.
#[utoipa::path(
    get,
    path = "/api/v1/energy/epc/{id}",
    params(("id" = Uuid, Path, description = "EPC ID")),
    responses(
        (status = 200, description = "EPC details", body = db::models::energy::EnergyPerformanceCertificate),
        (status = 404, description = "Not found", body = ErrorResponse),
    ),
    tag = "energy"
)]
async fn get_epc(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<db::models::energy::EnergyPerformanceCertificate>> {
    let repo = &state.energy_repo;

    match repo.get_epc(id).await {
        Ok(Some(epc)) => Ok(Json(epc)),
        Ok(None) => Err(not_found_error("EPC not found")),
        Err(e) => {
            tracing::error!("Failed to get EPC: {:?}", e);
            Err(internal_error("Failed to get EPC"))
        }
    }
}

/// Delete EPC.
#[utoipa::path(
    delete,
    path = "/api/v1/energy/epc/{id}",
    params(("id" = Uuid, Path, description = "EPC ID")),
    responses(
        (status = 204, description = "EPC deleted"),
        (status = 404, description = "Not found", body = ErrorResponse),
    ),
    tag = "energy"
)]
async fn delete_epc(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    let repo = &state.energy_repo;

    match repo.delete_epc(id).await {
        Ok(true) => Ok(StatusCode::NO_CONTENT),
        Ok(false) => Err(not_found_error("EPC not found")),
        Err(e) => {
            tracing::error!("Failed to delete EPC: {:?}", e);
            Err(internal_error("Failed to delete EPC"))
        }
    }
}

// ==================== Carbon Footprint Handlers (Story 65.2) ====================

/// Get carbon dashboard for a building.
#[utoipa::path(
    get,
    path = "/api/v1/energy/buildings/{building_id}/carbon",
    params(("building_id" = Uuid, Path, description = "Building ID"), CarbonDashboardQuery),
    responses(
        (status = 200, description = "Carbon dashboard", body = db::models::energy::CarbonDashboard),
        (status = 404, description = "Building not found", body = ErrorResponse),
    ),
    tag = "energy"
)]
async fn get_carbon_dashboard(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(building_id): Path<Uuid>,
    Query(query): Query<CarbonDashboardQuery>,
) -> ApiResult<Json<db::models::energy::CarbonDashboard>> {
    let repo = &state.energy_repo;
    let year = query.year.unwrap_or_else(|| chrono::Utc::now().year());

    match repo.get_carbon_dashboard(building_id, year).await {
        Ok(dashboard) => Ok(Json(dashboard)),
        Err(sqlx::Error::RowNotFound) => Err(not_found_error("Building not found")),
        Err(e) => {
            tracing::error!("Failed to get carbon dashboard: {:?}", e);
            Err(internal_error("Failed to get carbon dashboard"))
        }
    }
}

/// Record carbon emission.
#[utoipa::path(
    post,
    path = "/api/v1/energy/buildings/{building_id}/emissions",
    params(("building_id" = Uuid, Path, description = "Building ID")),
    request_body = CreateEmissionRequest,
    responses(
        (status = 201, description = "Emission recorded", body = db::models::energy::CarbonEmission),
        (status = 400, description = "Invalid request", body = ErrorResponse),
    ),
    tag = "energy"
)]
async fn record_emission(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(_building_id): Path<Uuid>,
    Json(payload): Json<CreateEmissionRequest>,
) -> ApiResult<(StatusCode, Json<db::models::energy::CarbonEmission>)> {
    let repo = &state.energy_repo;

    repo.create_emission(payload.organization_id, auth.user_id, payload.data)
        .await
        .map(|emission| (StatusCode::CREATED, Json(emission)))
        .map_err(|e| {
            tracing::error!("Failed to record emission: {:?}", e);
            internal_error("Failed to record emission")
        })
}

/// List emissions for a building.
#[utoipa::path(
    get,
    path = "/api/v1/energy/buildings/{building_id}/emissions",
    params(("building_id" = Uuid, Path, description = "Building ID"), EmissionsQuery),
    responses(
        (status = 200, description = "Emissions list", body = db::models::energy::ListEmissionsResponse),
    ),
    tag = "energy"
)]
async fn list_emissions(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(building_id): Path<Uuid>,
    Query(query): Query<EmissionsQuery>,
) -> ApiResult<Json<db::models::energy::ListEmissionsResponse>> {
    let repo = &state.energy_repo;
    let limit = query.limit.min(MAX_LIST_LIMIT);

    repo.list_emissions(building_id, query.year, limit, query.offset)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to list emissions: {:?}", e);
            internal_error("Failed to list emissions")
        })
}

/// Set carbon target for a building.
#[utoipa::path(
    post,
    path = "/api/v1/energy/buildings/{building_id}/carbon/target",
    params(("building_id" = Uuid, Path, description = "Building ID")),
    request_body = SetCarbonTargetRequest,
    responses(
        (status = 201, description = "Target set", body = db::models::energy::CarbonTarget),
        (status = 400, description = "Invalid request", body = ErrorResponse),
    ),
    tag = "energy"
)]
async fn set_carbon_target(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(_building_id): Path<Uuid>,
    Json(payload): Json<SetCarbonTargetRequest>,
) -> ApiResult<(StatusCode, Json<db::models::energy::CarbonTarget>)> {
    let repo = &state.energy_repo;

    repo.set_carbon_target(payload.organization_id, payload.data)
        .await
        .map(|target| (StatusCode::CREATED, Json(target)))
        .map_err(|e| {
            tracing::error!("Failed to set carbon target: {:?}", e);
            internal_error("Failed to set carbon target")
        })
}

/// Export carbon report as PDF.
#[utoipa::path(
    get,
    path = "/api/v1/energy/buildings/{building_id}/carbon/export",
    params(("building_id" = Uuid, Path, description = "Building ID")),
    responses(
        (status = 200, description = "PDF report", content_type = "application/pdf"),
        (status = 404, description = "Building not found", body = ErrorResponse),
    ),
    tag = "energy"
)]
async fn export_carbon_report(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(building_id): Path<Uuid>,
    Query(query): Query<CarbonDashboardQuery>,
) -> ApiResult<Json<db::models::energy::CarbonDashboard>> {
    // For now, return JSON; PDF generation would be added later
    let repo = &state.energy_repo;
    let year = query.year.unwrap_or_else(|| chrono::Utc::now().year());

    match repo.get_carbon_dashboard(building_id, year).await {
        Ok(dashboard) => Ok(Json(dashboard)),
        Err(sqlx::Error::RowNotFound) => Err(not_found_error("Building not found")),
        Err(e) => {
            tracing::error!("Failed to export carbon report: {:?}", e);
            Err(internal_error("Failed to export carbon report"))
        }
    }
}

// ==================== Sustainability Score Handlers (Story 65.3) ====================

/// Get sustainability score for a listing.
#[utoipa::path(
    get,
    path = "/api/v1/energy/listings/{listing_id}/sustainability",
    params(("listing_id" = Uuid, Path, description = "Listing ID")),
    responses(
        (status = 200, description = "Sustainability score", body = db::models::energy::SustainabilityScore),
        (status = 404, description = "Not found", body = ErrorResponse),
    ),
    tag = "energy"
)]
async fn get_sustainability_score(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(listing_id): Path<Uuid>,
) -> ApiResult<Json<db::models::energy::SustainabilityScore>> {
    let repo = &state.energy_repo;

    match repo.get_sustainability_score(listing_id).await {
        Ok(Some(score)) => Ok(Json(score)),
        Ok(None) => Err(not_found_error("Sustainability score not found")),
        Err(e) => {
            tracing::error!("Failed to get sustainability score: {:?}", e);
            Err(internal_error("Failed to get sustainability score"))
        }
    }
}

/// Create sustainability score for a listing.
#[utoipa::path(
    post,
    path = "/api/v1/energy/listings/{listing_id}/sustainability",
    params(("listing_id" = Uuid, Path, description = "Listing ID")),
    request_body = CreateSustainabilityScoreRequest,
    responses(
        (status = 201, description = "Score created", body = db::models::energy::SustainabilityScore),
        (status = 400, description = "Invalid request", body = ErrorResponse),
    ),
    tag = "energy"
)]
async fn create_sustainability_score(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(_listing_id): Path<Uuid>,
    Json(payload): Json<CreateSustainabilityScoreRequest>,
) -> ApiResult<(StatusCode, Json<db::models::energy::SustainabilityScore>)> {
    let repo = &state.energy_repo;

    repo.upsert_sustainability_score(payload.organization_id, payload.data)
        .await
        .map(|score| (StatusCode::CREATED, Json(score)))
        .map_err(|e| {
            tracing::error!("Failed to create sustainability score: {:?}", e);
            internal_error("Failed to create sustainability score")
        })
}

/// Update sustainability score for a listing.
#[utoipa::path(
    put,
    path = "/api/v1/energy/listings/{listing_id}/sustainability",
    params(("listing_id" = Uuid, Path, description = "Listing ID")),
    request_body = UpdateSustainabilityScoreRequest,
    responses(
        (status = 200, description = "Score updated", body = db::models::energy::SustainabilityScore),
        (status = 404, description = "Not found", body = ErrorResponse),
    ),
    tag = "energy"
)]
async fn update_sustainability_score(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(listing_id): Path<Uuid>,
    Json(payload): Json<UpdateSustainabilityScoreRequest>,
) -> ApiResult<Json<db::models::energy::SustainabilityScore>> {
    let repo = &state.energy_repo;

    match repo
        .update_sustainability_score(listing_id, payload.data)
        .await
    {
        Ok(score) => Ok(Json(score)),
        Err(sqlx::Error::RowNotFound) => Err(not_found_error("Sustainability score not found")),
        Err(e) => {
            tracing::error!("Failed to update sustainability score: {:?}", e);
            Err(internal_error("Failed to update sustainability score"))
        }
    }
}

/// Search listings by sustainability criteria.
#[utoipa::path(
    get,
    path = "/api/v1/energy/listings/sustainability/search",
    params(SustainabilitySearchQuery),
    responses(
        (status = 200, description = "Matching listing IDs", body = Vec<Uuid>),
    ),
    tag = "energy"
)]
async fn search_sustainable_listings(
    State(state): State<AppState>,
    _auth: AuthUser,
    Query(query): Query<SustainabilitySearchQuery>,
) -> ApiResult<Json<Vec<Uuid>>> {
    let repo = &state.energy_repo;
    let limit = query.limit.min(MAX_LIST_LIMIT);

    let filter = SustainabilityFilter {
        min_score: query.min_score,
        has_solar: query.has_solar,
        has_heat_pump: query.has_heat_pump,
        has_ev_charging: query.has_ev_charging,
        min_energy_rating: None,
    };

    repo.filter_listings_by_sustainability(query.organization_id, filter, limit, query.offset)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to search sustainable listings: {:?}", e);
            internal_error("Failed to search sustainable listings")
        })
}

// ==================== Benchmarking Handlers (Story 65.4) ====================

/// Get benchmark dashboard for a building.
#[utoipa::path(
    get,
    path = "/api/v1/energy/buildings/{building_id}/benchmark",
    params(("building_id" = Uuid, Path, description = "Building ID"), BenchmarkQuery),
    responses(
        (status = 200, description = "Benchmark dashboard", body = db::models::energy::BenchmarkDashboard),
        (status = 404, description = "Building not found", body = ErrorResponse),
    ),
    tag = "energy"
)]
async fn get_benchmark_dashboard(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(building_id): Path<Uuid>,
    Query(query): Query<BenchmarkQuery>,
) -> ApiResult<Json<db::models::energy::BenchmarkDashboard>> {
    let repo = &state.energy_repo;

    match repo.get_benchmark_dashboard(building_id, query).await {
        Ok(dashboard) => Ok(Json(dashboard)),
        Err(sqlx::Error::RowNotFound) => Err(not_found_error("Building not found")),
        Err(e) => {
            tracing::error!("Failed to get benchmark dashboard: {:?}", e);
            Err(internal_error("Failed to get benchmark dashboard"))
        }
    }
}

/// Calculate benchmark for a building.
#[utoipa::path(
    post,
    path = "/api/v1/energy/buildings/{building_id}/benchmark/calculate",
    params(("building_id" = Uuid, Path, description = "Building ID")),
    request_body = CalculateBenchmarkRequest,
    responses(
        (status = 201, description = "Benchmark calculated", body = db::models::energy::BuildingBenchmark),
        (status = 400, description = "Invalid request", body = ErrorResponse),
    ),
    tag = "energy"
)]
async fn calculate_benchmark(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(_building_id): Path<Uuid>,
    Json(payload): Json<CalculateBenchmarkRequest>,
) -> ApiResult<(StatusCode, Json<db::models::energy::BuildingBenchmark>)> {
    let repo = &state.energy_repo;

    repo.calculate_benchmark(payload.organization_id, payload.data)
        .await
        .map(|benchmark| (StatusCode::CREATED, Json(benchmark)))
        .map_err(|e| {
            tracing::error!("Failed to calculate benchmark: {:?}", e);
            internal_error("Failed to calculate benchmark")
        })
}

/// List benchmark alerts for a building.
#[utoipa::path(
    get,
    path = "/api/v1/energy/buildings/{building_id}/benchmark/alerts",
    params(("building_id" = Uuid, Path, description = "Building ID"), BenchmarkAlertsQuery),
    responses(
        (status = 200, description = "Benchmark alerts", body = db::models::energy::ListBenchmarkAlertsResponse),
    ),
    tag = "energy"
)]
async fn list_benchmark_alerts(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(building_id): Path<Uuid>,
    Query(query): Query<BenchmarkAlertsQuery>,
) -> ApiResult<Json<db::models::energy::ListBenchmarkAlertsResponse>> {
    let repo = &state.energy_repo;

    repo.list_benchmark_alerts(building_id, query)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to list benchmark alerts: {:?}", e);
            internal_error("Failed to list benchmark alerts")
        })
}

/// Resolve a benchmark alert.
#[utoipa::path(
    post,
    path = "/api/v1/energy/benchmark/alerts/{id}/resolve",
    params(("id" = Uuid, Path, description = "Alert ID")),
    responses(
        (status = 200, description = "Alert resolved", body = db::models::energy::BenchmarkAlert),
        (status = 404, description = "Not found", body = ErrorResponse),
    ),
    tag = "energy"
)]
async fn resolve_benchmark_alert(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<db::models::energy::BenchmarkAlert>> {
    let repo = &state.energy_repo;

    match repo.resolve_benchmark_alert(id, auth.user_id).await {
        Ok(alert) => Ok(Json(alert)),
        Err(sqlx::Error::RowNotFound) => Err(not_found_error("Alert not found")),
        Err(e) => {
            tracing::error!("Failed to resolve benchmark alert: {:?}", e);
            Err(internal_error("Failed to resolve benchmark alert"))
        }
    }
}
