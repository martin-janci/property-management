//! Meter routes (Epic 12).
//!
//! Implements meter management, readings, utility bills,
//! and consumption tracking.

use api_core::extractors::AuthUser;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post, put},
    Json, Router,
};
use chrono::{DateTime, NaiveDate, Utc};
use common::errors::ErrorResponse;
use db::models::{
    CreateSubmissionWindow, CreateUtilityBill, DistributeUtilityBill, RegisterMeter, ReplaceMeter,
    SubmitReading, ValidateReading,
};
use rust_decimal::Decimal;
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::state::AppState;

/// Default page size for listings
const DEFAULT_LIST_LIMIT: i64 = 50;

/// Maximum page size
const MAX_LIST_LIMIT: i64 = 100;

type ApiResult<T> = Result<T, (StatusCode, Json<ErrorResponse>)>;

/// Create meters router.
pub fn router() -> Router<AppState> {
    Router::new()
        // Meters (Story 12.1)
        .route("/", post(register_meter))
        .route("/buildings/{building_id}", get(list_meters))
        .route("/{id}", get(get_meter))
        .route("/{id}/replace", post(replace_meter))
        .route("/units/{unit_id}", get(list_unit_meters))
        // Readings (Story 12.2)
        .route("/readings", post(submit_reading))
        .route("/readings/{id}", get(get_reading))
        .route("/{meter_id}/readings", get(list_readings))
        // Submission windows
        .route("/submission-windows", post(create_submission_window))
        .route(
            "/submission-windows/open/{building_id}",
            get(get_open_window),
        )
        // Validation (Story 12.3)
        .route("/readings/{id}/validate", put(validate_reading))
        .route("/readings/pending", get(get_pending_readings))
        .route("/validation-rules", get(get_validation_rules))
        // Utility bills (Story 12.4)
        .route("/utility-bills", post(create_utility_bill))
        .route("/utility-bills/{id}", get(get_utility_bill))
        .route("/utility-bills/{id}/distribute", post(distribute_bill))
        // Consumption analytics (Story 12.5)
        .route("/{meter_id}/consumption", get(get_consumption_history))
        .route("/{meter_id}/aggregates", get(get_consumption_aggregates))
        // Smart meters (Story 12.6)
        .route("/providers", get(list_providers))
        .route("/providers/{id}", get(get_provider))
        .route("/ingest", post(ingest_smart_reading))
        .route("/alerts", get(list_missing_alerts))
        .route("/alerts/{id}/resolve", post(resolve_alert))
}

// ==================== Request/Response Types ====================

/// Register meter request with organization context.
#[derive(Debug, Deserialize, ToSchema)]
pub struct RegisterMeterRequest {
    /// Organization ID
    pub organization_id: Uuid,
    /// Meter data
    #[serde(flatten)]
    pub data: RegisterMeter,
}

/// Replace meter request with organization context.
#[derive(Debug, Deserialize, ToSchema)]
pub struct ReplaceMeterRequest {
    /// Organization ID
    pub organization_id: Uuid,
    /// Replacement data
    #[serde(flatten)]
    pub data: ReplaceMeter,
}

/// Create submission window request with organization context.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateSubmissionWindowRequest {
    /// Organization ID
    pub organization_id: Uuid,
    /// Window data
    #[serde(flatten)]
    pub data: CreateSubmissionWindow,
}

/// Create utility bill request with organization context.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateUtilityBillRequest {
    /// Organization ID
    pub organization_id: Uuid,
    /// Bill data
    #[serde(flatten)]
    pub data: CreateUtilityBill,
}

/// List meters query parameters.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListMetersQuery {
    /// Page limit
    #[serde(default = "default_limit")]
    pub limit: i64,
    /// Page offset
    #[serde(default)]
    pub offset: i64,
}

/// List readings query parameters.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListReadingsQuery {
    /// Start date filter
    pub from: Option<NaiveDate>,
    /// End date filter
    pub to: Option<NaiveDate>,
    /// Page limit
    #[serde(default = "default_limit")]
    pub limit: i64,
    /// Page offset
    #[serde(default)]
    pub offset: i64,
}

/// Pending readings query.
#[derive(Debug, Deserialize, IntoParams)]
pub struct PendingReadingsQuery {
    /// Organization ID
    pub organization_id: Uuid,
    /// Filter by building
    pub building_id: Option<Uuid>,
}

