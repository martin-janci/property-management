//! Fault routes (Epic 4: Fault Reporting & Resolution).

use crate::routes::pagination::clamp_limit;
use crate::state::AppState;
use api_core::extractors::principal::RequestPrincipal;
use api_core::extractors::RlsConnection;
use axum::{
    body::Body,
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
    routing::{delete, get, post, put},
    Json, Router,
};
use common::errors::ErrorResponse;
use common::notifications::{Notification, NotificationCategory};
use db::models::{
    AddFaultComment, AddWorkNote, AiSuggestion, AssignFault, ConfirmFault, CreateFault,
    CreateFaultAttachment, Fault, FaultAttachment, FaultListQuery, FaultStatistics, FaultSummary,
    FaultTimelineEntryWithUser, FaultWithDetails, ReopenFault, ResolveFault, TriageFault,
    UpdateFault, UpdateFaultStatus,
};
use db::repositories::MembershipRepository;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

// ============================================================================
// Notification recipient policy (single source of truth — #2029)
// ============================================================================
//
// The fault-lifecycle notification recipient rules are extracted into pure
// functions so the production handlers below and the recipient tests in
// `tests/fault_notification_recipient_tests.rs` assert against the *same*
// logic. Previously the tests re-implemented the selection inline, so the
// handler's real policy (self-exclusion, dedup, `assigned_to`/manager
// handling) could drift while the tests stayed green (#1974 / #2029).

/// Recipients for a `triage_fault` notification (#1793).
///
/// The reporter (unless they are the triaging manager) plus the assigned
/// technician if triage set one (skipping the actor and any duplicate).
pub fn triage_fault_recipients(
    reporter_id: Uuid,
    actor_id: Uuid,
    assigned_to: Option<Uuid>,
) -> Vec<Uuid> {
    let mut recipients: Vec<Uuid> = Vec::new();
    if reporter_id != actor_id {
        recipients.push(reporter_id);
    }
    if let Some(assignee) = assigned_to {
        if assignee != actor_id && !recipients.contains(&assignee) {
            recipients.push(assignee);
        }
    }
    recipients
}

/// Recipients for a `confirm_fault` notification (#1793).
///
/// The assignee who did the work (unless they are the confirming reporter)
/// plus the org's managers, excluding the actor and any duplicate.
pub fn confirm_fault_recipients(
    assigned_to: Option<Uuid>,
    actor_id: Uuid,
    manager_ids: impl IntoIterator<Item = Uuid>,
) -> Vec<Uuid> {
    let mut recipients: Vec<Uuid> = Vec::new();
    if let Some(assignee) = assigned_to {
        if assignee != actor_id {
            recipients.push(assignee);
        }
    }
    for mid in manager_ids {
        if mid != actor_id && !recipients.contains(&mid) {
            recipients.push(mid);
        }
    }
    recipients
}

/// Recipients for an `assign_fault` notification (Story 4.3 / #2085).
///
/// The newly assigned technician (unless the assigning manager assigned it to
/// themselves) plus the reporter, excluding the actor and de-duplicating the
/// reporter against the assignee.
pub fn assign_fault_recipients(
    assigned_to: Option<Uuid>,
    reporter_id: Uuid,
    actor_id: Uuid,
) -> Vec<Uuid> {
    let mut recipients: Vec<Uuid> = Vec::new();
    if let Some(assignee) = assigned_to {
        if assignee != actor_id {
            recipients.push(assignee);
        }
    }
    if reporter_id != actor_id && Some(reporter_id) != assigned_to {
        recipients.push(reporter_id);
    }
    recipients
}

/// Recipients for a manager-broadcast notification (fault created / reopened).
///
/// The org's managers, excluding the acting user and any duplicate ids. Shared
/// by the create (Story 4.1) and reopen (Story 4.6) handlers so the
/// self-exclusion + dedup policy stays in one place (#2085).
pub fn manager_recipients(
    manager_ids: impl IntoIterator<Item = Uuid>,
    actor_id: Uuid,
) -> Vec<Uuid> {
    let mut recipients: Vec<Uuid> = Vec::new();
    for mid in manager_ids {
        if mid != actor_id && !recipients.contains(&mid) {
            recipients.push(mid);
        }
    }
    recipients
}

// ============================================================================
// Helper Functions
// ============================================================================
//
// SECURITY: The previous `extract_tenant_context` helper deserialized the
// client-supplied `X-Tenant-Context` JSON header directly into a
// `TenantContext`. No JWT verification — any unauthenticated caller could
// forge tenancy. That helper has been deleted; every handler now goes
// through `RequestPrincipal` (verified bearer JWT + host-resolved tenant).
//
// The `is_manager` branches below now perform a real role lookup against
// `user_memberships` via `MembershipRepository::is_manager_in_org` (P0-07);
// no client-supplied role is trusted.

/// Resolve the effective tenant id from a verified [`RequestPrincipal`].
fn require_tenant_id(
    principal: &RequestPrincipal,
) -> Result<Uuid, (StatusCode, Json<ErrorResponse>)> {
    principal.effective_org.ok_or_else(|| {
        (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "TENANT_REQUIRED",
                "Faults endpoints require a tenant-resolved request",
            )),
        )
    })
}

/// Assert the fault `id` belongs to `tenant_id`, returning 404 otherwise.
///
/// SECURITY (#770): the legacy pool-based mutate-by-id repository methods
/// (`assign`, `resolve`, `confirm`, `reopen`, comments, attachments) run
/// OUTSIDE an `RlsConnection`, so they are NOT protected by the
/// `faults_tenant_isolation` RLS policy and would otherwise act on any org's
/// fault by UUID. This pre-flight guard scopes the row to the caller's org
/// before the mutation runs, collapsing a cross-tenant id to a 404.
async fn require_fault_in_org(
    state: &AppState,
    id: Uuid,
    tenant_id: Uuid,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    match state.fault_repo.find_by_id_for_org(id, tenant_id).await {
        Ok(Some(_)) => Ok(()),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Fault not found")),
        )),
        Err(e) => {
            tracing::error!("Failed to load fault for org scope check: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to load fault")),
            ))
        }
    }
}

/// Require the caller to hold a manager-tier role in `tenant_id`.
///
/// SECURITY (#770): triage/assign/resolve are workflow actions that any tenant
/// member could previously invoke. This gate restricts them to managers via
/// the canonical `is_manager_in_org` membership lookup (same set used by
/// `get_fault`/`list_comments` for internal-note visibility).
async fn require_manager(
    state: &AppState,
    user_id: Uuid,
    tenant_id: Uuid,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    let is_manager = MembershipRepository::new(state.db.clone())
        .is_manager_in_org(user_id, tenant_id)
        .await
        .unwrap_or(false);
    if is_manager {
        Ok(())
    } else {
        Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Manager role required for this action",
            )),
        ))
    }
}

// ============================================================================
// Response Types
// ============================================================================

