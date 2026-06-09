//! Work orders and maintenance scheduling routes (Epic 20).
//!
//! # RLS (PAP-67)
//!
//! Migration `00179` put `FORCE ROW LEVEL SECURITY` on the work-order /
//! maintenance-schedule tables, so every query MUST run on a connection that has
//! `app.current_org_id` set or it collapses to deny-all. Each handler therefore
//! acquires an [`RlsConnection`] (which validates tenant membership and sets the
//! org/user GUCs on a dedicated connection) and passes `&mut **rls.conn()` to the
//! repository. The authoritative organization is `rls.tenant_id()` — the tenant
//! the caller was validated against — not a client-supplied `organization_id`,
//! so the SQL org filter and the RLS context can never disagree. Cross-tenant
//! access is blocked by RLS: a by-id read of another org's row returns no row
//! (`404`), and a write targeting another org fails the policy's `WITH CHECK`.
//! `rls.release()` clears the context before the connection returns to the pool.

use api_core::extractors::RlsConnection;
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

// ==================== Error Helpers ====================

/// Map a repository error to a `500` with a stable code, logging the cause.
fn db_error(msg: &'static str, e: sqlx::Error) -> (StatusCode, Json<ErrorResponse>) {
    tracing::error!("{}: {:?}", msg, e);
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse::new("DB_ERROR", msg)),
    )
}

/// Build a `404` response.
fn not_found(msg: &'static str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse::new("NOT_FOUND", msg)),
    )
}

/// Load a work order by id under the caller's RLS context.
///
/// RLS scopes `find_by_id` to the connection's org, so a work order owned by
/// another organization is invisible and surfaces as `404 NOT_FOUND` — the
/// cross-tenant IDOR guard is now enforced by the database, not an app-level
/// membership check.
async fn load_work_order_for_user(
    state: &AppState,
    rls: &mut RlsConnection,
    work_order_id: Uuid,
) -> Result<WorkOrder, (StatusCode, Json<ErrorResponse>)> {
    state
        .work_order_repo
        .find_by_id(&mut **rls.conn(), work_order_id)
        .await
        .map_err(|e| db_error("Failed to get work order", e))?
        .ok_or_else(|| not_found("Work order not found"))
}

/// Load a maintenance schedule by id under the caller's RLS context.
/// Same contract as [`load_work_order_for_user`].
async fn load_schedule_for_user(
    state: &AppState,
    rls: &mut RlsConnection,
    schedule_id: Uuid,
) -> Result<MaintenanceSchedule, (StatusCode, Json<ErrorResponse>)> {
    state
        .work_order_repo
        .find_schedule_by_id(&mut **rls.conn(), schedule_id)
        .await
        .map_err(|e| db_error("Failed to get schedule", e))?
        .ok_or_else(|| not_found("Schedule not found"))
}

// ==================== Request/Response Types ====================

/// Organization query parameter.
///
/// Retained for wire compatibility; the authoritative org is `rls.tenant_id()`.
#[derive(Debug, Deserialize, IntoParams)]
pub struct OrgQuery {
    pub organization_id: Option<Uuid>,
}

/// Create work order request.
#[derive(Debug, Deserialize)]
pub struct CreateWorkOrderRequest {
    pub organization_id: Option<Uuid>,
    #[serde(flatten)]
    pub data: CreateWorkOrder,
}

/// List work orders query.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListWorkOrdersQuery {
    pub organization_id: Option<Uuid>,
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
    pub organization_id: Option<Uuid>,
    #[serde(flatten)]
    pub data: CreateMaintenanceSchedule,
}

/// List schedules query.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListSchedulesQuery {
    pub organization_id: Option<Uuid>,
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
    pub organization_id: Option<Uuid>,
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
    pub organization_id: Option<Uuid>,
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

#[derive(Debug, Deserialize, IntoParams)]
pub struct OverdueQuery {
    pub organization_id: Option<Uuid>,
    pub limit: Option<i32>,
}

// ==================== Work Orders (Story 20.2) ====================

async fn create_work_order(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Json(payload): Json<CreateWorkOrderRequest>,
) -> Result<(StatusCode, Json<WorkOrder>), (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let user_id = rls.user_id();
    let out = state
        .work_order_repo
        .create_work_order(&mut **rls.conn(), org_id, user_id, payload.data)
        .await
        .map(|wo| (StatusCode::CREATED, Json(wo)))
        .map_err(|e| db_error("Failed to create work order", e));
    rls.release().await;
    out
}