/// Validation rules query.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ValidationRulesQuery {
    /// Organization ID
    pub organization_id: Uuid,
}

/// Consumption history query.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ConsumptionQuery {
    /// Start date
    pub from: NaiveDate,
    /// End date
    pub to: NaiveDate,
}

/// Consumption aggregates query.
#[derive(Debug, Deserialize, IntoParams)]
pub struct AggregatesQuery {
    /// Year to fetch
    pub year: i32,
}

/// Missing alerts query.
#[derive(Debug, Deserialize, IntoParams)]
pub struct AlertsQuery {
    /// Organization ID
    pub organization_id: Uuid,
    /// Only unresolved
    #[serde(default = "default_true")]
    pub unresolved_only: bool,
}

/// Providers query.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ProvidersQuery {
    /// Organization ID
    pub organization_id: Uuid,
}

/// Resolve alert request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct ResolveAlertRequest {
    /// Resolution notes
    pub notes: Option<String>,
}

/// Smart meter reading ingestion request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct IngestReadingRequest {
    /// Organization ID
    pub organization_id: Uuid,
    /// Provider ID
    pub provider_id: Uuid,
    /// Meter number
    pub meter_number: String,
    /// Reading value
    pub reading: Decimal,
    /// Reading timestamp
    pub reading_timestamp: DateTime<Utc>,
    /// Raw data from provider
    pub raw_data: Option<serde_json::Value>,
}

fn default_limit() -> i64 {
    DEFAULT_LIST_LIMIT
}

fn default_true() -> bool {
    true
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

// ==================== Meter Handlers ====================

/// Register a new meter.
#[utoipa::path(
    post,
    path = "/api/v1/meters",
    request_body = RegisterMeterRequest,
    responses(
        (status = 201, description = "Meter registered", body = db::models::Meter),
        (status = 400, description = "Invalid request", body = ErrorResponse),
    ),
    tag = "meters"
)]
async fn register_meter(
    State(state): State<AppState>,
    _auth: AuthUser,
    Json(payload): Json<RegisterMeterRequest>,
) -> ApiResult<(StatusCode, Json<db::models::Meter>)> {
    let repo = &state.meter_repo;

    repo.register_meter(payload.organization_id, payload.data)
        .await
        .map(|meter| (StatusCode::CREATED, Json(meter)))
        .map_err(|e| {
            tracing::error!("Failed to register meter: {:?}", e);
            internal_error("Failed to register meter")
        })
}

/// List meters for a building.
#[utoipa::path(
    get,
    path = "/api/v1/meters/buildings/{building_id}",
    params(("building_id" = Uuid, Path, description = "Building ID"), ListMetersQuery),
    responses(
        (status = 200, description = "Meters list", body = db::models::ListMetersResponse),
    ),
    tag = "meters"
)]
async fn list_meters(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(building_id): Path<Uuid>,
    Query(query): Query<ListMetersQuery>,
) -> ApiResult<Json<db::models::ListMetersResponse>> {
    let repo = &state.meter_repo;
    let limit = query.limit.min(MAX_LIST_LIMIT);

    repo.list_meters_for_building(building_id, limit, query.offset)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to list meters: {:?}", e);
            internal_error("Failed to list meters")
        })
}

/// Get a meter with recent readings.
#[utoipa::path(
    get,
    path = "/api/v1/meters/{id}",
    params(("id" = Uuid, Path, description = "Meter ID")),
    responses(
        (status = 200, description = "Meter details", body = db::models::MeterResponse),
        (status = 404, description = "Not found", body = ErrorResponse),
    ),
    tag = "meters"
)]
async fn get_meter(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<db::models::MeterResponse>> {
    let repo = &state.meter_repo;

    match repo.get_meter_with_readings(id, 10).await {
        Ok(Some(meter)) => Ok(Json(meter)),
        Ok(None) => Err(not_found_error("Meter not found")),
        Err(e) => {
            tracing::error!("Failed to get meter: {:?}", e);
            Err(internal_error("Failed to get meter"))
        }
    }
}

/// Replace a meter.
#[utoipa::path(
    post,
    path = "/api/v1/meters/{id}/replace",
    params(("id" = Uuid, Path, description = "Meter ID to replace")),
    request_body = ReplaceMeterRequest,
    responses(
        (status = 201, description = "New meter created", body = db::models::Meter),
        (status = 404, description = "Original meter not found", body = ErrorResponse),
    ),
    tag = "meters"
)]
async fn replace_meter(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(payload): Json<ReplaceMeterRequest>,
) -> ApiResult<(StatusCode, Json<db::models::Meter>)> {
    let repo = &state.meter_repo;

    match repo
        .replace_meter(id, payload.organization_id, payload.data)
        .await
    {
        Ok(meter) => Ok((StatusCode::CREATED, Json(meter))),
        Err(sqlx::Error::RowNotFound) => Err(not_found_error("Meter not found")),
        Err(e) => {
            tracing::error!("Failed to replace meter: {:?}", e);
            Err(internal_error("Failed to replace meter"))
        }
    }
}