/// Response for fault creation.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateFaultResponse {
    pub id: Uuid,
    pub message: String,
}

/// Response for fault list.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct FaultListResponse {
    pub faults: Vec<FaultSummary>,
    pub count: usize,
}

/// Response for fault details.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct FaultDetailResponse {
    pub fault: FaultWithDetails,
    pub timeline: Vec<FaultTimelineEntryWithUser>,
    pub attachments: Vec<FaultAttachment>,
}

/// Response for generic fault action.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct FaultActionResponse {
    pub message: String,
    pub fault: Fault,
}

/// Response for timeline list.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TimelineResponse {
    pub entries: Vec<FaultTimelineEntryWithUser>,
}

/// Response for attachments list.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AttachmentsResponse {
    pub attachments: Vec<FaultAttachment>,
}

/// Response for AI suggestion.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AiSuggestionResponse {
    pub suggestion: AiSuggestion,
}

/// Response for statistics.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct StatisticsResponse {
    pub statistics: FaultStatistics,
}

// ============================================================================
// Request Types
// ============================================================================

/// Request for creating a fault.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateFaultRequest {
    pub building_id: Uuid,
    pub unit_id: Option<Uuid>,
    pub title: String,
    pub description: String,
    pub location_description: Option<String>,
    pub category: String,
    pub priority: Option<String>,
    pub idempotency_key: Option<String>,
}

/// Request for updating a fault.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdateFaultRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub location_description: Option<String>,
    pub category: Option<String>,
}

/// Request for triaging a fault.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TriageFaultRequest {
    pub priority: String,
    pub category: Option<String>,
    pub assigned_to: Option<Uuid>,
}

/// Request for updating fault status.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdateStatusRequest {
    pub status: String,
    pub note: Option<String>,
    pub scheduled_date: Option<chrono::NaiveDate>,
    pub estimated_completion: Option<chrono::NaiveDate>,
}

/// Request for resolving a fault.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ResolveFaultRequest {
    pub resolution_notes: String,
}

/// Request for confirming fault resolution.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ConfirmFaultRequest {
    pub rating: Option<i32>,
    pub feedback: Option<String>,
}

/// Request for reopening a fault.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ReopenFaultRequest {
    pub reason: String,
}

/// Request for assigning a fault.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AssignFaultRequest {
    pub assigned_to: Uuid,
}

/// Request for adding a comment.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AddCommentRequest {
    pub note: String,
    #[serde(default)]
    pub is_internal: bool,
}

/// Request for adding a work note.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AddWorkNoteRequest {
    pub note: String,
}

/// Request for adding an attachment.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AddAttachmentRequest {
    pub filename: String,
    pub original_filename: String,
    pub content_type: String,
    pub size_bytes: i32,
    pub storage_url: String,
    pub thumbnail_url: Option<String>,
    pub description: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
}

/// Query for listing faults.
#[derive(Debug, Serialize, Deserialize, ToSchema, Default, utoipa::IntoParams)]
pub struct ListFaultsQuery {
    pub building_id: Option<Uuid>,
    pub unit_id: Option<Uuid>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub category: Option<String>,
    pub assigned_to: Option<Uuid>,
    pub search: Option<String>,
    pub from_date: Option<chrono::NaiveDate>,
    pub to_date: Option<chrono::NaiveDate>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
}

/// Query for statistics.
#[derive(Debug, Serialize, Deserialize, ToSchema, Default, utoipa::IntoParams)]
pub struct StatisticsQuery {
    pub building_id: Option<Uuid>,
}

// ============================================================================
// Router
// ============================================================================

/// Create faults router.
pub fn router() -> Router<AppState> {
    Router::new()
        // CRUD
        .route(
            "/",
            post(create_fault).route_layer(axum::middleware::from_fn(create_fault_idempotency)),
        )
        .route("/", get(list_faults))
        .route("/my", get(list_my_faults))
        .route("/{id}", get(get_fault))
        .route("/{id}", put(update_fault))
        // Workflow
        .route("/{id}/triage", post(triage_fault))
        .route("/{id}/assign", post(assign_fault))
        .route("/{id}/status", put(update_status))
        .route("/{id}/resolve", post(resolve_fault))
        .route("/{id}/confirm", post(confirm_fault))
        .route("/{id}/reopen", post(reopen_fault))
        // Comments & Notes
        .route("/{id}/comments", get(list_comments))
        .route("/{id}/comments", post(add_comment))
        .route("/{id}/work-notes", post(add_work_note))
        // Attachments
        .route("/{id}/attachments", get(list_attachments))
        .route("/{id}/attachments", post(add_attachment))
        .route(
            "/{id}/attachments/{attachment_id}",
            delete(delete_attachment),
        )
        // AI
        .route("/{id}/suggest", post(get_ai_suggestion))
        // Statistics
        .route("/statistics", get(get_statistics))
}

async fn create_fault_idempotency(
    Extension(state): Extension<AppState>,
    request: axum::http::Request<Body>,
    next: Next,
) -> Response {
    api_core::middleware::handle_idempotent_request(state.db.clone(), request, next).await
}

// ============================================================================
// Handlers
// ============================================================================

/// Create a new fault (Story 4.1).
#[utoipa::path(
    post,
    path = "/api/v1/faults",
    request_body = CreateFaultRequest,
    responses(
        (status = 201, description = "Fault created", body = CreateFaultResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "Faults"
)]
async fn create_fault(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    mut rls: RlsConnection,
    Json(req): Json<CreateFaultRequest>,
) -> Result<(StatusCode, Json<CreateFaultResponse>), (StatusCode, Json<ErrorResponse>)> {
    let tenant_id = require_tenant_id(&principal)?;

    let data = CreateFault {
        organization_id: tenant_id,
        building_id: req.building_id,
        unit_id: req.unit_id,
        reporter_id: principal.user_id,
        title: req.title,
        description: req.description,
        location_description: req.location_description,
        category: req.category,
        priority: req.priority,
        idempotency_key: req.idempotency_key,
    };

    // Idempotent offline create (#970): if the client supplied an idempotency
    // key and a fault with that key already exists in this tenant, return the
    // existing fault instead of inserting a duplicate. The lookup runs on the
    // RLS-scoped connection (NOT the org-unscoped pool-based finder) so a key
    // collision across organizations cannot leak another tenant's fault.
    if let Some(ref key) = data.idempotency_key {
        match state
            .fault_repo
            .find_by_idempotency_key_rls(&mut **rls.conn(), key)
            .await
        {
            Ok(Some(existing)) => {
                rls.release().await;
                return Ok((
                    StatusCode::OK,
                    Json(CreateFaultResponse {
                        id: existing.id,
                        message: "Fault already exists".to_string(),
                    }),
                ));
            }
            Ok(None) => {}
            Err(e) => {
                tracing::error!("Failed to check fault idempotency key: {}", e);
                rls.release().await;
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new(
                        "INTERNAL_ERROR",
                        "Failed to create fault",
                    )),
                ));
            }
        }
    }

    let fault = state
        .fault_repo
        .create_rls(&mut **rls.conn(), data)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create fault: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to create fault",
                )),
            )
        })?;

    // Story 4.1: notify all org managers about the new fault. Best-effort —
    // notification failures must not fail the mutation.
    match MembershipRepository::new(state.db.clone())
        .list_manager_ids(tenant_id)
        .await
    {
        Ok(manager_ids) => {
            let recipients = manager_recipients(manager_ids, principal.user_id);
            if !recipients.is_empty() {
                let notification = Notification::new(
                    Uuid::nil(),
                    NotificationCategory::Faults,
                    format!("New fault reported: {}", fault.title),
                    "A new fault has been reported and requires triage.".to_string(),
                )
                .with_action_url(format!("/faults/{}", fault.id))
                .with_data(serde_json::json!({
                    "fault_id": fault.id,
                    "organization_id": fault.organization_id,
                }));
                let results = state
                    .notification_pipeline
                    .dispatch_to_users(&recipients, &notification, Some(fault.id), None)
                    .await;
                tracing::info!(
                    fault_id = %fault.id,
                    recipients = results.len(),
                    "FaultCreated notifications dispatched to managers"
                );
            }
        }
        Err(e) => {
            tracing::error!(
                fault_id = %fault.id,
                error = %e,
                "Failed to load manager ids for FaultCreated notification"
            );
        }
    }

    rls.release().await;
    Ok((
        StatusCode::CREATED,
        Json(CreateFaultResponse {
            id: fault.id,
            message: "Fault created successfully".to_string(),
        }),
    ))
}

