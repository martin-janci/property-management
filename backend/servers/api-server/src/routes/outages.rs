//! Outage routes (UC-12: Utility Outages).

use crate::state::AppState;
use api_core::extractors::RlsConnection;
use api_core::{AuthUser, TenantExtractor};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post, put},
    Json, Router,
};
use chrono::{DateTime, Utc};
use common::errors::ErrorResponse;
use db::models::{
    outage_commodity, outage_severity, CreateOutage, Outage, OutageDashboard, OutageListQuery,
    OutageStatistics, OutageSummary, OutageWithDetails, UpdateOutage,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

// ============================================================================
// Constants
// ============================================================================

/// Maximum allowed title length (characters).
const MAX_TITLE_LENGTH: usize = 255;

/// Maximum allowed description length (characters).
const MAX_DESCRIPTION_LENGTH: usize = 10_000;

// ============================================================================
// Response Types
// ============================================================================

/// Response for outage creation.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateOutageResponse {
    pub id: Uuid,
    pub message: String,
}

/// Response for outage list with pagination.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct OutageListResponse {
    pub outages: Vec<OutageSummary>,
    /// Number of items in this response.
    pub count: usize,
    /// Total number of items matching the query (for pagination).
    pub total: i64,
}

/// Response for outage details.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct OutageDetailResponse {
    pub outage: OutageWithDetails,
}

/// Response for generic outage action.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct OutageActionResponse {
    pub message: String,
    pub outage: Outage,
}

/// Response for outage statistics.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct OutageStatisticsResponse {
    pub statistics: OutageStatistics,
}

/// Response for outage dashboard.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct OutageDashboardResponse {
    pub dashboard: OutageDashboard,
}

/// Response for unread outages count.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UnreadOutagesCountResponse {
    pub unread_count: i64,
}

// ============================================================================
// Request Types
// ============================================================================

/// Request for creating an outage.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateOutageRequest {
    pub title: String,
    pub description: Option<String>,
    pub commodity: String,
    pub severity: String,
    pub building_ids: Option<Vec<Uuid>>,
    pub scheduled_start: DateTime<Utc>,
    pub scheduled_end: Option<DateTime<Utc>>,
    pub external_reference: Option<String>,
    pub supplier_name: Option<String>,
}

/// Request for updating an outage.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdateOutageRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub commodity: Option<String>,
    pub severity: Option<String>,
    pub building_ids: Option<Vec<Uuid>>,
    pub scheduled_start: Option<DateTime<Utc>>,
    pub scheduled_end: Option<DateTime<Utc>>,
    pub external_reference: Option<String>,
    pub supplier_name: Option<String>,
}

/// Request for starting an outage.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct StartOutageRequest {
    /// Actual start time. Defaults to now if not provided.
    pub actual_start: Option<DateTime<Utc>>,
}

/// Request for resolving an outage.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ResolveOutageRequest {
    /// Actual end time. Defaults to now if not provided.
    pub actual_end: Option<DateTime<Utc>>,
    /// Resolution notes.
    pub resolution_notes: Option<String>,
}

/// Request for cancelling an outage.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CancelOutageRequest {
    /// Reason for cancellation.
    pub reason: Option<String>,
}