/// List meters for a unit.
#[utoipa::path(
    get,
    path = "/api/v1/meters/units/{unit_id}",
    params(("unit_id" = Uuid, Path, description = "Unit ID")),
    responses(
        (status = 200, description = "Meters list", body = Vec<db::models::Meter>),
    ),
    tag = "meters"
)]
async fn list_unit_meters(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(unit_id): Path<Uuid>,
) -> ApiResult<Json<Vec<db::models::Meter>>> {
    let repo = &state.meter_repo;

    repo.list_meters_for_unit(unit_id)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to list unit meters: {:?}", e);
            internal_error("Failed to list meters")
        })
}

// ==================== Reading Handlers ====================

/// Submit a meter reading.
#[utoipa::path(
    post,
    path = "/api/v1/meters/readings",
    request_body = SubmitReading,
    responses(
        (status = 201, description = "Reading submitted", body = db::models::MeterReading),
        (status = 400, description = "Invalid request", body = ErrorResponse),
    ),
    tag = "meters"
)]
async fn submit_reading(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<SubmitReading>,
) -> ApiResult<(StatusCode, Json<db::models::MeterReading>)> {
    let repo = &state.meter_repo;

    repo.submit_reading(auth.user_id, payload)
        .await
        .map(|reading| (StatusCode::CREATED, Json(reading)))
        .map_err(|e| {
            tracing::error!("Failed to submit reading: {:?}", e);
            internal_error("Failed to submit reading")
        })
}

/// Get a reading by ID.
#[utoipa::path(
    get,
    path = "/api/v1/meters/readings/{id}",
    params(("id" = Uuid, Path, description = "Reading ID")),
    responses(
        (status = 200, description = "Reading details", body = db::models::MeterReading),
        (status = 404, description = "Not found", body = ErrorResponse),
    ),
    tag = "meters"
)]
async fn get_reading(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<db::models::MeterReading>> {
    let repo = &state.meter_repo;

    match repo.get_reading(id).await {
        Ok(Some(reading)) => Ok(Json(reading)),
        Ok(None) => Err(not_found_error("Reading not found")),
        Err(e) => {
            tracing::error!("Failed to get reading: {:?}", e);
            Err(internal_error("Failed to get reading"))
        }
    }
}

/// List readings for a meter.
#[utoipa::path(
    get,
    path = "/api/v1/meters/{meter_id}/readings",
    params(
        ("meter_id" = Uuid, Path, description = "Meter ID"),
        ListReadingsQuery
    ),
    responses(
        (status = 200, description = "Readings list", body = db::models::ListReadingsResponse),
    ),
    tag = "meters"
)]
async fn list_readings(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(meter_id): Path<Uuid>,
    Query(query): Query<ListReadingsQuery>,
) -> ApiResult<Json<db::models::ListReadingsResponse>> {
    let repo = &state.meter_repo;
    let limit = query.limit.min(MAX_LIST_LIMIT);

    repo.list_readings_for_meter(meter_id, query.from, query.to, limit, query.offset)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to list readings: {:?}", e);
            internal_error("Failed to list readings")
        })
}

// ==================== Submission Window Handlers ====================

/// Create a submission window.
#[utoipa::path(
    post,
    path = "/api/v1/meters/submission-windows",
    request_body = CreateSubmissionWindowRequest,
    responses(
        (status = 201, description = "Window created", body = db::models::ReadingSubmissionWindow),
        (status = 400, description = "Invalid request", body = ErrorResponse),
    ),
    tag = "meters"
)]
async fn create_submission_window(
    State(state): State<AppState>,
    _auth: AuthUser,
    Json(payload): Json<CreateSubmissionWindowRequest>,
) -> ApiResult<(StatusCode, Json<db::models::ReadingSubmissionWindow>)> {
    let repo = &state.meter_repo;

    repo.create_submission_window(payload.organization_id, payload.data)
        .await
        .map(|window| (StatusCode::CREATED, Json(window)))
        .map_err(|e| {
            tracing::error!("Failed to create submission window: {:?}", e);
            internal_error("Failed to create window")
        })
}

