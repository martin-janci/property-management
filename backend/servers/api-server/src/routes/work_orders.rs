//! Work orders and maintenance scheduling routes (Epic 20).

use api_core::extractors::AuthUser;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, patch, post},
    Json, Router,
};
use chrono::NaiveDate;
use common::errors::ErrorResponse;
use db::models::{
    AddWorkOrderUpdate, CreateMaintenanceSchedule, CreateWorkOrder, MaintenanceCostSummary,
    MaintenanceSchedule, ScheduleExecution, ScheduleQuery, ServiceHistoryEntry, UpcomingSchedule,
    UpdateMaintenanceSchedule, UpdateWorkOrder, WorkOrder, WorkOrderQuery, WorkOrderStatistics,
    WorkOrderUpdate, WorkOrderWithDetails,
};
use serde::Deserialize;
use utoipa::IntoParams;
use uuid::Uuid;

use crate::state::AppState;

/// Create work orders router.
pub fn router() -> Router<AppState> {
    Router::new()
        // Work Orders
        .route("/", post(create_work_order))
        .route("/", get(list_work_orders))
        .route("/with-details", get(list_work_orders_with_details))
        .route("/statistics", get(get_statistics))
        .route("/overdue", get(list_overdue))
        .route("/{id}", get(get_work_order))
        .route("/{id}", patch(update_work_order))
        .route("/{id}", delete(delete_work_order))
        .route("/{id}/assign", post(assign_work_order))
        .route("/{id}/start", post(start_work))
        .route("/{id}/complete", post(complete_work_order))
        .route("/{id}/hold", post(put_on_hold))
        .route("/{id}/comments", post(add_comment))
        .route("/{id}/comments", get(list_comments))
        // Maintenance Schedules
        .route("/schedules", post(create_schedule))
        .route("/schedules", get(list_schedules))
        .route("/schedules/upcoming", get(get_upcoming_schedules))
        .route("/schedules/process-due", post(process_due_schedules))
        .route("/schedules/{id}", get(get_schedule))
        .route("/schedules/{id}", patch(update_schedule))
        .route("/schedules/{id}", delete(delete_schedule))
        .route("/schedules/{id}/activate", post(activate_schedule))
        .route("/schedules/{id}/deactivate", post(deactivate_schedule))
        .route("/schedules/{id}/skip", post(skip_schedule))
        .route("/schedules/{id}/executions", get(list_executions))
        // Service History
        .route(
            "/equipment/{equipment_id}/service-history",
            get(get_equipment_service_history),
        )
        .route(
            "/buildings/{building_id}/service-history",
            get(get_building_service_history),
        )
        .route("/cost-summary", get(get_cost_summary))
}

// ==================== Request/Response Types ====================

/// Organization query parameter.
#[derive(Debug, Deserialize, IntoParams)]
pub struct OrgQuery {
    pub organization_id: Uuid,
}

/// Create work order request.
#[derive(Debug, Deserialize)]
pub struct CreateWorkOrderRequest {
    pub organization_id: Uuid,
    #[serde(flatten)]
    pub data: CreateWorkOrder,
}

/// List work orders query.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListWorkOrdersQuery {
    pub organization_id: Uuid,
    pub building_id: Option<Uuid>,
    pub equipment_id: Option<Uuid>,
    pub fault_id: Option<Uuid>,
    pub assigned_to: Option<Uuid>,
    pub vendor_id: Option<Uuid>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub work_type: Option<String>,
    pub source: Option<String>,
    pub due_before: Option<NaiveDate>,
    pub due_after: Option<NaiveDate>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

impl From<&ListWorkOrdersQuery> for WorkOrderQuery {
    fn from(q: &ListWorkOrdersQuery) -> Self {
        WorkOrderQuery {
            building_id: q.building_id,
            equipment_id: q.equipment_id,
            fault_id: q.fault_id,
            assigned_to: q.assigned_to,
            vendor_id: q.vendor_id,
            status: q.status.clone(),
            priority: q.priority.clone(),
            work_type: q.work_type.clone(),
            source: q.source.clone(),
            due_before: q.due_before,
            due_after: q.due_after,
            limit: q.limit,
            offset: q.offset,
        }
    }
}