async fn list_work_orders(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<ListWorkOrdersQuery>,
) -> Result<Json<Vec<WorkOrder>>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .work_order_repo
        .list(&mut **rls.conn(), org_id, (&query).into())
        .await
        .map(Json)
        .map_err(|e| db_error("Failed to list work orders", e));
    rls.release().await;
    out
}

async fn list_work_orders_with_details(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<ListWorkOrdersQuery>,
) -> Result<Json<Vec<WorkOrderWithDetails>>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .work_order_repo
        .list_with_details(&mut **rls.conn(), org_id, (&query).into())
        .await
        .map(Json)
        .map_err(|e| db_error("Failed to list work orders", e));
    rls.release().await;
    out
}

async fn get_statistics(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(_query): Query<OrgQuery>,
) -> Result<Json<WorkOrderStatistics>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .work_order_repo
        .get_statistics(&mut **rls.conn(), org_id)
        .await
        .map(Json)
        .map_err(|e| db_error("Failed to get statistics", e));
    rls.release().await;
    out
}

async fn list_overdue(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<OverdueQuery>,
) -> Result<Json<Vec<WorkOrder>>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .work_order_repo
        .list_overdue(&mut **rls.conn(), org_id, query.limit.unwrap_or(20))
        .await
        .map(Json)
        .map_err(|e| db_error("Failed to list overdue", e));
    rls.release().await;
    out
}