/// List faults with filters (Story 4.3).
#[utoipa::path(
    get,
    path = "/api/v1/faults",
    params(ListFaultsQuery),
    responses(
        (status = 200, description = "Fault list", body = FaultListResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "Faults"
)]
async fn list_faults(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Query(query): Query<ListFaultsQuery>,
) -> Result<Json<FaultListResponse>, (StatusCode, Json<ErrorResponse>)> {
    let tenant_id = require_tenant_id(&principal)?;

    let list_query = FaultListQuery {
        building_id: query.building_id,
        unit_id: query.unit_id,
        status: query.status.map(|s| vec![s]),
        priority: query.priority.map(|p| vec![p]),
        category: query.category.map(|c| vec![c]),
        assigned_to: query.assigned_to,
        reporter_id: None,
        search: query.search,
        from_date: query.from_date,
        to_date: query.to_date,
        limit: query.limit,
        offset: query.offset,
        sort_by: query.sort_by,
        sort_order: query.sort_order,
    };

    let faults = state
        .fault_repo
        .list(tenant_id, list_query)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list faults: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to list faults",
                )),
            )
        })?;

    let count = faults.len();
    Ok(Json(FaultListResponse { faults, count }))
}

/// List my faults (Story 4.5).
#[utoipa::path(
    get,
    path = "/api/v1/faults/my",
    responses(
        (status = 200, description = "My fault list", body = FaultListResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "Faults"
)]
async fn list_my_faults(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Query(query): Query<ListFaultsQuery>,
) -> Result<Json<FaultListResponse>, (StatusCode, Json<ErrorResponse>)> {
    let _tenant_id = require_tenant_id(&principal)?;

    let faults = state
        .fault_repo
        .list_by_reporter(
            principal.user_id,
            clamp_limit(query.limit, 50),
            query.offset.unwrap_or(0),
        )
        .await
        .map_err(|e| {
            tracing::error!("Failed to list my faults: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to list faults",
                )),
            )
        })?;

    let count = faults.len();
    Ok(Json(FaultListResponse { faults, count }))
}

/// Get fault details.
#[utoipa::path(
    get,
    path = "/api/v1/faults/{id}",
    params(
        ("id" = Uuid, Path, description = "Fault ID")
    ),
    responses(
        (status = 200, description = "Fault details", body = FaultDetailResponse),
        (status = 404, description = "Fault not found", body = ErrorResponse),
    ),
    tag = "Faults"
)]
async fn get_fault(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(id): Path<Uuid>,
) -> Result<Json<FaultDetailResponse>, (StatusCode, Json<ErrorResponse>)> {
    let tenant_id = require_tenant_id(&principal)?;
    // P0-07: real role lookup. Previously this was hardcoded `false` as a
    // least-privilege placeholder, which silently disabled every
    // manager-only branch (timeline visibility, internal notes, status
    // forces). The query goes against `user_memberships` and checks the
    // manager-tier role set defined in TenantRole::is_manager_role.
    let is_manager = MembershipRepository::new(state.db.clone())
        .is_manager_in_org(principal.user_id, tenant_id)
        .await
        .unwrap_or(false);

    // SECURITY (#770): scope the lookup to the caller's organization. The
    // previous `find_by_id_with_details(id)` ran on the raw pool with no org
    // predicate, so a caller in org B could read org A's fault (reporter PII,
    // timeline, attachment counts) by enumerating the UUID — a cross-tenant
    // IDOR. The `_for_org` variant returns None (→ 404) for a cross-tenant id.
    let fault = match state
        .fault_repo
        .find_by_id_with_details_for_org(id, tenant_id)
        .await
    {
        Ok(Some(f)) => f,
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Fault not found")),
            ))
        }
        Err(e) => {
            tracing::error!("Failed to get fault: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to get fault")),
            ));
        }
    };

    let timeline = state
        .fault_repo
        .list_timeline(id, is_manager)
        .await
        .unwrap_or_default();

    let attachments = state
        .fault_repo
        .list_attachments(id)
        .await
        .unwrap_or_default();

    Ok(Json(FaultDetailResponse {
        fault,
        timeline,
        attachments,
    }))
}

/// Update fault details.
#[utoipa::path(
    put,
    path = "/api/v1/faults/{id}",
    params(
        ("id" = Uuid, Path, description = "Fault ID")
    ),
    request_body = UpdateFaultRequest,
    responses(
        (status = 200, description = "Fault updated", body = FaultActionResponse),
        (status = 400, description = "Cannot update", body = ErrorResponse),
        (status = 404, description = "Fault not found", body = ErrorResponse),
    ),
    tag = "Faults"
)]
async fn update_fault(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateFaultRequest>,
) -> Result<Json<FaultActionResponse>, (StatusCode, Json<ErrorResponse>)> {
    let tenant_id = require_tenant_id(&principal)?;

    // Check fault exists and can be edited
    let existing = match state.fault_repo.find_by_id_rls(&mut **rls.conn(), id).await {
        Ok(Some(f)) => f,
        Ok(None) => {
            rls.release().await;
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Fault not found")),
            ));
        }
        Err(e) => {
            tracing::error!("Failed to find fault: {}", e);
            rls.release().await;
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to find fault")),
            ));
        }
    };

    // SECURITY (#970): only the reporter who filed the fault or a manager in
    // the tenant may edit it. Any other tenant member must be rejected.
    if existing.reporter_id != principal.user_id {
        if let Err(e) = require_manager(&state, principal.user_id, tenant_id).await {
            rls.release().await;
            return Err(e);
        }
    }

    if !existing.can_reporter_edit() {
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "BAD_REQUEST",
                "Fault cannot be edited after triage",
            )),
        ));
    }

    let data = UpdateFault {
        title: req.title,
        description: req.description,
        location_description: req.location_description,
        category: req.category,
    };

    match state
        .fault_repo
        .update_rls(&mut **rls.conn(), id, data)
        .await
    {
        Ok(fault) => {
            rls.release().await;
            Ok(Json(FaultActionResponse {
                message: "Fault updated".to_string(),
                fault,
            }))
        }
        Err(e) => {
            tracing::error!("Failed to update fault: {}", e);
            rls.release().await;
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to update fault",
                )),
            ))
        }
    }
}