/// Query for listing outages.
#[derive(Debug, Serialize, Deserialize, ToSchema, Default, utoipa::IntoParams)]
pub struct ListOutagesQuery {
    pub status: Option<String>,
    pub commodity: Option<String>,
    pub severity: Option<String>,
    pub building_id: Option<Uuid>,
    pub from_date: Option<chrono::NaiveDate>,
    pub to_date: Option<chrono::NaiveDate>,
    pub active_only: Option<bool>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// ============================================================================
// Router
// ============================================================================

/// Create outages router.
pub fn router() -> Router<AppState> {
    Router::new()
        // CRUD
        .route("/", post(create_outage))
        .route("/", get(list_outages))
        .route("/active", get(list_active_outages))
        .route("/{id}", get(get_outage))
        .route("/{id}", put(update_outage))
        .route("/{id}", delete(delete_outage))
        // Status changes
        .route("/{id}/start", post(start_outage))
        .route("/{id}/resolve", post(resolve_outage))
        .route("/{id}/cancel", post(cancel_outage))
        // Read tracking
        .route("/{id}/read", post(mark_read))
        // Statistics & Dashboard
        .route("/statistics", get(get_statistics))
        .route("/dashboard", get(get_dashboard))
        .route("/unread-count", get(get_unread_count))
}

// ============================================================================
// Handlers
// ============================================================================

/// Create a new outage.
///
/// Requires manager-level role (Manager, TechnicalManager, OrgAdmin, or SuperAdmin).
#[utoipa::path(
    post,
    path = "/api/v1/outages",
    request_body = CreateOutageRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 201, description = "Outage created", body = CreateOutageResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - requires manager role", body = ErrorResponse),
    ),
    tag = "Outages"
)]
async fn create_outage(
    State(state): State<AppState>,
    auth: AuthUser,
    tenant: TenantExtractor,
    mut rls: RlsConnection,
    Json(req): Json<CreateOutageRequest>,
) -> Result<(StatusCode, Json<CreateOutageResponse>), (StatusCode, Json<ErrorResponse>)> {
    // Authorization: require manager-level role
    let role = tenant.role;
    if !role.is_manager() {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only managers can create outages",
            )),
        ));
    }

    let created_by = auth.user_id;
    let org_id = tenant.tenant_id;

    // Validate content length
    if req.title.len() > MAX_TITLE_LENGTH {
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "BAD_REQUEST",
                format!(
                    "Title exceeds maximum length of {} characters",
                    MAX_TITLE_LENGTH
                ),
            )),
        ));
    }

    if let Some(ref desc) = req.description {
        if desc.len() > MAX_DESCRIPTION_LENGTH {
            rls.release().await;
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "BAD_REQUEST",
                    format!(
                        "Description exceeds maximum length of {} characters",
                        MAX_DESCRIPTION_LENGTH
                    ),
                )),
            ));
        }
    }

    // Validate commodity
    if !outage_commodity::ALL.contains(&req.commodity.as_str()) {
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "BAD_REQUEST",
                format!(
                    "Invalid commodity. Must be one of: {:?}",
                    outage_commodity::ALL
                ),
            )),
        ));
    }

    // Validate severity
    if !outage_severity::ALL.contains(&req.severity.as_str()) {
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "BAD_REQUEST",
                format!(
                    "Invalid severity. Must be one of: {:?}",
                    outage_severity::ALL
                ),
            )),
        ));
    }

    // Create outage
    let data = CreateOutage {
        organization_id: org_id,
        created_by,
        title: req.title,
        description: req.description,
        commodity: req.commodity,
        severity: req.severity,
        building_ids: req.building_ids.unwrap_or_default(),
        scheduled_start: req.scheduled_start,
        scheduled_end: req.scheduled_end,
        external_reference: req.external_reference,
        supplier_name: req.supplier_name,
    };

    let outage = state
        .outage_repo
        .create_rls(&mut **rls.conn(), data)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", e.to_string())),
            )
        })?;

    rls.release().await;

    Ok((
        StatusCode::CREATED,
        Json(CreateOutageResponse {
            id: outage.id,
            message: "Outage created successfully".to_string(),
        }),
    ))
}

/// List outages with filtering and pagination.
#[utoipa::path(
    get,
    path = "/api/v1/outages",
    params(ListOutagesQuery),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of outages", body = OutageListResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "Outages"
)]
async fn list_outages(
    State(state): State<AppState>,
    _auth: AuthUser,
    _tenant: TenantExtractor,
    mut rls: RlsConnection,
    Query(query): Query<ListOutagesQuery>,
) -> Result<Json<OutageListResponse>, (StatusCode, Json<ErrorResponse>)> {
    let list_query = OutageListQuery {
        status: query.status.map(|s| vec![s]),
        commodity: query.commodity.map(|c| vec![c]),
        severity: query.severity.map(|s| vec![s]),
        building_id: query.building_id,
        from_date: query.from_date,
        to_date: query.to_date,
        active_only: query.active_only,
        limit: query.limit,
        offset: query.offset,
    };

    let outages = state
        .outage_repo
        .list_rls(&mut **rls.conn(), &list_query)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", e.to_string())),
            )
        })?;

    let total = state
        .outage_repo
        .count_rls(&mut **rls.conn(), &list_query)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", e.to_string())),
            )
        })?;

    rls.release().await;

    let count = outages.len();
    Ok(Json(OutageListResponse {
        outages,
        count,
        total,
    }))
}

/// List active outages (planned and ongoing).
#[utoipa::path(
    get,
    path = "/api/v1/outages/active",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of active outages", body = OutageListResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "Outages"
)]
async fn list_active_outages(
    State(state): State<AppState>,
    _auth: AuthUser,
    _tenant: TenantExtractor,
    mut rls: RlsConnection,
) -> Result<Json<OutageListResponse>, (StatusCode, Json<ErrorResponse>)> {
    let list_query = OutageListQuery {
        active_only: Some(true),
        limit: Some(50),
        ..Default::default()
    };

    let outages = state
        .outage_repo
        .list_rls(&mut **rls.conn(), &list_query)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", e.to_string())),
            )
        })?;

    let total = state
        .outage_repo
        .count_rls(&mut **rls.conn(), &list_query)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", e.to_string())),
            )
        })?;

    rls.release().await;

    let count = outages.len();
    Ok(Json(OutageListResponse {
        outages,
        count,
        total,
    }))
}