async fn get_work_order(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<WorkOrder>, (StatusCode, Json<ErrorResponse>)> {
    let out = load_work_order_for_user(&state, &mut rls, id).await.map(Json);
    rls.release().await;
    out
}

async fn update_work_order(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(data): Json<UpdateWorkOrder>,
) -> Result<Json<WorkOrder>, (StatusCode, Json<ErrorResponse>)> {
    let user_id = rls.user_id();
    let out = async {
        load_work_order_for_user(&state, &mut rls, id).await?;
        state
            .work_order_repo
            .update(&mut **rls.conn(), id, user_id, data)
            .await
            .map(Json)
            .map_err(|e| db_error("Failed to update work order", e))
    }
    .await;
    rls.release().await;
    out
}

async fn delete_work_order(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let out = async {
        load_work_order_for_user(&state, &mut rls, id).await?;
        let deleted = state
            .work_order_repo
            .delete(&mut **rls.conn(), id)
            .await
            .map_err(|e| db_error("Failed to delete work order", e))?;
        if deleted {
            Ok(StatusCode::NO_CONTENT)
        } else {
            Err(not_found("Work order not found"))
        }
    }
    .await;
    rls.release().await;
    out
}

async fn assign_work_order(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(data): Json<AssignRequest>,
) -> Result<Json<WorkOrder>, (StatusCode, Json<ErrorResponse>)> {
    let user_id = rls.user_id();
    let out = async {
        load_work_order_for_user(&state, &mut rls, id).await?;
        state
            .work_order_repo
            .assign(&mut **rls.conn(), id, user_id, data.assigned_to, data.vendor_id)
            .await
            .map(Json)
            .map_err(|e| db_error("Failed to assign work order", e))
    }
    .await;
    rls.release().await;
    out
}

async fn start_work(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<WorkOrder>, (StatusCode, Json<ErrorResponse>)> {
    let user_id = rls.user_id();
    let out = async {
        load_work_order_for_user(&state, &mut rls, id).await?;
        state
            .work_order_repo
            .start_work(&mut **rls.conn(), id, user_id)
            .await
            .map(Json)
            .map_err(|e| db_error("Failed to start work", e))
    }
    .await;
    rls.release().await;
    out
}

async fn complete_work_order(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(data): Json<CompleteRequest>,
) -> Result<Json<WorkOrder>, (StatusCode, Json<ErrorResponse>)> {
    let user_id = rls.user_id();
    let out = async {
        load_work_order_for_user(&state, &mut rls, id).await?;
        state
            .work_order_repo
            .complete(
                &mut **rls.conn(),
                id,
                user_id,
                data.actual_cost,
                data.resolution_notes.as_deref(),
            )
            .await
            .map(Json)
            .map_err(|e| db_error("Failed to complete work order", e))
    }
    .await;
    rls.release().await;
    out
}

async fn put_on_hold(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(data): Json<HoldRequest>,
) -> Result<Json<WorkOrder>, (StatusCode, Json<ErrorResponse>)> {
    let user_id = rls.user_id();
    let out = async {
        load_work_order_for_user(&state, &mut rls, id).await?;
        state
            .work_order_repo
            .put_on_hold(&mut **rls.conn(), id, user_id, &data.reason)
            .await
            .map(Json)
            .map_err(|e| db_error("Failed to put on hold", e))
    }
    .await;
    rls.release().await;
    out
}

async fn add_comment(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(data): Json<AddWorkOrderUpdate>,
) -> Result<(StatusCode, Json<WorkOrderUpdate>), (StatusCode, Json<ErrorResponse>)> {
    let user_id = rls.user_id();
    let out = async {
        load_work_order_for_user(&state, &mut rls, id).await?;
        state
            .work_order_repo
            .add_comment(&mut **rls.conn(), id, user_id, data)
            .await
            .map(|u| (StatusCode::CREATED, Json(u)))
            .map_err(|e| db_error("Failed to add comment", e))
    }
    .await;
    rls.release().await;
    out
}

async fn list_comments(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<Vec<WorkOrderUpdate>>, (StatusCode, Json<ErrorResponse>)> {
    let out = async {
        load_work_order_for_user(&state, &mut rls, id).await?;
        state
            .work_order_repo
            .list_updates(
                &mut **rls.conn(),
                id,
                query.limit.unwrap_or(50),
                query.offset.unwrap_or(0),
            )
            .await
            .map(Json)
            .map_err(|e| db_error("Failed to list comments", e))
    }
    .await;
    rls.release().await;
    out
}

// ==================== Maintenance Schedules (Story 20.3) ====================

async fn create_schedule(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Json(payload): Json<CreateScheduleRequest>,
) -> Result<(StatusCode, Json<MaintenanceSchedule>), (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let user_id = rls.user_id();
    let out = state
        .work_order_repo
        .create_schedule(&mut **rls.conn(), org_id, user_id, payload.data)
        .await
        .map(|s| (StatusCode::CREATED, Json(s)))
        .map_err(|e| db_error("Failed to create schedule", e));
    rls.release().await;
    out
}

async fn list_schedules(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<ListSchedulesQuery>,
) -> Result<Json<Vec<MaintenanceSchedule>>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .work_order_repo
        .list_schedules(&mut **rls.conn(), org_id, (&query).into())
        .await
        .map(Json)
        .map_err(|e| db_error("Failed to list schedules", e));
    rls.release().await;
    out
}

async fn get_upcoming_schedules(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<UpcomingQuery>,
) -> Result<Json<Vec<UpcomingSchedule>>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .work_order_repo
        .get_upcoming_schedules(
            &mut **rls.conn(),
            org_id,
            query.days_ahead.unwrap_or(30),
            query.limit.unwrap_or(20),
        )
        .await
        .map(Json)
        .map_err(|e| db_error("Failed to get upcoming", e));
    rls.release().await;
    out
}

async fn process_due_schedules(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(_query): Query<OrgQuery>,
) -> Result<Json<Vec<WorkOrder>>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .work_order_repo
        .process_due_schedules(&mut **rls.conn(), org_id)
        .await
        .map(Json)
        .map_err(|e| db_error("Failed to process schedules", e));
    rls.release().await;
    out
}