/// Triage a fault (Story 4.3).
#[utoipa::path(
    post,
    path = "/api/v1/faults/{id}/triage",
    params(
        ("id" = Uuid, Path, description = "Fault ID")
    ),
    request_body = TriageFaultRequest,
    responses(
        (status = 200, description = "Fault triaged", body = FaultActionResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 404, description = "Fault not found", body = ErrorResponse),
    ),
    tag = "Faults"
)]
async fn triage_fault(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<TriageFaultRequest>,
) -> Result<Json<FaultActionResponse>, (StatusCode, Json<ErrorResponse>)> {
    let tenant_id = require_tenant_id(&principal)?;
    require_manager(&state, principal.user_id, tenant_id).await?;

    // Check fault exists
    let existing = match state.fault_repo.find_by_id_rls(&mut **rls.conn(), id).await {
        Ok(Some(f)) => f,
        Ok(None) => {
            rls.release().await;
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Fault not found")),
            ));
        }
        Err(e) => {
            tracing::error!("Failed to find fault: {}", e);
            rls.release().await;
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to find fault")),
            ));
        }
    };

    if existing.status != "new" {
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "BAD_REQUEST",
                "Fault has already been triaged",
            )),
        ));
    }

    let data = TriageFault {
        priority: req.priority,
        category: req.category,
        assigned_to: req.assigned_to,
    };

    let fault = state
        .fault_repo
        .triage(id, principal.user_id, data)
        .await
        .map_err(|e| {
            tracing::error!("Failed to triage fault: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to triage fault",
                )),
            )
        })?;

    rls.release().await;

    // #1793: notify the reporter (and the assigned technician, if triage set
    // one) that the fault has been triaged — the first manager touch on a fault.
    // Best-effort, mirroring the other transitions: a dispatch failure is logged
    // and never fails the mutation. Recipient policy lives in the shared
    // `triage_fault_recipients` (#2029) so the tests exercise this exact logic.
    let recipients =
        triage_fault_recipients(fault.reporter_id, principal.user_id, fault.assigned_to);
    if !recipients.is_empty() {
        let notification = Notification::new(
            Uuid::nil(),
            NotificationCategory::Faults,
            format!("Fault triaged: {}", fault.title),
            "The fault has been triaged and prioritized.".to_string(),
        )
        .with_action_url(format!("/faults/{}", fault.id))
        .with_data(serde_json::json!({
            "fault_id": fault.id,
            "organization_id": fault.organization_id,
        }));
        let results = state
            .notification_pipeline
            .dispatch_to_users(&recipients, &notification, Some(fault.id), None)
            .await;
        tracing::info!(
            fault_id = %fault.id,
            recipients = results.len(),
            "FaultTriaged notifications dispatched"
        );
    }

    Ok(Json(FaultActionResponse {
        message: "Fault triaged successfully".to_string(),
        fault,
    }))
}

/// Assign a fault.
#[utoipa::path(
    post,
    path = "/api/v1/faults/{id}/assign",
    params(
        ("id" = Uuid, Path, description = "Fault ID")
    ),
    request_body = AssignFaultRequest,
    responses(
        (status = 200, description = "Fault assigned", body = FaultActionResponse),
        (status = 404, description = "Fault not found", body = ErrorResponse),
    ),
    tag = "Faults"
)]
async fn assign_fault(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(id): Path<Uuid>,
    Json(req): Json<AssignFaultRequest>,
) -> Result<Json<FaultActionResponse>, (StatusCode, Json<ErrorResponse>)> {
    let tenant_id = require_tenant_id(&principal)?;
    require_manager(&state, principal.user_id, tenant_id).await?;
    require_fault_in_org(&state, id, tenant_id).await?;

    let data = AssignFault {
        assigned_to: req.assigned_to,
    };

    let fault = state
        .fault_repo
        .assign(id, principal.user_id, data)
        .await
        .map_err(|e| {
            tracing::error!("Failed to assign fault: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to assign fault",
                )),
            )
        })?;

    // Story 4.3: notify assignee + reporter (relevant parties). Best-effort.
    // Recipient policy lives in the shared `assign_fault_recipients` (#2085) so
    // the handler and the recipient tests assert against the same logic.
    {
        let recipients =
            assign_fault_recipients(fault.assigned_to, fault.reporter_id, principal.user_id);
        if !recipients.is_empty() {
            let notification = Notification::new(
                Uuid::nil(),
                NotificationCategory::Faults,
                format!("Fault assigned: {}", fault.title),
                "The fault has been assigned and is now in progress.".to_string(),
            )
            .with_action_url(format!("/faults/{}", fault.id))
            .with_data(serde_json::json!({
                "fault_id": fault.id,
                "organization_id": fault.organization_id,
            }));
            let results = state
                .notification_pipeline
                .dispatch_to_users(&recipients, &notification, Some(fault.id), None)
                .await;
            tracing::info!(
                fault_id = %fault.id,
                recipients = results.len(),
                "FaultAssigned notifications dispatched"
            );
        }
    }

    Ok(Json(FaultActionResponse {
        message: "Fault assigned successfully".to_string(),
        fault,
    }))
}