/// Create schedule request.
#[derive(Debug, Deserialize)]
pub struct CreateScheduleRequest {
    pub organization_id: Uuid,
    #[serde(flatten)]
    pub data: CreateMaintenanceSchedule,
}

/// List schedules query.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListSchedulesQuery {
    pub organization_id: Uuid,
    pub building_id: Option<Uuid>,
    pub equipment_id: Option<Uuid>,
    pub frequency: Option<String>,
    pub is_active: Option<bool>,
    pub due_before: Option<NaiveDate>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

impl From<&ListSchedulesQuery> for ScheduleQuery {
    fn from(q: &ListSchedulesQuery) -> Self {
        ScheduleQuery {
            building_id: q.building_id,
            equipment_id: q.equipment_id,
            frequency: q.frequency.clone(),
            is_active: q.is_active,
            due_before: q.due_before,
            limit: q.limit,
            offset: q.offset,
        }
    }
}

/// Upcoming schedules query.
#[derive(Debug, Deserialize, IntoParams)]
pub struct UpcomingQuery {
    pub organization_id: Uuid,
    pub days_ahead: Option<i32>,
    pub limit: Option<i32>,
}

/// Pagination query.
#[derive(Debug, Deserialize, IntoParams)]
pub struct PaginationQuery {
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

/// Cost summary query.
#[derive(Debug, Deserialize, IntoParams)]
pub struct CostSummaryQuery {
    pub organization_id: Uuid,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
}

/// Assign request.
#[derive(Debug, Deserialize)]
pub struct AssignRequest {
    pub assigned_to: Option<Uuid>,
    pub vendor_id: Option<Uuid>,
}

/// Complete request.
#[derive(Debug, Deserialize)]
pub struct CompleteRequest {
    pub actual_cost: Option<rust_decimal::Decimal>,
    pub resolution_notes: Option<String>,
}

/// Hold request.
#[derive(Debug, Deserialize)]
pub struct HoldRequest {
    pub reason: String,
}

/// Skip request.
#[derive(Debug, Deserialize)]
pub struct SkipRequest {
    pub reason: String,
}

// ==================== Work Orders (Story 20.2) ====================

async fn create_work_order(
    State(state): State<AppState>,
    user: AuthUser,
    Json(payload): Json<CreateWorkOrderRequest>,
) -> Result<(StatusCode, Json<WorkOrder>), (StatusCode, Json<ErrorResponse>)> {
    state
        .work_order_repo
        .create_work_order(payload.organization_id, user.user_id, payload.data)
        .await
        .map(|wo| (StatusCode::CREATED, Json(wo)))
        .map_err(|e| {
            tracing::error!("Failed to create work order: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DB_ERROR",
                    "Failed to create work order",
                )),
            )
        })
}

async fn list_work_orders(
    State(state): State<AppState>,
    _user: AuthUser,
    Query(query): Query<ListWorkOrdersQuery>,
) -> Result<Json<Vec<WorkOrder>>, (StatusCode, Json<ErrorResponse>)> {
    state
        .work_order_repo
        .list(query.organization_id, (&query).into())
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to list work orders: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to list work orders")),
            )
        })
}

async fn list_work_orders_with_details(
    State(state): State<AppState>,
    _user: AuthUser,
    Query(query): Query<ListWorkOrdersQuery>,
) -> Result<Json<Vec<WorkOrderWithDetails>>, (StatusCode, Json<ErrorResponse>)> {
    state
        .work_order_repo
        .list_with_details(query.organization_id, (&query).into())
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to list work orders: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to list work orders")),
            )
        })
}

async fn get_statistics(
    State(state): State<AppState>,
    _user: AuthUser,
    Query(query): Query<OrgQuery>,
) -> Result<Json<WorkOrderStatistics>, (StatusCode, Json<ErrorResponse>)> {
    state
        .work_order_repo
        .get_statistics(query.organization_id)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to get statistics: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to get statistics")),
            )
        })
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct OverdueQuery {
    pub organization_id: Uuid,
    pub limit: Option<i32>,
}

async fn list_overdue(
    State(state): State<AppState>,
    _user: AuthUser,
    Query(query): Query<OverdueQuery>,
) -> Result<Json<Vec<WorkOrder>>, (StatusCode, Json<ErrorResponse>)> {
    state
        .work_order_repo
        .list_overdue(query.organization_id, query.limit.unwrap_or(20))
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to list overdue work orders: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to list overdue")),
            )
        })
}