/// Get open submission window for a building.
#[utoipa::path(
    get,
    path = "/api/v1/meters/submission-windows/open/{building_id}",
    params(("building_id" = Uuid, Path, description = "Building ID")),
    responses(
        (status = 200, description = "Open window", body = db::models::ReadingSubmissionWindow),
        (status = 404, description = "No open window", body = ErrorResponse),
    ),
    tag = "meters"
)]
async fn get_open_window(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(building_id): Path<Uuid>,
) -> ApiResult<Json<db::models::ReadingSubmissionWindow>> {
    let repo = &state.meter_repo;

    match repo.get_open_submission_window(building_id).await {
        Ok(Some(window)) => Ok(Json(window)),
        Ok(None) => Err(not_found_error("No open submission window")),
        Err(e) => {
            tracing::error!("Failed to get submission window: {:?}", e);
            Err(internal_error("Failed to get window"))
        }
    }
}

// ==================== Validation Handlers ====================

/// Validate a reading (approve/reject).
#[utoipa::path(
    put,
    path = "/api/v1/meters/readings/{id}/validate",
    params(("id" = Uuid, Path, description = "Reading ID")),
    request_body = ValidateReading,
    responses(
        (status = 200, description = "Reading validated", body = db::models::MeterReading),
        (status = 404, description = "Not found", body = ErrorResponse),
    ),
    tag = "meters"
)]
async fn validate_reading(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(payload): Json<ValidateReading>,
) -> ApiResult<Json<db::models::MeterReading>> {
    let repo = &state.meter_repo;

    match repo.validate_reading(id, auth.user_id, payload).await {
        Ok(Some(reading)) => Ok(Json(reading)),
        Ok(None) => Err(not_found_error("Reading not found")),
        Err(e) => {
            tracing::error!("Failed to validate reading: {:?}", e);
            Err(internal_error("Failed to validate reading"))
        }
    }
}

/// Get readings pending validation.
#[utoipa::path(
    get,
    path = "/api/v1/meters/readings/pending",
    params(PendingReadingsQuery),
    responses(
        (status = 200, description = "Pending readings", body = Vec<db::models::MeterReading>),
    ),
    tag = "meters"
)]
async fn get_pending_readings(
    State(state): State<AppState>,
    _auth: AuthUser,
    Query(query): Query<PendingReadingsQuery>,
) -> ApiResult<Json<Vec<db::models::MeterReading>>> {
    let repo = &state.meter_repo;

    repo.get_pending_readings(query.organization_id, query.building_id)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to get pending readings: {:?}", e);
            internal_error("Failed to get pending readings")
        })
}

/// Get validation rules.
#[utoipa::path(
    get,
    path = "/api/v1/meters/validation-rules",
    params(ValidationRulesQuery),
    responses(
        (status = 200, description = "Validation rules", body = Vec<db::models::ReadingValidationRule>),
    ),
    tag = "meters"
)]
async fn get_validation_rules(
    State(state): State<AppState>,
    _auth: AuthUser,
    Query(query): Query<ValidationRulesQuery>,
) -> ApiResult<Json<Vec<db::models::ReadingValidationRule>>> {
    let repo = &state.meter_repo;

    repo.get_validation_rules(query.organization_id)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to get validation rules: {:?}", e);
            internal_error("Failed to get validation rules")
        })
}

// ==================== Utility Bill Handlers ====================

/// Create a utility bill.
#[utoipa::path(
    post,
    path = "/api/v1/meters/utility-bills",
    request_body = CreateUtilityBillRequest,
    responses(
        (status = 201, description = "Bill created", body = db::models::UtilityBill),
        (status = 400, description = "Invalid request", body = ErrorResponse),
    ),
    tag = "meters"
)]
async fn create_utility_bill(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<CreateUtilityBillRequest>,
) -> ApiResult<(StatusCode, Json<db::models::UtilityBill>)> {
    let repo = &state.meter_repo;

    repo.create_utility_bill(payload.organization_id, auth.user_id, payload.data)
        .await
        .map(|bill| (StatusCode::CREATED, Json(bill)))
        .map_err(|e| {
            tracing::error!("Failed to create utility bill: {:?}", e);
            internal_error("Failed to create utility bill")
        })
}