/// Update fault status (Story 4.4).
#[utoipa::path(
    put,
    path = "/api/v1/faults/{id}/status",
    params(
        ("id" = Uuid, Path, description = "Fault ID")
    ),
    request_body = UpdateStatusRequest,
    responses(
        (status = 200, description = "Status updated", body = FaultActionResponse),
        (status = 400, description = "Invalid status", body = ErrorResponse),
        (status = 404, description = "Fault not found", body = ErrorResponse),
    ),
    tag = "Faults"
)]
async fn update_status(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateStatusRequest>,
) -> Result<Json<FaultActionResponse>, (StatusCode, Json<ErrorResponse>)> {
    let tenant_id = require_tenant_id(&principal)?;
    require_manager(&state, principal.user_id, tenant_id).await?;

    // Get current fault to obtain current status
    let existing = match state.fault_repo.find_by_id_rls(&mut **rls.conn(), id).await {
        Ok(Some(f)) => f,
        Ok(None) => {
            rls.release().await;
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Fault not found")),
            ));
        }
        Err(e) => {
            tracing::error!("Failed to find fault: {}", e);
            rls.release().await;
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to find fault")),
            ));
        }
    };

    let data = UpdateFaultStatus {
        status: req.status,
        note: req.note,
        scheduled_date: req.scheduled_date,
        estimated_completion: req.estimated_completion,
    };

    let fault = state
        .fault_repo
        .update_status_rls(
            &mut **rls.conn(),
            id,
            principal.user_id,
            data,
            existing.status,
        )
        .await
        .map_err(|e| {
            tracing::error!("Failed to update status: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to update status",
                )),
            )
        })?;

    // Story 4.4: notify reporter on status change (push). Best-effort.
    if fault.reporter_id != principal.user_id {
        let notification = Notification::new(
            Uuid::nil(),
            NotificationCategory::Faults,
            format!("Fault status updated: {}", fault.title),
            format!(
                "Your fault report status has been updated to '{}'.",
                fault.status
            ),
        )
        .with_action_url(format!("/faults/{}", fault.id))
        .with_data(serde_json::json!({
            "fault_id": fault.id,
            "organization_id": fault.organization_id,
            "status": fault.status,
        }));
        if let Err(e) = state
            .notification_pipeline
            .dispatch(fault.reporter_id, &notification, Some(fault.id), None)
            .await
        {
            tracing::error!(
                fault_id = %fault.id,
                error = %e,
                "Failed to dispatch FaultStatusChanged notification"
            );
        }
    }

    rls.release().await;
    Ok(Json(FaultActionResponse {
        message: "Status updated successfully".to_string(),
        fault,
    }))
}

/// Resolve a fault (Story 4.4).
#[utoipa::path(
    post,
    path = "/api/v1/faults/{id}/resolve",
    params(
        ("id" = Uuid, Path, description = "Fault ID")
    ),
    request_body = ResolveFaultRequest,
    responses(
        (status = 200, description = "Fault resolved", body = FaultActionResponse),
        (status = 404, description = "Fault not found", body = ErrorResponse),
    ),
    tag = "Faults"
)]
async fn resolve_fault(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(id): Path<Uuid>,
    Json(req): Json<ResolveFaultRequest>,
) -> Result<Json<FaultActionResponse>, (StatusCode, Json<ErrorResponse>)> {
    let tenant_id = require_tenant_id(&principal)?;
    require_manager(&state, principal.user_id, tenant_id).await?;
    require_fault_in_org(&state, id, tenant_id).await?;

    let data = ResolveFault {
        resolution_notes: req.resolution_notes,
    };

    let fault = state
        .fault_repo
        .resolve(id, principal.user_id, data)
        .await
        .map_err(|e| {
            tracing::error!("Failed to resolve fault: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to resolve fault",
                )),
            )
        })?;

    // Story 4.5: notify reporter that their fault has been resolved. Best-effort.
    if fault.reporter_id != principal.user_id {
        let notification = Notification::new(
            Uuid::nil(),
            NotificationCategory::Faults,
            format!("Fault resolved: {}", fault.title),
            "Your fault report has been resolved. Please confirm if the issue was fixed."
                .to_string(),
        )
        .with_action_url(format!("/faults/{}", fault.id))
        .with_data(serde_json::json!({
            "fault_id": fault.id,
            "organization_id": fault.organization_id,
        }));
        if let Err(e) = state
            .notification_pipeline
            .dispatch(fault.reporter_id, &notification, Some(fault.id), None)
            .await
        {
            tracing::error!(
                fault_id = %fault.id,
                error = %e,
                "Failed to dispatch FaultResolved notification"
            );
        }
    }

    Ok(Json(FaultActionResponse {
        message: "Fault resolved successfully".to_string(),
        fault,
    }))
}

/// Confirm fault resolution (Story 4.6).
#[utoipa::path(
    post,
    path = "/api/v1/faults/{id}/confirm",
    params(
        ("id" = Uuid, Path, description = "Fault ID")
    ),
    request_body = ConfirmFaultRequest,
    responses(
        (status = 200, description = "Resolution confirmed", body = FaultActionResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 404, description = "Fault not found", body = ErrorResponse),
    ),
    tag = "Faults"
)]
async fn confirm_fault(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(id): Path<Uuid>,
    Json(req): Json<ConfirmFaultRequest>,
) -> Result<Json<FaultActionResponse>, (StatusCode, Json<ErrorResponse>)> {
    let tenant_id = require_tenant_id(&principal)?;
    require_fault_in_org(&state, id, tenant_id).await?;

    let data = ConfirmFault {
        rating: req.rating,
        feedback: req.feedback,
    };

    let fault = state
        .fault_repo
        .confirm(id, principal.user_id, data)
        .await
        .map_err(|e| {
            tracing::error!("Failed to confirm fault: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to confirm fault",
                )),
            )
        })?;

    // #1793: the reporter has confirmed (and rated) the resolution — typically
    // the closing transition. Notify the assignee who did the work plus the
    // org's managers so the lifecycle gets a closure signal. Best-effort.
    let manager_ids = match MembershipRepository::new(state.db.clone())
        .list_manager_ids(tenant_id)
        .await
    {
        Ok(manager_ids) => manager_ids,
        Err(e) => {
            tracing::error!(
                fault_id = %fault.id,
                error = %e,
                "Failed to load manager ids for FaultConfirmed notification"
            );
            Vec::new()
        }
    };
    // Recipient policy lives in the shared `confirm_fault_recipients` (#2029)
    // so the tests exercise this exact selection (assignee + managers, minus
    // the confirming reporter, deduplicated).
    let recipients = confirm_fault_recipients(fault.assigned_to, principal.user_id, manager_ids);
    if !recipients.is_empty() {
        let notification = Notification::new(
            Uuid::nil(),
            NotificationCategory::Faults,
            format!("Fault resolution confirmed: {}", fault.title),
            "The reporter has confirmed the fault resolution.".to_string(),
        )
        .with_action_url(format!("/faults/{}", fault.id))
        .with_data(serde_json::json!({
            "fault_id": fault.id,
            "organization_id": fault.organization_id,
        }));
        let results = state
            .notification_pipeline
            .dispatch_to_users(&recipients, &notification, Some(fault.id), None)
            .await;
        tracing::info!(
            fault_id = %fault.id,
            recipients = results.len(),
            "FaultConfirmed notifications dispatched"
        );
    }

    Ok(Json(FaultActionResponse {
        message: "Resolution confirmed successfully".to_string(),
        fault,
    }))
}