async fn get_schedule(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<MaintenanceSchedule>, (StatusCode, Json<ErrorResponse>)> {
    let out = load_schedule_for_user(&state, &mut rls, id).await.map(Json);
    rls.release().await;
    out
}

async fn update_schedule(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(data): Json<UpdateMaintenanceSchedule>,
) -> Result<Json<MaintenanceSchedule>, (StatusCode, Json<ErrorResponse>)> {
    let out = async {
        load_schedule_for_user(&state, &mut rls, id).await?;
        state
            .work_order_repo
            .update_schedule(&mut **rls.conn(), id, data)
            .await
            .map(Json)
            .map_err(|e| db_error("Failed to update schedule", e))
    }
    .await;
    rls.release().await;
    out
}

async fn delete_schedule(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let out = async {
        load_schedule_for_user(&state, &mut rls, id).await?;
        let deleted = state
            .work_order_repo
            .delete_schedule(&mut **rls.conn(), id)
            .await
            .map_err(|e| db_error("Failed to delete schedule", e))?;
        if deleted {
            Ok(StatusCode::NO_CONTENT)
        } else {
            Err(not_found("Schedule not found"))
        }
    }
    .await;
    rls.release().await;
    out
}

async fn activate_schedule(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<MaintenanceSchedule>, (StatusCode, Json<ErrorResponse>)> {
    let out = async {
        load_schedule_for_user(&state, &mut rls, id).await?;
        state
            .work_order_repo
            .set_schedule_active(&mut **rls.conn(), id, true)
            .await
            .map(Json)
            .map_err(|e| db_error("Failed to activate schedule", e))
    }
    .await;
    rls.release().await;
    out
}

async fn deactivate_schedule(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<MaintenanceSchedule>, (StatusCode, Json<ErrorResponse>)> {
    let out = async {
        load_schedule_for_user(&state, &mut rls, id).await?;
        state
            .work_order_repo
            .set_schedule_active(&mut **rls.conn(), id, false)
            .await
            .map(Json)
            .map_err(|e| db_error("Failed to deactivate schedule", e))
    }
    .await;
    rls.release().await;
    out
}

async fn skip_schedule(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(data): Json<SkipRequest>,
) -> Result<Json<MaintenanceSchedule>, (StatusCode, Json<ErrorResponse>)> {
    let out = async {
        load_schedule_for_user(&state, &mut rls, id).await?;
        state
            .work_order_repo
            .skip_schedule_execution(&mut **rls.conn(), id, &data.reason)
            .await
            .map(Json)
            .map_err(|e| db_error("Failed to skip schedule", e))
    }
    .await;
    rls.release().await;
    out
}

async fn list_executions(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<Vec<ScheduleExecution>>, (StatusCode, Json<ErrorResponse>)> {
    let out = async {
        load_schedule_for_user(&state, &mut rls, id).await?;
        state
            .work_order_repo
            .list_executions(
                &mut **rls.conn(),
                id,
                query.limit.unwrap_or(50),
                query.offset.unwrap_or(0),
            )
            .await
            .map(Json)
            .map_err(|e| db_error("Failed to list executions", e))
    }
    .await;
    rls.release().await;
    out
}

// ==================== Service History (Story 20.4) ====================
//
// These endpoints are keyed by `equipment_id` / `building_id`, which carry no
// `organization_id` of their own. Under PAP-67 they now run on an
// `RlsConnection`, so RLS on `work_orders` scopes the result to the caller's
// org automatically — closing the cross-tenant gap the previous #821 P2 note
// described, without a separate equipment/building → org lookup.

async fn get_equipment_service_history(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(equipment_id): Path<Uuid>,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<Vec<ServiceHistoryEntry>>, (StatusCode, Json<ErrorResponse>)> {
    let out = state
        .work_order_repo
        .get_service_history(
            &mut **rls.conn(),
            equipment_id,
            query.limit.unwrap_or(50),
            query.offset.unwrap_or(0),
        )
        .await
        .map(Json)
        .map_err(|e| db_error("Failed to get service history", e));
    rls.release().await;
    out
}

async fn get_building_service_history(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(building_id): Path<Uuid>,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<Vec<ServiceHistoryEntry>>, (StatusCode, Json<ErrorResponse>)> {
    let out = state
        .work_order_repo
        .get_building_service_history(
            &mut **rls.conn(),
            building_id,
            query.limit.unwrap_or(50),
            query.offset.unwrap_or(0),
        )
        .await
        .map(Json)
        .map_err(|e| db_error("Failed to get service history", e));
    rls.release().await;
    out
}

async fn get_cost_summary(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<CostSummaryQuery>,
) -> Result<Json<Vec<MaintenanceCostSummary>>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .work_order_repo
        .get_cost_summary(&mut **rls.conn(), org_id, query.start_date, query.end_date)
        .await
        .map(Json)
        .map_err(|e| db_error("Failed to get cost summary", e));
    rls.release().await;
    out
}