/// Get outage details by ID.
#[utoipa::path(
    get,
    path = "/api/v1/outages/{id}",
    params(("id" = Uuid, Path, description = "Outage ID")),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Outage details", body = OutageDetailResponse),
        (status = 404, description = "Outage not found", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "Outages"
)]
async fn get_outage(
    State(state): State<AppState>,
    _auth: AuthUser,
    _tenant: TenantExtractor,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<OutageDetailResponse>, (StatusCode, Json<ErrorResponse>)> {
    let outage = state
        .outage_repo
        .find_by_id_with_details_rls(&mut **rls.conn(), id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", e.to_string())),
            )
        })?;

    rls.release().await;

    match outage {
        Some(o) => Ok(Json(OutageDetailResponse { outage: o })),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Outage not found")),
        )),
    }
}

/// Update an outage.
///
/// Requires manager-level role.
#[utoipa::path(
    put,
    path = "/api/v1/outages/{id}",
    params(("id" = Uuid, Path, description = "Outage ID")),
    request_body = UpdateOutageRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Outage updated", body = OutageActionResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Outage not found", body = ErrorResponse),
    ),
    tag = "Outages"
)]
async fn update_outage(
    State(state): State<AppState>,
    _auth: AuthUser,
    tenant: TenantExtractor,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateOutageRequest>,
) -> Result<Json<OutageActionResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Authorization: require manager-level role
    if !tenant.role.is_manager() {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only managers can update outages",
            )),
        ));
    }

    // Validate commodity if provided
    if let Some(ref commodity) = req.commodity {
        if !outage_commodity::ALL.contains(&commodity.as_str()) {
            rls.release().await;
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "BAD_REQUEST",
                    format!(
                        "Invalid commodity. Must be one of: {:?}",
                        outage_commodity::ALL
                    ),
                )),
            ));
        }
    }

    // Validate severity if provided
    if let Some(ref severity) = req.severity {
        if !outage_severity::ALL.contains(&severity.as_str()) {
            rls.release().await;
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "BAD_REQUEST",
                    format!(
                        "Invalid severity. Must be one of: {:?}",
                        outage_severity::ALL
                    ),
                )),
            ));
        }
    }

    let data = UpdateOutage {
        title: req.title,
        description: req.description,
        commodity: req.commodity,
        severity: req.severity,
        building_ids: req.building_ids,
        scheduled_start: req.scheduled_start,
        scheduled_end: req.scheduled_end,
        external_reference: req.external_reference,
        supplier_name: req.supplier_name,
    };

    let outage = state
        .outage_repo
        .update_rls(&mut **rls.conn(), id, data)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", e.to_string())),
            )
        })?;

    rls.release().await;

    Ok(Json(OutageActionResponse {
        message: "Outage updated successfully".to_string(),
        outage,
    }))
}

/// Delete an outage.
///
/// Requires manager-level role.
#[utoipa::path(
    delete,
    path = "/api/v1/outages/{id}",
    params(("id" = Uuid, Path, description = "Outage ID")),
    security(("bearer_auth" = [])),
    responses(
        (status = 204, description = "Outage deleted"),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Outage not found", body = ErrorResponse),
    ),
    tag = "Outages"
)]
async fn delete_outage(
    State(state): State<AppState>,
    _auth: AuthUser,
    tenant: TenantExtractor,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    // Authorization: require manager-level role
    if !tenant.role.is_manager() {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only managers can delete outages",
            )),
        ));
    }

    state
        .outage_repo
        .delete_rls(&mut **rls.conn(), id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", e.to_string())),
            )
        })?;

    rls.release().await;

    Ok(StatusCode::NO_CONTENT)
}

/// Start an outage (change status from planned to ongoing).
///
/// Requires manager-level role.
#[utoipa::path(
    post,
    path = "/api/v1/outages/{id}/start",
    params(("id" = Uuid, Path, description = "Outage ID")),
    request_body = StartOutageRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Outage started", body = OutageActionResponse),
        (status = 400, description = "Invalid state transition", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Outage not found", body = ErrorResponse),
    ),
    tag = "Outages"
)]
async fn start_outage(
    State(state): State<AppState>,
    _auth: AuthUser,
    tenant: TenantExtractor,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<StartOutageRequest>,
) -> Result<Json<OutageActionResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Authorization: require manager-level role
    if !tenant.role.is_manager() {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only managers can start outages",
            )),
        ));
    }

    let outage = state
        .outage_repo
        .start_rls(&mut **rls.conn(), id, req.actual_start)
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new("BAD_REQUEST", e.to_string())),
            )
        })?;

    rls.release().await;

    Ok(Json(OutageActionResponse {
        message: "Outage started".to_string(),
        outage,
    }))
}