/// Reopen a fault (Story 4.6).
#[utoipa::path(
    post,
    path = "/api/v1/faults/{id}/reopen",
    params(
        ("id" = Uuid, Path, description = "Fault ID")
    ),
    request_body = ReopenFaultRequest,
    responses(
        (status = 200, description = "Fault reopened", body = FaultActionResponse),
        (status = 404, description = "Fault not found", body = ErrorResponse),
    ),
    tag = "Faults"
)]
async fn reopen_fault(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(id): Path<Uuid>,
    Json(req): Json<ReopenFaultRequest>,
) -> Result<Json<FaultActionResponse>, (StatusCode, Json<ErrorResponse>)> {
    let tenant_id = require_tenant_id(&principal)?;
    require_fault_in_org(&state, id, tenant_id).await?;

    let data = ReopenFault { reason: req.reason };

    let fault = state
        .fault_repo
        .reopen(id, principal.user_id, data)
        .await
        .map_err(|e| {
            tracing::error!("Failed to reopen fault: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to reopen fault",
                )),
            )
        })?;

    // Story 4.6: notify managers when a fault is reopened. Best-effort.
    match MembershipRepository::new(state.db.clone())
        .list_manager_ids(tenant_id)
        .await
    {
        Ok(manager_ids) => {
            let recipients = manager_recipients(manager_ids, principal.user_id);
            if !recipients.is_empty() {
                let notification = Notification::new(
                    Uuid::nil(),
                    NotificationCategory::Faults,
                    format!("Fault reopened: {}", fault.title),
                    "A resolved fault has been reopened and requires attention.".to_string(),
                )
                .with_action_url(format!("/faults/{}", fault.id))
                .with_data(serde_json::json!({
                    "fault_id": fault.id,
                    "organization_id": fault.organization_id,
                }));
                let results = state
                    .notification_pipeline
                    .dispatch_to_users(&recipients, &notification, Some(fault.id), None)
                    .await;
                tracing::info!(
                    fault_id = %fault.id,
                    recipients = results.len(),
                    "FaultReopened notifications dispatched to managers"
                );
            }
        }
        Err(e) => {
            tracing::error!(
                fault_id = %fault.id,
                error = %e,
                "Failed to load manager ids for FaultReopened notification"
            );
        }
    }

    Ok(Json(FaultActionResponse {
        message: "Fault reopened successfully".to_string(),
        fault,
    }))
}

/// List comments for a fault.
#[utoipa::path(
    get,
    path = "/api/v1/faults/{id}/comments",
    params(
        ("id" = Uuid, Path, description = "Fault ID")
    ),
    responses(
        (status = 200, description = "Timeline entries", body = TimelineResponse),
    ),
    tag = "Faults"
)]
async fn list_comments(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(id): Path<Uuid>,
) -> Result<Json<TimelineResponse>, (StatusCode, Json<ErrorResponse>)> {
    let tenant_id = require_tenant_id(&principal)?;
    // SECURITY (#770): scope the timeline read to the caller's org. The
    // `list_timeline` query runs on the raw pool (no RLS), so without this
    // guard a caller could read another org's fault timeline by UUID.
    require_fault_in_org(&state, id, tenant_id).await?;
    // P0-07: real role lookup (see get_fault above).
    let is_manager = MembershipRepository::new(state.db.clone())
        .is_manager_in_org(principal.user_id, tenant_id)
        .await
        .unwrap_or(false);

    match state.fault_repo.list_timeline(id, is_manager).await {
        Ok(entries) => Ok(Json(TimelineResponse { entries })),
        Err(e) => {
            tracing::error!("Failed to list timeline: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to list comments",
                )),
            ))
        }
    }
}

/// Add a comment to a fault.
#[utoipa::path(
    post,
    path = "/api/v1/faults/{id}/comments",
    params(
        ("id" = Uuid, Path, description = "Fault ID")
    ),
    request_body = AddCommentRequest,
    responses(
        (status = 201, description = "Comment added"),
        (status = 404, description = "Fault not found", body = ErrorResponse),
    ),
    tag = "Faults"
)]
async fn add_comment(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(id): Path<Uuid>,
    Json(req): Json<AddCommentRequest>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let tenant_id = require_tenant_id(&principal)?;
    require_fault_in_org(&state, id, tenant_id).await?;

    // SECURITY (#770): internal notes are manager-only. Read filtering was
    // already gated in P0-07; the write path was not. A non-manager asking to
    // mark a comment internal is downgraded to a normal comment rather than
    // silently creating a manager-visibility note (least surprise + no leak).
    let is_internal = if req.is_internal {
        MembershipRepository::new(state.db.clone())
            .is_manager_in_org(principal.user_id, tenant_id)
            .await
            .unwrap_or(false)
    } else {
        false
    };

    let data = AddFaultComment {
        note: req.note,
        is_internal,
    };

    state
        .fault_repo
        .add_comment(id, principal.user_id, data)
        .await
        .map_err(|e| {
            tracing::error!("Failed to add comment: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to add comment",
                )),
            )
        })?;

    Ok(StatusCode::CREATED)
}

/// Add a work note to a fault.
#[utoipa::path(
    post,
    path = "/api/v1/faults/{id}/work-notes",
    params(
        ("id" = Uuid, Path, description = "Fault ID")
    ),
    request_body = AddWorkNoteRequest,
    responses(
        (status = 201, description = "Work note added"),
        (status = 404, description = "Fault not found", body = ErrorResponse),
    ),
    tag = "Faults"
)]
async fn add_work_note(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(id): Path<Uuid>,
    Json(req): Json<AddWorkNoteRequest>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let tenant_id = require_tenant_id(&principal)?;
    require_fault_in_org(&state, id, tenant_id).await?;

    let data = AddWorkNote { note: req.note };

    state
        .fault_repo
        .add_work_note(id, principal.user_id, data)
        .await
        .map_err(|e| {
            tracing::error!("Failed to add work note: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to add work note",
                )),
            )
        })?;

    Ok(StatusCode::CREATED)
}

/// List attachments for a fault.
#[utoipa::path(
    get,
    path = "/api/v1/faults/{id}/attachments",
    params(
        ("id" = Uuid, Path, description = "Fault ID")
    ),
    responses(
        (status = 200, description = "Attachments list", body = AttachmentsResponse),
    ),
    tag = "Faults"
)]
async fn list_attachments(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(id): Path<Uuid>,
) -> Result<Json<AttachmentsResponse>, (StatusCode, Json<ErrorResponse>)> {
    // SECURITY (#770): the attachment list runs on the raw pool (no RLS), so
    // scope it to the caller's org before returning storage URLs / metadata.
    let tenant_id = require_tenant_id(&principal)?;
    require_fault_in_org(&state, id, tenant_id).await?;

    match state.fault_repo.list_attachments(id).await {
        Ok(attachments) => Ok(Json(AttachmentsResponse { attachments })),
        Err(e) => {
            tracing::error!("Failed to list attachments: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to list attachments",
                )),
            ))
        }
    }
}

