//! Work orders and maintenance scheduling routes (Epic 20).
//!
//! # RLS routing (PAP-179)
//!
//! Migration `00179` put `FORCE ROW LEVEL SECURITY` on every work-order table
//! (`work_orders`, `work_order_updates`, `maintenance_schedules`,
//! `schedule_executions`). Under `FORCE` the api-server's owner connection is no
//! longer exempt, so a query on a connection without `app.current_org_id` set
//! collapses to deny-all. Each handler therefore acquires an [`RlsConnection`]
//! (which validates membership + sets the org/user GUCs on a dedicated
//! connection) and passes `rls.conn()` to the now-stateless
//! `WorkOrderRepository`.
//!
//! The explicit `verify_org_access` membership check and the
//! `load_*_for_user` fetch-then-authorize helpers are retained as
//! defense-in-depth: they are what produces the `403`/`404` semantics on the
//! superuser-backed test pool (where `FORCE` is bypassed) and they guard the
//! client-supplied `organization_id` on the create/list endpoints.

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
use sqlx::PgConnection;
use utoipa::IntoParams;
use uuid::Uuid;

use crate::routes::pagination::clamp_limit;
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

// ==================== Authorization Helpers ====================

/// Verify the authenticated user is a member of the given organization.
///
/// Work-order routes accept the target `organization_id` from the client
/// (request body / query string) or derive it from the resource being
/// addressed by id. Either way the caller's membership must be checked
/// against `organization_members` so org A cannot read or mutate org B's
/// work orders (cross-tenant IDOR). Mirrors `integrations::sync::verify_org_access`.
async fn verify_org_access(
    state: &AppState,
    user_id: Uuid,
    org_id: Uuid,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    let is_member = state
        .org_member_repo
        .is_member(org_id, user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, "Failed to check org membership");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?;

    if !is_member {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "You are not a member of this organization",
            )),
        ));
    }

    Ok(())
}

/// Load a work order by id and authorize the caller against its owning org.
///
/// Returns `404 NOT_FOUND` if the work order does not exist and `403 FORBIDDEN`
/// if the caller is not a member of the organization that owns it. By-id
/// handlers route through this so an org A caller can never read or address an
/// org B work order. The fetch runs on the RLS-context connection supplied by
/// the handler's `RlsConnection`, so under `FORCE` RLS a foreign-org row is
/// invisible (404) even before the membership check.
async fn load_work_order_for_user(
    state: &AppState,
    conn: &mut PgConnection,
    user_id: Uuid,
    work_order_id: Uuid,
) -> Result<WorkOrder, (StatusCode, Json<ErrorResponse>)> {
    let work_order = state
        .work_order_repo
        .find_by_id(&mut *conn, work_order_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get work order: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to get work order")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Work order not found")),
            )
        })?;

    verify_org_access(state, user_id, work_order.organization_id).await?;

    Ok(work_order)
}