/// Resolve an outage.
///
/// Requires manager-level role.
#[utoipa::path(
    post,
    path = "/api/v1/outages/{id}/resolve",
    params(("id" = Uuid, Path, description = "Outage ID")),
    request_body = ResolveOutageRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Outage resolved", body = OutageActionResponse),
        (status = 400, description = "Invalid state transition", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Outage not found", body = ErrorResponse),
    ),
    tag = "Outages"
)]
async fn resolve_outage(
    State(state): State<AppState>,
    _auth: AuthUser,
    tenant: TenantExtractor,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<ResolveOutageRequest>,
) -> Result<Json<OutageActionResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Authorization: require manager-level role
    if !tenant.role.is_manager() {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only managers can resolve outages",
            )),
        ));
    }

    let outage = state
        .outage_repo
        .resolve_rls(&mut **rls.conn(), id, req.actual_end, req.resolution_notes)
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new("BAD_REQUEST", e.to_string())),
            )
        })?;

    rls.release().await;

    Ok(Json(OutageActionResponse {
        message: "Outage resolved".to_string(),
        outage,
    }))
}

/// Cancel an outage.
///
/// Requires manager-level role.
#[utoipa::path(
    post,
    path = "/api/v1/outages/{id}/cancel",
    params(("id" = Uuid, Path, description = "Outage ID")),
    request_body = CancelOutageRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Outage cancelled", body = OutageActionResponse),
        (status = 400, description = "Invalid state transition", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Outage not found", body = ErrorResponse),
    ),
    tag = "Outages"
)]
async fn cancel_outage(
    State(state): State<AppState>,
    _auth: AuthUser,
    tenant: TenantExtractor,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<CancelOutageRequest>,
) -> Result<Json<OutageActionResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Authorization: require manager-level role
    if !tenant.role.is_manager() {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only managers can cancel outages",
            )),
        ));
    }

    let outage = state
        .outage_repo
        .cancel_rls(&mut **rls.conn(), id, req.reason)
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new("BAD_REQUEST", e.to_string())),
            )
        })?;

    rls.release().await;

    Ok(Json(OutageActionResponse {
        message: "Outage cancelled".to_string(),
        outage,
    }))
}

/// Mark an outage as read.
#[utoipa::path(
    post,
    path = "/api/v1/outages/{id}/read",
    params(("id" = Uuid, Path, description = "Outage ID")),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Outage marked as read"),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "Outages"
)]
async fn mark_read(
    State(state): State<AppState>,
    auth: AuthUser,
    _tenant: TenantExtractor,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    state
        .outage_repo
        .mark_read_rls(&mut **rls.conn(), id, auth.user_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", e.to_string())),
            )
        })?;

    rls.release().await;

    Ok(StatusCode::OK)
}

/// Get outage statistics.
#[utoipa::path(
    get,
    path = "/api/v1/outages/statistics",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Outage statistics", body = OutageStatisticsResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "Outages"
)]
async fn get_statistics(
    State(state): State<AppState>,
    _auth: AuthUser,
    _tenant: TenantExtractor,
    mut rls: RlsConnection,
) -> Result<Json<OutageStatisticsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let statistics = state
        .outage_repo
        .get_statistics_rls(&mut **rls.conn())
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", e.to_string())),
            )
        })?;

    rls.release().await;

    Ok(Json(OutageStatisticsResponse { statistics }))
}

/// Get outage dashboard.
#[utoipa::path(
    get,
    path = "/api/v1/outages/dashboard",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Outage dashboard", body = OutageDashboardResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "Outages"
)]
async fn get_dashboard(
    State(state): State<AppState>,
    _auth: AuthUser,
    _tenant: TenantExtractor,
    mut rls: RlsConnection,
) -> Result<Json<OutageDashboardResponse>, (StatusCode, Json<ErrorResponse>)> {
    let dashboard = state
        .outage_repo
        .get_dashboard_rls(&mut **rls.conn())
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", e.to_string())),
            )
        })?;

    rls.release().await;

    Ok(Json(OutageDashboardResponse { dashboard }))
}

/// Get unread outages count for current user.
#[utoipa::path(
    get,
    path = "/api/v1/outages/unread-count",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Unread outages count", body = UnreadOutagesCountResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "Outages"
)]
async fn get_unread_count(
    State(state): State<AppState>,
    auth: AuthUser,
    _tenant: TenantExtractor,
    mut rls: RlsConnection,
) -> Result<Json<UnreadOutagesCountResponse>, (StatusCode, Json<ErrorResponse>)> {
    let unread_count = state
        .outage_repo
        .count_unread_rls(&mut **rls.conn(), auth.user_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", e.to_string())),
            )
        })?;

    rls.release().await;

    Ok(Json(UnreadOutagesCountResponse { unread_count }))
}