/// Add an attachment to a fault.
#[utoipa::path(
    post,
    path = "/api/v1/faults/{id}/attachments",
    params(
        ("id" = Uuid, Path, description = "Fault ID")
    ),
    request_body = AddAttachmentRequest,
    responses(
        (status = 201, description = "Attachment added", body = FaultAttachment),
        (status = 404, description = "Fault not found", body = ErrorResponse),
    ),
    tag = "Faults"
)]
async fn add_attachment(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(id): Path<Uuid>,
    Json(req): Json<AddAttachmentRequest>,
) -> Result<(StatusCode, Json<FaultAttachment>), (StatusCode, Json<ErrorResponse>)> {
    let tenant_id = require_tenant_id(&principal)?;
    require_fault_in_org(&state, id, tenant_id).await?;

    let data = CreateFaultAttachment {
        fault_id: id,
        filename: req.filename.clone(),
        original_filename: req.original_filename,
        content_type: req.content_type,
        size_bytes: req.size_bytes,
        storage_url: req.storage_url,
        thumbnail_url: req.thumbnail_url,
        uploaded_by: principal.user_id,
        description: req.description,
        width: req.width,
        height: req.height,
    };

    let attachment = state.fault_repo.add_attachment(data).await.map_err(|e| {
        tracing::error!("Failed to add attachment: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "INTERNAL_ERROR",
                "Failed to add attachment",
            )),
        )
    })?;

    Ok((StatusCode::CREATED, Json(attachment)))
}

/// Delete an attachment.
#[utoipa::path(
    delete,
    path = "/api/v1/faults/{id}/attachments/{attachment_id}",
    params(
        ("id" = Uuid, Path, description = "Fault ID"),
        ("attachment_id" = Uuid, Path, description = "Attachment ID")
    ),
    responses(
        (status = 204, description = "Attachment deleted"),
        (status = 404, description = "Attachment not found", body = ErrorResponse),
    ),
    tag = "Faults"
)]
async fn delete_attachment(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path((id, attachment_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    // SECURITY (#770): scope the delete to the caller's org via the parent
    // fault. `delete_attachment` runs on the raw pool with no org predicate,
    // so without this guard any caller could delete another org's attachment
    // by enumerating its UUID.
    let tenant_id = require_tenant_id(&principal)?;
    require_fault_in_org(&state, id, tenant_id).await?;

    match state.fault_repo.delete_attachment(attachment_id).await {
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        Err(e) => {
            tracing::error!("Failed to delete attachment: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to delete attachment",
                )),
            ))
        }
    }
}

/// Get AI suggestion for a fault (Story 4.2).
#[utoipa::path(
    post,
    path = "/api/v1/faults/{id}/suggest",
    params(
        ("id" = Uuid, Path, description = "Fault ID")
    ),
    responses(
        (status = 200, description = "AI suggestion", body = AiSuggestionResponse),
        (status = 404, description = "Fault not found", body = ErrorResponse),
    ),
    tag = "Faults"
)]
async fn get_ai_suggestion(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<AiSuggestionResponse>, (StatusCode, Json<ErrorResponse>)> {
    // SECURITY (#970): AI triage suggestions are a manager-tier workflow action.
    let tenant_id = require_tenant_id(&principal)?;
    require_manager(&state, principal.user_id, tenant_id).await?;

    // Get fault to analyze
    let fault = match state.fault_repo.find_by_id_rls(&mut **rls.conn(), id).await {
        Ok(Some(f)) => f,
        Ok(None) => {
            rls.release().await;
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Fault not found")),
            ));
        }
        Err(e) => {
            tracing::error!("Failed to get fault: {}", e);
            rls.release().await;
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to get fault")),
            ));
        }
    };

    // Simple keyword-based suggestion (real ML in Phase 3)
    let description_lower = fault.description.to_lowercase();
    let title_lower = fault.title.to_lowercase();
    let combined = format!("{} {}", title_lower, description_lower);

    let (category, confidence) = if combined.contains("water")
        || combined.contains("pipe")
        || combined.contains("leak")
        || combined.contains("faucet")
        || combined.contains("drain")
        || combined.contains("toilet")
    {
        ("plumbing", 0.85)
    } else if combined.contains("electric")
        || combined.contains("power")
        || combined.contains("outlet")
        || combined.contains("light")
        || combined.contains("switch")
        || combined.contains("wire")
    {
        ("electrical", 0.82)
    } else if combined.contains("heat")
        || combined.contains("cold")
        || combined.contains("radiator")
        || combined.contains("thermostat")
        || combined.contains("boiler")
        || combined.contains("furnace")
    {
        ("heating", 0.80)
    } else if combined.contains("crack")
        || combined.contains("wall")
        || combined.contains("foundation")
        || combined.contains("ceiling")
        || combined.contains("floor")
        || combined.contains("structural")
    {
        ("structural", 0.75)
    } else if combined.contains("roof")
        || combined.contains("window")
        || combined.contains("door")
        || combined.contains("facade")
        || combined.contains("balcony")
        || combined.contains("exterior")
    {
        ("exterior", 0.78)
    } else if combined.contains("elevator") || combined.contains("lift") {
        ("elevator", 0.90)
    } else if combined.contains("hallway")
        || combined.contains("lobby")
        || combined.contains("staircase")
        || combined.contains("common")
        || combined.contains("garage")
        || combined.contains("parking")
    {
        ("common_area", 0.70)
    } else if combined.contains("security")
        || combined.contains("lock")
        || combined.contains("key")
        || combined.contains("intercom")
        || combined.contains("camera")
    {
        ("security", 0.75)
    } else if combined.contains("clean")
        || combined.contains("trash")
        || combined.contains("garbage")
        || combined.contains("dirty")
    {
        ("cleaning", 0.72)
    } else {
        ("other", 0.50)
    };

    // Determine priority based on keywords
    let priority = if combined.contains("urgent")
        || combined.contains("emergency")
        || combined.contains("dangerous")
        || combined.contains("flood")
        || combined.contains("fire")
    {
        Some("urgent".to_string())
    } else if combined.contains("broken") || combined.contains("not working") {
        Some("high".to_string())
    } else {
        None
    };

    // Update fault with AI suggestion - log failures for debugging
    if let Err(e) = state
        .fault_repo
        .update_ai_suggestion(id, category, priority.as_deref(), confidence)
        .await
    {
        tracing::warn!(
            fault_id = %id,
            category = %category,
            error = %e,
            "Failed to persist AI suggestion for fault"
        );
    }

    rls.release().await;
    Ok(Json(AiSuggestionResponse {
        suggestion: AiSuggestion {
            category: category.to_string(),
            confidence,
            priority,
        },
    }))
}