/// Load a maintenance schedule by id and authorize the caller against its
/// owning org. Same contract as [`load_work_order_for_user`] for schedules.
async fn load_schedule_for_user(
    state: &AppState,
    conn: &mut PgConnection,
    user_id: Uuid,
    schedule_id: Uuid,
) -> Result<MaintenanceSchedule, (StatusCode, Json<ErrorResponse>)> {
    let schedule = state
        .work_order_repo
        .find_schedule_by_id(&mut *conn, schedule_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get schedule: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to get schedule")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Schedule not found")),
            )
        })?;

    verify_org_access(state, user_id, schedule.organization_id).await?;

    Ok(schedule)
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
    mut rls: RlsConnection,
    Json(payload): Json<CreateWorkOrderRequest>,
) -> Result<(StatusCode, Json<WorkOrder>), (StatusCode, Json<ErrorResponse>)> {
    let uid = rls.user_id();
    let out: Result<(StatusCode, Json<WorkOrder>), (StatusCode, Json<ErrorResponse>)> = async {
        verify_org_access(&state, uid, payload.organization_id).await?;
        state
            .work_order_repo
            .create_work_order(
                &mut **rls.conn(),
                payload.organization_id,
                uid,
                payload.data,
            )
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
    .await;
    rls.release().await;
    out
}

async fn list_work_orders(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<ListWorkOrdersQuery>,
) -> Result<Json<Vec<WorkOrder>>, (StatusCode, Json<ErrorResponse>)> {
    let uid = rls.user_id();
    let out: Result<Json<Vec<WorkOrder>>, (StatusCode, Json<ErrorResponse>)> = async {
        verify_org_access(&state, uid, query.organization_id).await?;
        state
            .work_order_repo
            .list(&mut **rls.conn(), query.organization_id, (&query).into())
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
    .await;
    rls.release().await;
    out
}

async fn list_work_orders_with_details(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<ListWorkOrdersQuery>,
) -> Result<Json<Vec<WorkOrderWithDetails>>, (StatusCode, Json<ErrorResponse>)> {
    let uid = rls.user_id();
    let out: Result<Json<Vec<WorkOrderWithDetails>>, (StatusCode, Json<ErrorResponse>)> = async {
        verify_org_access(&state, uid, query.organization_id).await?;
        state
            .work_order_repo
            .list_with_details(&mut **rls.conn(), query.organization_id, (&query).into())
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
    .await;
    rls.release().await;
    out
}

async fn get_statistics(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<OrgQuery>,
) -> Result<Json<WorkOrderStatistics>, (StatusCode, Json<ErrorResponse>)> {
    let uid = rls.user_id();
    let out: Result<Json<WorkOrderStatistics>, (StatusCode, Json<ErrorResponse>)> = async {
        verify_org_access(&state, uid, query.organization_id).await?;
        state
            .work_order_repo
            .get_statistics(&mut **rls.conn(), query.organization_id)
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
    .await;
    rls.release().await;
    out
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct OverdueQuery {
    pub organization_id: Uuid,
    pub limit: Option<i32>,
}

async fn list_overdue(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<OverdueQuery>,
) -> Result<Json<Vec<WorkOrder>>, (StatusCode, Json<ErrorResponse>)> {
    let uid = rls.user_id();
    let out: Result<Json<Vec<WorkOrder>>, (StatusCode, Json<ErrorResponse>)> = async {
        verify_org_access(&state, uid, query.organization_id).await?;
        state
            .work_order_repo
            .list_overdue(
                &mut **rls.conn(),
                query.organization_id,
                clamp_limit(query.limit.map(i64::from), 20) as i32,
            )
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
    .await;
    rls.release().await;
    out
}

async fn get_work_order(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<WorkOrder>, (StatusCode, Json<ErrorResponse>)> {
    let uid = rls.user_id();
    let out = load_work_order_for_user(&state, rls.conn(), uid, id)
        .await
        .map(Json);
    rls.release().await;
    out
}

async fn update_work_order(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(data): Json<UpdateWorkOrder>,
) -> Result<Json<WorkOrder>, (StatusCode, Json<ErrorResponse>)> {
    let uid = rls.user_id();
    let out: Result<Json<WorkOrder>, (StatusCode, Json<ErrorResponse>)> = async {
        load_work_order_for_user(&state, rls.conn(), uid, id).await?;
        state
            .work_order_repo
            .update(rls.conn(), id, uid, data)
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
    .await;
    rls.release().await;
    out
}

async fn delete_work_order(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let uid = rls.user_id();
    let out: Result<StatusCode, (StatusCode, Json<ErrorResponse>)> = async {
        load_work_order_for_user(&state, rls.conn(), uid, id).await?;
        let deleted = state
            .work_order_repo
            .delete(&mut **rls.conn(), id)
            .await
            .map_err(|e| {
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
    let uid = rls.user_id();
    let out: Result<Json<WorkOrder>, (StatusCode, Json<ErrorResponse>)> = async {
        load_work_order_for_user(&state, rls.conn(), uid, id).await?;
        state
            .work_order_repo
            .assign(rls.conn(), id, uid, data.assigned_to, data.vendor_id)
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
    .await;
    rls.release().await;
    out
}

async fn start_work(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<WorkOrder>, (StatusCode, Json<ErrorResponse>)> {
    let uid = rls.user_id();
    let out: Result<Json<WorkOrder>, (StatusCode, Json<ErrorResponse>)> = async {
        load_work_order_for_user(&state, rls.conn(), uid, id).await?;
        state
            .work_order_repo
            .start_work(rls.conn(), id, uid)
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
    let uid = rls.user_id();
    let out: Result<Json<WorkOrder>, (StatusCode, Json<ErrorResponse>)> = async {
        load_work_order_for_user(&state, rls.conn(), uid, id).await?;
        state
            .work_order_repo
            .complete(
                rls.conn(),
                id,
                uid,
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
    let uid = rls.user_id();
    let out: Result<Json<WorkOrder>, (StatusCode, Json<ErrorResponse>)> = async {
        load_work_order_for_user(&state, rls.conn(), uid, id).await?;
        state
            .work_order_repo
            .put_on_hold(rls.conn(), id, uid, &data.reason)
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
    let uid = rls.user_id();
    let out: Result<(StatusCode, Json<WorkOrderUpdate>), (StatusCode, Json<ErrorResponse>)> =
        async {
            load_work_order_for_user(&state, rls.conn(), uid, id).await?;
            state
                .work_order_repo
                .add_comment(&mut **rls.conn(), id, uid, data)
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
    let uid = rls.user_id();
    let out: Result<Json<Vec<WorkOrderUpdate>>, (StatusCode, Json<ErrorResponse>)> = async {
        load_work_order_for_user(&state, rls.conn(), uid, id).await?;
        state
            .work_order_repo
            .list_updates(
                &mut **rls.conn(),
                id,
                clamp_limit(query.limit.map(i64::from), 50) as i32,
                query.offset.unwrap_or(0),
            )
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
    let uid = rls.user_id();
    let out: Result<(StatusCode, Json<MaintenanceSchedule>), (StatusCode, Json<ErrorResponse>)> =
        async {
            verify_org_access(&state, uid, payload.organization_id).await?;
            state
                .work_order_repo
                .create_schedule(
                    &mut **rls.conn(),
                    payload.organization_id,
                    uid,
                    payload.data,
                )
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
        .await;
    rls.release().await;
    out
}

async fn list_schedules(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<ListSchedulesQuery>,
) -> Result<Json<Vec<MaintenanceSchedule>>, (StatusCode, Json<ErrorResponse>)> {
    let uid = rls.user_id();
    let out: Result<Json<Vec<MaintenanceSchedule>>, (StatusCode, Json<ErrorResponse>)> = async {
        verify_org_access(&state, uid, query.organization_id).await?;
        state
            .work_order_repo
            .list_schedules(&mut **rls.conn(), query.organization_id, (&query).into())
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
    .await;
    rls.release().await;
    out
}

async fn get_upcoming_schedules(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<UpcomingQuery>,
) -> Result<Json<Vec<UpcomingSchedule>>, (StatusCode, Json<ErrorResponse>)> {
    let uid = rls.user_id();
    let out: Result<Json<Vec<UpcomingSchedule>>, (StatusCode, Json<ErrorResponse>)> = async {
        verify_org_access(&state, uid, query.organization_id).await?;
        state
            .work_order_repo
            .get_upcoming_schedules(
                &mut **rls.conn(),
                query.organization_id,
                query.days_ahead.unwrap_or(30),
                clamp_limit(query.limit.map(i64::from), 20) as i32,
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
    .await;
    rls.release().await;
    out
}

async fn process_due_schedules(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<OrgQuery>,
) -> Result<Json<Vec<WorkOrder>>, (StatusCode, Json<ErrorResponse>)> {
    let uid = rls.user_id();
    let out: Result<Json<Vec<WorkOrder>>, (StatusCode, Json<ErrorResponse>)> = async {
        verify_org_access(&state, uid, query.organization_id).await?;
        state
            .work_order_repo
            .process_due_schedules(rls.conn(), query.organization_id)
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
    .await;
    rls.release().await;
    out
}

async fn get_schedule(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<MaintenanceSchedule>, (StatusCode, Json<ErrorResponse>)> {
    let uid = rls.user_id();
    let out = load_schedule_for_user(&state, rls.conn(), uid, id)
        .await
        .map(Json);
    rls.release().await;
    out
}

async fn update_schedule(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(data): Json<UpdateMaintenanceSchedule>,
) -> Result<Json<MaintenanceSchedule>, (StatusCode, Json<ErrorResponse>)> {
    let uid = rls.user_id();
    let out: Result<Json<MaintenanceSchedule>, (StatusCode, Json<ErrorResponse>)> = async {
        load_schedule_for_user(&state, rls.conn(), uid, id).await?;
        state
            .work_order_repo
            .update_schedule(&mut **rls.conn(), id, data)
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
    .await;
    rls.release().await;
    out
}

async fn delete_schedule(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let uid = rls.user_id();
    let out: Result<StatusCode, (StatusCode, Json<ErrorResponse>)> = async {
        load_schedule_for_user(&state, rls.conn(), uid, id).await?;
        let deleted = state
            .work_order_repo
            .delete_schedule(&mut **rls.conn(), id)
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
    .await;
    rls.release().await;
    out
}

async fn activate_schedule(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<MaintenanceSchedule>, (StatusCode, Json<ErrorResponse>)> {
    let uid = rls.user_id();
    let out: Result<Json<MaintenanceSchedule>, (StatusCode, Json<ErrorResponse>)> = async {
        load_schedule_for_user(&state, rls.conn(), uid, id).await?;
        state
            .work_order_repo
            .set_schedule_active(&mut **rls.conn(), id, true)
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
    .await;
    rls.release().await;
    out
}

async fn deactivate_schedule(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<MaintenanceSchedule>, (StatusCode, Json<ErrorResponse>)> {
    let uid = rls.user_id();
    let out: Result<Json<MaintenanceSchedule>, (StatusCode, Json<ErrorResponse>)> = async {
        load_schedule_for_user(&state, rls.conn(), uid, id).await?;
        state
            .work_order_repo
            .set_schedule_active(&mut **rls.conn(), id, false)
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
    let uid = rls.user_id();
    let out: Result<Json<MaintenanceSchedule>, (StatusCode, Json<ErrorResponse>)> = async {
        load_schedule_for_user(&state, rls.conn(), uid, id).await?;
        state
            .work_order_repo
            .skip_schedule_execution(rls.conn(), id, &data.reason)
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
    let uid = rls.user_id();
    let out: Result<Json<Vec<ScheduleExecution>>, (StatusCode, Json<ErrorResponse>)> = async {
        load_schedule_for_user(&state, rls.conn(), uid, id).await?;
        state
            .work_order_repo
            .list_executions(
                &mut **rls.conn(),
                id,
                clamp_limit(query.limit.map(i64::from), 50) as i32,
                query.offset.unwrap_or(0),
            )
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
    .await;
    rls.release().await;
    out
}

// ==================== Service History (Story 20.4) ====================
//
// These two endpoints are keyed by `equipment_id` / `building_id`, neither of
// which carries an `organization_id` the handler can authorize against without
// a cross-resource lookup. With the PAP-179 conversion they now run on the
// caller's `RlsConnection`, so under `FORCE` RLS the `work_orders` join is
// scoped to the caller's org by the row-security policy — closing the #821 P2
// cross-tenant gap at the database layer (a foreign org's equipment/building id
// yields an empty history rather than another tenant's service records).
//
// BIT-56: the org-gate pre-lookup also runs on the caller's `RlsConnection`
// (not the raw pool). `equipment` and `buildings` are tenant-isolation RLS
// tables, so the lookup is scoped to the caller's org: an unknown id AND a
// foreign-org id both resolve to empty -> 404. This deliberately supersedes
// #1372's 404-vs-403 split: returning 403 for a foreign-org id is a
// cross-tenant existence oracle (it confirms the resource exists in another
// tenant), so 404 for both is the more secure contract. `verify_org_access`
// is retained as defense-in-depth — under correct RLS the resolved org is
// always the caller's own, so it is a belt-and-suspenders guard against RLS
// being misconfigured or disabled.

async fn get_equipment_service_history(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(equipment_id): Path<Uuid>,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<Vec<ServiceHistoryEntry>>, (StatusCode, Json<ErrorResponse>)> {
    let uid = rls.user_id();
    let out: Result<Json<Vec<ServiceHistoryEntry>>, (StatusCode, Json<ErrorResponse>)> = async {
        // Resolve the owning org on the caller's RLS connection — unknown id
        // and foreign-org id both resolve to empty -> 404 (see BIT-56 note above).
        let org_id: Option<Uuid> =
            sqlx::query_scalar("SELECT organization_id FROM equipment WHERE id = $1")
                .bind(equipment_id)
                .fetch_optional(&mut **rls.conn())
                .await
                .map_err(|e| {
                    tracing::error!(error = ?e, "Failed to look up equipment org");
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse::new("DB_ERROR", "Database error")),
                    )
                })?;
        let org_id = org_id.ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Equipment not found")),
            )
        })?;
        verify_org_access(&state, uid, org_id).await?;
        state
            .work_order_repo
            .get_service_history(
                &mut **rls.conn(),
                equipment_id,
                clamp_limit(query.limit.map(i64::from), 50) as i32,
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
    .await;
    rls.release().await;
    out
}

async fn get_building_service_history(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(building_id): Path<Uuid>,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<Vec<ServiceHistoryEntry>>, (StatusCode, Json<ErrorResponse>)> {
    let uid = rls.user_id();
    let out: Result<Json<Vec<ServiceHistoryEntry>>, (StatusCode, Json<ErrorResponse>)> = async {
        // Resolve the owning org on the caller's RLS connection — unknown id
        // and foreign-org id both resolve to empty -> 404 (see BIT-56 note above).
        let org_id: Option<Uuid> =
            sqlx::query_scalar("SELECT organization_id FROM buildings WHERE id = $1")
                .bind(building_id)
                .fetch_optional(&mut **rls.conn())
                .await
                .map_err(|e| {
                    tracing::error!(error = ?e, "Failed to look up building org");
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse::new("DB_ERROR", "Database error")),
                    )
                })?;
        let org_id = org_id.ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Building not found")),
            )
        })?;
        verify_org_access(&state, uid, org_id).await?;
        state
            .work_order_repo
            .get_building_service_history(
                &mut **rls.conn(),
                building_id,
                clamp_limit(query.limit.map(i64::from), 50) as i32,
                query.offset.unwrap_or(0),
            )
            .await
            .map(Json)
            .map_err(|e| {
                tracing::error!("Failed to get building service history: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new(
                        "DB_ERROR",
                        "Failed to get building service history",
                    )),
                )
            })
    }
    .await;
    rls.release().await;
    out
}

async fn get_cost_summary(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<CostSummaryQuery>,
) -> Result<Json<Vec<MaintenanceCostSummary>>, (StatusCode, Json<ErrorResponse>)> {
    let uid = rls.user_id();
    let out: Result<Json<Vec<MaintenanceCostSummary>>, (StatusCode, Json<ErrorResponse>)> = async {
        verify_org_access(&state, uid, query.organization_id).await?;
        state
            .work_order_repo
            .get_cost_summary(
                &mut **rls.conn(),
                query.organization_id,
                query.start_date,
                query.end_date,
            )
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
    .await;
    rls.release().await;
    out
}