/// Get a utility bill with distributions.
#[utoipa::path(
    get,
    path = "/api/v1/meters/utility-bills/{id}",
    params(("id" = Uuid, Path, description = "Bill ID")),
    responses(
        (status = 200, description = "Bill details", body = db::models::UtilityBillResponse),
        (status = 404, description = "Not found", body = ErrorResponse),
    ),
    tag = "meters"
)]
async fn get_utility_bill(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<db::models::UtilityBillResponse>> {
    let repo = &state.meter_repo;

    match repo.get_utility_bill_with_distributions(id).await {
        Ok(Some(bill)) => Ok(Json(bill)),
        Ok(None) => Err(not_found_error("Utility bill not found")),
        Err(e) => {
            tracing::error!("Failed to get utility bill: {:?}", e);
            Err(internal_error("Failed to get utility bill"))
        }
    }
}

/// Distribute a utility bill to units.
#[utoipa::path(
    post,
    path = "/api/v1/meters/utility-bills/{id}/distribute",
    params(("id" = Uuid, Path, description = "Bill ID")),
    request_body = DistributeUtilityBill,
    responses(
        (status = 200, description = "Bill distributed", body = db::models::UtilityBillResponse),
        (status = 404, description = "Not found", body = ErrorResponse),
    ),
    tag = "meters"
)]
async fn distribute_bill(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(payload): Json<DistributeUtilityBill>,
) -> ApiResult<Json<db::models::UtilityBillResponse>> {
    let repo = &state.meter_repo;

    match repo
        .distribute_utility_bill(id, auth.user_id, payload)
        .await
    {
        Ok(response) => Ok(Json(response)),
        Err(sqlx::Error::RowNotFound) => Err(not_found_error("Utility bill not found")),
        Err(e) => {
            tracing::error!("Failed to distribute utility bill: {:?}", e);
            Err(internal_error("Failed to distribute utility bill"))
        }
    }
}

// ==================== Consumption Analytics Handlers ====================

/// Get consumption history for a meter.
#[utoipa::path(
    get,
    path = "/api/v1/meters/{meter_id}/consumption",
    params(("meter_id" = Uuid, Path, description = "Meter ID"), ConsumptionQuery),
    responses(
        (status = 200, description = "Consumption history", body = db::models::ConsumptionHistoryResponse),
        (status = 404, description = "Meter not found", body = ErrorResponse),
    ),
    tag = "meters"
)]
async fn get_consumption_history(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(meter_id): Path<Uuid>,
    Query(query): Query<ConsumptionQuery>,
) -> ApiResult<Json<db::models::ConsumptionHistoryResponse>> {
    let repo = &state.meter_repo;

    match repo
        .get_consumption_history(meter_id, query.from, query.to)
        .await
    {
        Ok(history) => Ok(Json(history)),
        Err(sqlx::Error::RowNotFound) => Err(not_found_error("Meter not found")),
        Err(e) => {
            tracing::error!("Failed to get consumption history: {:?}", e);
            Err(internal_error("Failed to get consumption history"))
        }
    }
}

/// Get consumption aggregates for a meter.
#[utoipa::path(
    get,
    path = "/api/v1/meters/{meter_id}/aggregates",
    params(("meter_id" = Uuid, Path, description = "Meter ID"), AggregatesQuery),
    responses(
        (status = 200, description = "Consumption aggregates", body = Vec<db::models::ConsumptionAggregate>),
    ),
    tag = "meters"
)]
async fn get_consumption_aggregates(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(meter_id): Path<Uuid>,
    Query(query): Query<AggregatesQuery>,
) -> ApiResult<Json<Vec<db::models::ConsumptionAggregate>>> {
    let repo = &state.meter_repo;

    repo.get_consumption_aggregates(meter_id, query.year)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to get consumption aggregates: {:?}", e);
            internal_error("Failed to get aggregates")
        })
}

// ==================== Smart Meter Handlers ====================