/// Get fault statistics (Story 4.7).
#[utoipa::path(
    get,
    path = "/api/v1/faults/statistics",
    params(StatisticsQuery),
    responses(
        (status = 200, description = "Fault statistics", body = StatisticsResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "Faults"
)]
async fn get_statistics(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Query(query): Query<StatisticsQuery>,
) -> Result<Json<StatisticsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let tenant_id = require_tenant_id(&principal)?;

    let statistics = state
        .fault_repo
        .get_statistics(tenant_id, query.building_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get statistics: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to get statistics",
                )),
            )
        })?;

    Ok(Json(StatisticsResponse { statistics }))
}

// ============================================================================
// Recipient-policy unit tests (#2054)
// ============================================================================
//
// The DB-backed R7/R8 integration cases in
// `tests/fault_notification_recipient_tests.rs` seed *distinct* actor /
// reporter / assignee, so the actor is never in the candidate set and the
// self-exclusion (`!= actor_id`) and dedup (`!recipients.contains(..)`)
// branches never fire — a regression that deleted them would keep those
// tests green (#2054). These pure-function cases put the actor *into* the
// candidate set to exercise exactly those branches. They need no DB.
#[cfg(test)]
mod recipient_policy_tests {
    use super::{
        assign_fault_recipients, confirm_fault_recipients, manager_recipients,
        triage_fault_recipients,
    };
    use uuid::Uuid;

    // --- triage_fault_recipients -------------------------------------------

    /// Self-exclusion via the reporter path: the triaging manager IS the
    /// reporter, so `reporter_id == actor_id` must drop them.
    #[test]
    fn triage_excludes_actor_when_reporter_is_the_triaging_manager() {
        let actor = Uuid::new_v4(); // triaging manager == reporter
        let assignee = Uuid::new_v4();

        let recipients = triage_fault_recipients(actor, actor, Some(assignee));

        assert!(
            !recipients.contains(&actor),
            "reporter == actor must be excluded (self-notify guard)"
        );
        assert_eq!(recipients, vec![assignee]);
    }

    /// Self-exclusion via the assignee path: the fault is self-assigned to the
    /// actor, so `assignee != actor_id` must drop them.
    #[test]
    fn triage_excludes_actor_when_self_assigned() {
        let actor = Uuid::new_v4();
        let reporter = Uuid::new_v4();

        let recipients = triage_fault_recipients(reporter, actor, Some(actor));

        assert!(
            !recipients.contains(&actor),
            "self-assigned actor must be excluded"
        );
        assert_eq!(recipients, vec![reporter]);
    }

    /// Dedup: reporter and assignee are the same person, so
    /// `!recipients.contains(&assignee)` must keep them to a single entry.
    #[test]
    fn triage_dedups_when_reporter_is_also_assignee() {
        let actor = Uuid::new_v4();
        let both = Uuid::new_v4();

        let recipients = triage_fault_recipients(both, actor, Some(both));

        assert_eq!(
            recipients,
            vec![both],
            "reporter == assignee must appear exactly once"
        );
        assert_eq!(recipients.len(), 1);
    }

    // --- confirm_fault_recipients ------------------------------------------

    /// Self-exclusion via `manager_ids`: the confirming reporter is also a
    /// seeded manager, so `mid != actor_id` must drop the actor.
    #[test]
    fn confirm_excludes_actor_present_in_manager_ids() {
        let actor = Uuid::new_v4(); // confirming reporter, also a manager
        let assignee = Uuid::new_v4();
        let other_manager = Uuid::new_v4();

        let recipients = confirm_fault_recipients(Some(assignee), actor, [actor, other_manager]);

        assert!(
            !recipients.contains(&actor),
            "actor present in manager_ids must be excluded"
        );
        assert_eq!(recipients, vec![assignee, other_manager]);
    }

    /// Dedup: a manager id equals the assignee already collected, so
    /// `!recipients.contains(&mid)` must keep them to a single entry.
    #[test]
    fn confirm_dedups_manager_equal_to_assignee() {
        let actor = Uuid::new_v4();
        let assignee = Uuid::new_v4(); // also listed in manager_ids

        let recipients = confirm_fault_recipients(Some(assignee), actor, [assignee]);

        assert_eq!(
            recipients,
            vec![assignee],
            "assignee also listed as manager must appear exactly once"
        );
        assert_eq!(recipients.len(), 1);
    }

    // --- assign_fault_recipients -------------------------------------------

    /// Self-exclusion via the assignee path: the assigning manager assigned the
    /// fault to themselves, so `assignee != actor_id` must drop them. The
    /// guard-removed set would include the actor.
    #[test]
    fn assign_excludes_actor_when_self_assigned() {
        let actor = Uuid::new_v4(); // assigning manager == assignee
        let reporter = Uuid::new_v4();

        let recipients = assign_fault_recipients(Some(actor), reporter, actor);

        assert!(
            !recipients.contains(&actor),
            "self-assigned actor must be excluded"
        );
        assert_eq!(recipients, vec![reporter]);
        // Guard-removed output (naive assignee + reporter) would differ.
        assert_eq!(recipients.len(), 1);
    }

    /// Self-exclusion via the reporter path: the assigning manager is also the
    /// reporter, so `reporter_id != actor_id` must drop them.
    #[test]
    fn assign_excludes_actor_when_reporter_is_the_assigning_manager() {
        let actor = Uuid::new_v4(); // assigning manager == reporter
        let assignee = Uuid::new_v4();

        let recipients = assign_fault_recipients(Some(assignee), actor, actor);

        assert!(
            !recipients.contains(&actor),
            "reporter == actor must be excluded (self-notify guard)"
        );
        assert_eq!(recipients, vec![assignee]);
        assert_eq!(recipients.len(), 1);
    }

    /// Dedup: reporter and assignee are the same non-actor person, so
    /// `Some(reporter_id) != assigned_to` must keep them to a single entry.
    #[test]
    fn assign_dedups_when_reporter_is_also_assignee() {
        let actor = Uuid::new_v4();
        let both = Uuid::new_v4();

        let recipients = assign_fault_recipients(Some(both), both, actor);

        assert_eq!(
            recipients,
            vec![both],
            "reporter == assignee must appear exactly once"
        );
        // Guard-removed output would push `both` twice (len 2); the dedup guard
        // keeps it at one.
        assert_eq!(recipients.len(), 1);
    }

    // --- manager_recipients ------------------------------------------------

    /// Self-exclusion + dedup for the manager broadcast: the acting manager
    /// appears in the list (dropped) and a duplicate id collapses to one.
    #[test]
    fn manager_recipients_excludes_actor_and_dedups() {
        let actor = Uuid::new_v4();
        let manager = Uuid::new_v4();

        let recipients = manager_recipients([actor, manager, manager], actor);

        assert!(
            !recipients.contains(&actor),
            "acting manager must be excluded"
        );
        assert_eq!(recipients, vec![manager]);
        // Guard-removed output (raw list) would be len 3; the guard yields 1.
        assert_eq!(recipients.len(), 1);
    }
}