async fn get_work_order(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<WorkOrder>, (StatusCode, Json<ErrorResponse>)> {
    state
        .work_order_repo
        .find_by_id(id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get work order: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to get work order")),
            )
        })?
        .map(Json)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Work order not found")),
            )
        })
}

async fn update_work_order(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(data): Json<UpdateWorkOrder>,
) -> Result<Json<WorkOrder>, (StatusCode, Json<ErrorResponse>)> {
    state
        .work_order_repo
        .update(id, user.user_id, data)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to update work order: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DB_ERROR",
                    "Failed to update work order",
                )),
            )
        })
}

async fn delete_work_order(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let deleted = state.work_order_repo.delete(id).await.map_err(|e| {
        tracing::error!("Failed to delete work order: {:?}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "DB_ERROR",
                "Failed to delete work order",
            )),
        )
    })?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Work order not found")),
        ))
    }
}

async fn assign_work_order(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(data): Json<AssignRequest>,
) -> Result<Json<WorkOrder>, (StatusCode, Json<ErrorResponse>)> {
    state
        .work_order_repo
        .assign(id, user.user_id, data.assigned_to, data.vendor_id)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to assign work order: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DB_ERROR",
                    "Failed to assign work order",
                )),
            )
        })
}

async fn start_work(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<WorkOrder>, (StatusCode, Json<ErrorResponse>)> {
    state
        .work_order_repo
        .start_work(id, user.user_id)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to start work: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to start work")),
            )
        })
}

async fn complete_work_order(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(data): Json<CompleteRequest>,
) -> Result<Json<WorkOrder>, (StatusCode, Json<ErrorResponse>)> {
    state
        .work_order_repo
        .complete(
            id,
            user.user_id,
            data.actual_cost,
            data.resolution_notes.as_deref(),
        )
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to complete work order: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DB_ERROR",
                    "Failed to complete work order",
                )),
            )
        })
}

async fn put_on_hold(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(data): Json<HoldRequest>,
) -> Result<Json<WorkOrder>, (StatusCode, Json<ErrorResponse>)> {
    state
        .work_order_repo
        .put_on_hold(id, user.user_id, &data.reason)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to put work order on hold: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to put on hold")),
            )
        })
}

async fn add_comment(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(data): Json<AddWorkOrderUpdate>,
) -> Result<(StatusCode, Json<WorkOrderUpdate>), (StatusCode, Json<ErrorResponse>)> {
    state
        .work_order_repo
        .add_comment(id, user.user_id, data)
        .await
        .map(|u| (StatusCode::CREATED, Json(u)))
        .map_err(|e| {
            tracing::error!("Failed to add comment: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to add comment")),
            )
        })
}

async fn list_comments(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<Uuid>,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<Vec<WorkOrderUpdate>>, (StatusCode, Json<ErrorResponse>)> {
    state
        .work_order_repo
        .list_updates(id, query.limit.unwrap_or(50), query.offset.unwrap_or(0))
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to list comments: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to list comments")),
            )
        })
}

// ==================== Maintenance Schedules (Story 20.3) ====================

async fn create_schedule(
    State(state): State<AppState>,
    user: AuthUser,
    Json(payload): Json<CreateScheduleRequest>,
) -> Result<(StatusCode, Json<MaintenanceSchedule>), (StatusCode, Json<ErrorResponse>)> {
    state
        .work_order_repo
        .create_schedule(payload.organization_id, user.user_id, payload.data)
        .await
        .map(|s| (StatusCode::CREATED, Json(s)))
        .map_err(|e| {
            tracing::error!("Failed to create schedule: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to create schedule")),
            )
        })
}

async fn list_schedules(
    State(state): State<AppState>,
    _user: AuthUser,
    Query(query): Query<ListSchedulesQuery>,
) -> Result<Json<Vec<MaintenanceSchedule>>, (StatusCode, Json<ErrorResponse>)> {
    state
        .work_order_repo
        .list_schedules(query.organization_id, (&query).into())
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to list schedules: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to list schedules")),
            )
        })
}