/// List smart meter providers.
#[utoipa::path(
    get,
    path = "/api/v1/meters/providers",
    params(ProvidersQuery),
    responses(
        (status = 200, description = "Providers list", body = Vec<db::models::SmartMeterProvider>),
    ),
    tag = "meters"
)]
async fn list_providers(
    State(state): State<AppState>,
    _auth: AuthUser,
    Query(query): Query<ProvidersQuery>,
) -> ApiResult<Json<Vec<db::models::SmartMeterProvider>>> {
    let repo = &state.meter_repo;

    repo.get_smart_meter_providers(query.organization_id)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to list providers: {:?}", e);
            internal_error("Failed to list providers")
        })
}

/// Get a smart meter provider.
#[utoipa::path(
    get,
    path = "/api/v1/meters/providers/{id}",
    params(("id" = Uuid, Path, description = "Provider ID")),
    responses(
        (status = 200, description = "Provider details", body = db::models::SmartMeterProvider),
        (status = 404, description = "Not found", body = ErrorResponse),
    ),
    tag = "meters"
)]
async fn get_provider(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<db::models::SmartMeterProvider>> {
    let repo = &state.meter_repo;

    match repo.get_smart_meter_provider(id).await {
        Ok(Some(provider)) => Ok(Json(provider)),
        Ok(None) => Err(not_found_error("Provider not found")),
        Err(e) => {
            tracing::error!("Failed to get provider: {:?}", e);
            Err(internal_error("Failed to get provider"))
        }
    }
}

/// Ingest smart meter reading.
#[utoipa::path(
    post,
    path = "/api/v1/meters/ingest",
    request_body = IngestReadingRequest,
    responses(
        (status = 201, description = "Reading ingested"),
        (status = 400, description = "Invalid request", body = ErrorResponse),
    ),
    tag = "meters"
)]
async fn ingest_smart_reading(
    State(state): State<AppState>,
    _auth: AuthUser,
    Json(payload): Json<IngestReadingRequest>,
) -> ApiResult<StatusCode> {
    let repo = &state.meter_repo;

    // Find meter by number
    let meter = match repo
        .get_meter_by_number(payload.organization_id, &payload.meter_number)
        .await
    {
        Ok(Some(m)) => m,
        Ok(None) => return Err(not_found_error("Meter not found")),
        Err(e) => {
            tracing::error!("Failed to find meter: {:?}", e);
            return Err(internal_error("Failed to find meter"));
        }
    };

    repo.ingest_smart_meter_reading(
        payload.provider_id,
        meter.id,
        payload.reading,
        payload.reading_timestamp,
        payload.raw_data,
    )
    .await
    .map(|_| StatusCode::CREATED)
    .map_err(|e| {
        tracing::error!("Failed to ingest reading: {:?}", e);
        internal_error("Failed to ingest reading")
    })
}

/// List missing reading alerts.
#[utoipa::path(
    get,
    path = "/api/v1/meters/alerts",
    params(AlertsQuery),
    responses(
        (status = 200, description = "Alerts list", body = Vec<db::models::MissingReadingAlert>),
    ),
    tag = "meters"
)]
async fn list_missing_alerts(
    State(state): State<AppState>,
    _auth: AuthUser,
    Query(query): Query<AlertsQuery>,
) -> ApiResult<Json<Vec<db::models::MissingReadingAlert>>> {
    let repo = &state.meter_repo;

    repo.get_missing_reading_alerts(query.organization_id, query.unresolved_only)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to list alerts: {:?}", e);
            internal_error("Failed to list alerts")
        })
}

/// Resolve a missing reading alert.
#[utoipa::path(
    post,
    path = "/api/v1/meters/alerts/{id}/resolve",
    params(("id" = Uuid, Path, description = "Alert ID")),
    request_body = ResolveAlertRequest,
    responses(
        (status = 200, description = "Alert resolved", body = db::models::MissingReadingAlert),
        (status = 404, description = "Not found", body = ErrorResponse),
    ),
    tag = "meters"
)]
async fn resolve_alert(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(payload): Json<ResolveAlertRequest>,
) -> ApiResult<Json<db::models::MissingReadingAlert>> {
    let repo = &state.meter_repo;

    match repo
        .resolve_missing_alert(id, auth.user_id, payload.notes.as_deref())
        .await
    {
        Ok(Some(alert)) => Ok(Json(alert)),
        Ok(None) => Err(not_found_error("Alert not found")),
        Err(e) => {
            tracing::error!("Failed to resolve alert: {:?}", e);
            Err(internal_error("Failed to resolve alert"))
        }
    }
}