async fn get_upcoming_schedules(
    State(state): State<AppState>,
    _user: AuthUser,
    Query(query): Query<UpcomingQuery>,
) -> Result<Json<Vec<UpcomingSchedule>>, (StatusCode, Json<ErrorResponse>)> {
    state
        .work_order_repo
        .get_upcoming_schedules(
            query.organization_id,
            query.days_ahead.unwrap_or(30),
            query.limit.unwrap_or(20),
        )
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to get upcoming schedules: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to get upcoming")),
            )
        })
}

async fn process_due_schedules(
    State(state): State<AppState>,
    _user: AuthUser,
    Query(query): Query<OrgQuery>,
) -> Result<Json<Vec<WorkOrder>>, (StatusCode, Json<ErrorResponse>)> {
    state
        .work_order_repo
        .process_due_schedules(query.organization_id)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to process due schedules: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DB_ERROR",
                    "Failed to process schedules",
                )),
            )
        })
}

async fn get_schedule(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<MaintenanceSchedule>, (StatusCode, Json<ErrorResponse>)> {
    state
        .work_order_repo
        .find_schedule_by_id(id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get schedule: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to get schedule")),
            )
        })?
        .map(Json)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Schedule not found")),
            )
        })
}

async fn update_schedule(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<Uuid>,
    Json(data): Json<UpdateMaintenanceSchedule>,
) -> Result<Json<MaintenanceSchedule>, (StatusCode, Json<ErrorResponse>)> {
    state
        .work_order_repo
        .update_schedule(id, data)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to update schedule: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to update schedule")),
            )
        })
}

async fn delete_schedule(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let deleted = state
        .work_order_repo
        .delete_schedule(id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete schedule: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to delete schedule")),
            )
        })?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Schedule not found")),
        ))
    }
}

async fn activate_schedule(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<MaintenanceSchedule>, (StatusCode, Json<ErrorResponse>)> {
    state
        .work_order_repo
        .set_schedule_active(id, true)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to activate schedule: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DB_ERROR",
                    "Failed to activate schedule",
                )),
            )
        })
}

async fn deactivate_schedule(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<MaintenanceSchedule>, (StatusCode, Json<ErrorResponse>)> {
    state
        .work_order_repo
        .set_schedule_active(id, false)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to deactivate schedule: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DB_ERROR",
                    "Failed to deactivate schedule",
                )),
            )
        })
}

async fn skip_schedule(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<Uuid>,
    Json(data): Json<SkipRequest>,
) -> Result<Json<MaintenanceSchedule>, (StatusCode, Json<ErrorResponse>)> {
    state
        .work_order_repo
        .skip_schedule_execution(id, &data.reason)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to skip schedule: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to skip schedule")),
            )
        })
}

async fn list_executions(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<Uuid>,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<Vec<ScheduleExecution>>, (StatusCode, Json<ErrorResponse>)> {
    state
        .work_order_repo
        .list_executions(id, query.limit.unwrap_or(50), query.offset.unwrap_or(0))
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to list executions: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to list executions")),
            )
        })
}

// ==================== Service History (Story 20.4) ====================

async fn get_equipment_service_history(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(equipment_id): Path<Uuid>,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<Vec<ServiceHistoryEntry>>, (StatusCode, Json<ErrorResponse>)> {
    state
        .work_order_repo
        .get_service_history(
            equipment_id,
            query.limit.unwrap_or(50),
            query.offset.unwrap_or(0),
        )
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to get service history: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DB_ERROR",
                    "Failed to get service history",
                )),
            )
        })
}

async fn get_building_service_history(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(building_id): Path<Uuid>,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<Vec<ServiceHistoryEntry>>, (StatusCode, Json<ErrorResponse>)> {
    state
        .work_order_repo
        .get_building_service_history(
            building_id,
            query.limit.unwrap_or(50),
            query.offset.unwrap_or(0),
        )
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to get service history: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DB_ERROR",
                    "Failed to get service history",
                )),
            )
        })
}

async fn get_cost_summary(
    State(state): State<AppState>,
    _user: AuthUser,
    Query(query): Query<CostSummaryQuery>,
) -> Result<Json<Vec<MaintenanceCostSummary>>, (StatusCode, Json<ErrorResponse>)> {
    state
        .work_order_repo
        .get_cost_summary(query.organization_id, query.start_date, query.end_date)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to get cost summary: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to get cost summary")),
            )
        })
}
