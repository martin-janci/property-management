//! Violation Tracking & Enforcement routes for Epic 142.

use crate::state::AppState;
use api_core::extractors::AuthUser;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post},
    Json, Router,
};
use common::errors::ErrorResponse;
use db::models::violations::{
    AppealQuery, CreateCommunityRule, CreateEnforcementAction, CreateViolation,
    CreateViolationAppeal, CreateViolationComment, CreateViolationEvidence, EnforcementQuery,
    RecordFinePayment, UpdateCommunityRule, UpdateEnforcementAction, UpdateViolation,
    UpdateViolationAppeal, ViolationQuery,
};
use rust_decimal::Decimal;
use serde::Deserialize;
use uuid::Uuid;

type ApiResult<T> = Result<T, (StatusCode, Json<ErrorResponse>)>;

fn internal_error(msg: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse::internal_error(msg)),
    )
}

fn not_found_error(msg: &str) -> (StatusCode, Json<ErrorResponse>) {
    (StatusCode::NOT_FOUND, Json(ErrorResponse::not_found(msg)))
}

fn forbidden_error(msg: &str) -> (StatusCode, Json<ErrorResponse>) {
    (StatusCode::FORBIDDEN, Json(ErrorResponse::forbidden(msg)))
}

/// Create the violations router.
pub fn router() -> Router<AppState> {
    Router::new()
        // Community rules
        .route("/rules", get(list_rules).post(create_rule))
        .route(
            "/rules/{rule_id}",
            get(get_rule).put(update_rule).delete(delete_rule),
        )
        // Violations
        .route("/", get(list_violations).post(create_violation))
        .route("/{violation_id}", get(get_violation).put(update_violation))
        .route("/{violation_id}/assign", post(assign_violation))
        // Evidence
        .route(
            "/{violation_id}/evidence",
            get(list_evidence).post(add_evidence),
        )
        .route(
            "/{violation_id}/evidence/{evidence_id}",
            delete(delete_evidence),
        )
        // Enforcement actions
        .route(
            "/{violation_id}/actions",
            get(list_actions).post(create_action),
        )
        .route(
            "/{violation_id}/actions/{action_id}",
            get(get_action).put(update_action),
        )
        .route(
            "/{violation_id}/actions/{action_id}/send",
            post(mark_action_sent),
        )
        .route(
            "/{violation_id}/actions/{action_id}/payments",
            get(list_payments).post(record_payment),
        )
        // Appeals
        .route("/{violation_id}/appeals", post(create_appeal))
        .route("/appeals", get(list_appeals))
        .route("/appeals/{appeal_id}", get(get_appeal).put(update_appeal))
        .route("/appeals/{appeal_id}/decide", post(decide_appeal))
        // Comments
        .route(
            "/{violation_id}/comments",
            get(list_comments).post(add_comment),
        )
        // Dashboard & Reports
        .route("/dashboard", get(get_dashboard))
        .route("/history", get(get_violator_history))
        .route("/statistics", get(get_statistics))
}

// =============================================================================
// COMMUNITY RULES
// =============================================================================

async fn create_rule(
    State(state): State<AppState>,
    user: AuthUser,
    Json(req): Json<CreateCommunityRule>,
) -> ApiResult<Json<db::models::violations::CommunityRule>> {
    let org_id = user
        .tenant_id
        .ok_or_else(|| forbidden_error("No organization context"))?;

    state
        .violation_repo
        .create_rule(org_id, req, user.user_id)
        .await
        .map(Json)
        .map_err(|e| internal_error(&format!("Failed to create rule: {}", e)))
}

async fn get_rule(
    State(state): State<AppState>,
    user: AuthUser,
    Path(rule_id): Path<Uuid>,
) -> ApiResult<Json<db::models::violations::CommunityRule>> {
    let org_id = user
        .tenant_id
        .ok_or_else(|| forbidden_error("No organization context"))?;

    state
        .violation_repo
        .get_rule_for_org(rule_id, org_id)
        .await
        .map_err(|e| internal_error(&format!("Failed to get rule: {}", e)))?
        .ok_or_else(|| not_found_error("Rule not found"))
        .map(Json)
}

#[derive(Debug, Deserialize)]
struct ListRulesQuery {
    building_id: Option<Uuid>,
    active_only: Option<bool>,
}

async fn list_rules(
    State(state): State<AppState>,
    user: AuthUser,
    Query(query): Query<ListRulesQuery>,
) -> ApiResult<Json<Vec<db::models::violations::CommunityRule>>> {
    let org_id = user
        .tenant_id
        .ok_or_else(|| forbidden_error("No organization context"))?;

    state
        .violation_repo
        .list_rules(
            org_id,
            query.building_id,
            query.active_only.unwrap_or(false),
        )
        .await
        .map(Json)
        .map_err(|e| internal_error(&format!("Failed to list rules: {}", e)))
}

async fn update_rule(
    State(state): State<AppState>,
    user: AuthUser,
    Path(rule_id): Path<Uuid>,
    Json(req): Json<UpdateCommunityRule>,
) -> ApiResult<Json<db::models::violations::CommunityRule>> {
    let org_id = user
        .tenant_id
        .ok_or_else(|| forbidden_error("No organization context"))?;

    state
        .violation_repo
        .update_rule(rule_id, org_id, req)
        .await
        .map(Json)
        .map_err(|e| internal_error(&format!("Failed to update rule: {}", e)))
}

async fn delete_rule(
    State(state): State<AppState>,
    user: AuthUser,
    Path(rule_id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    let org_id = user
        .tenant_id
        .ok_or_else(|| forbidden_error("No organization context"))?;

    let deleted = state
        .violation_repo
        .delete_rule(rule_id, org_id)
        .await
        .map_err(|e| internal_error(&format!("Failed to delete rule: {}", e)))?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(not_found_error("Rule not found"))
    }
}

// =============================================================================
// VIOLATIONS
// =============================================================================

async fn create_violation(
    State(state): State<AppState>,
    user: AuthUser,
    Json(req): Json<CreateViolation>,
) -> ApiResult<Json<db::models::violations::Violation>> {
    let org_id = user
        .tenant_id
        .ok_or_else(|| forbidden_error("No organization context"))?;

    state
        .violation_repo
        .create_violation(org_id, req, user.user_id)
        .await
        .map(Json)
        .map_err(|e| internal_error(&format!("Failed to create violation: {}", e)))
}

async fn get_violation(
    State(state): State<AppState>,
    user: AuthUser,
    Path(violation_id): Path<Uuid>,
) -> ApiResult<Json<db::models::violations::Violation>> {
    let org_id = user
        .tenant_id
        .ok_or_else(|| forbidden_error("No organization context"))?;

    state
        .violation_repo
        .get_violation_for_org(violation_id, org_id)
        .await
        .map_err(|e| internal_error(&format!("Failed to get violation: {}", e)))?
        .ok_or_else(|| not_found_error("Violation not found"))
        .map(Json)
}

async fn list_violations(
    State(state): State<AppState>,
    user: AuthUser,
    Query(query): Query<ViolationQuery>,
) -> ApiResult<Json<Vec<db::models::violations::ViolationSummary>>> {
    let org_id = user
        .tenant_id
        .ok_or_else(|| forbidden_error("No organization context"))?;

    state
        .violation_repo
        .list_violation_summaries(org_id, query)
        .await
        .map(Json)
        .map_err(|e| internal_error(&format!("Failed to list violations: {}", e)))
}

async fn update_violation(
    State(state): State<AppState>,
    user: AuthUser,
    Path(violation_id): Path<Uuid>,
    Json(req): Json<UpdateViolation>,
) -> ApiResult<Json<db::models::violations::Violation>> {
    let org_id = user
        .tenant_id
        .ok_or_else(|| forbidden_error("No organization context"))?;

    state
        .violation_repo
        .update_violation(violation_id, org_id, req)
        .await
        .map(Json)
        .map_err(|e| internal_error(&format!("Failed to update violation: {}", e)))
}

#[derive(Debug, Deserialize)]
struct AssignRequest {
    assigned_to: Uuid,
}

async fn assign_violation(
    State(state): State<AppState>,
    user: AuthUser,
    Path(violation_id): Path<Uuid>,
    Json(req): Json<AssignRequest>,
) -> ApiResult<Json<db::models::violations::Violation>> {
    let org_id = user
        .tenant_id
        .ok_or_else(|| forbidden_error("No organization context"))?;

    state
        .violation_repo
        .assign_violation(violation_id, org_id, req.assigned_to)
        .await
        .map(Json)
        .map_err(|e| internal_error(&format!("Failed to assign violation: {}", e)))
}

// =============================================================================
// EVIDENCE
// =============================================================================

async fn add_evidence(
    State(state): State<AppState>,
    user: AuthUser,
    Path(violation_id): Path<Uuid>,
    Json(req): Json<CreateViolationEvidence>,
) -> ApiResult<Json<db::models::violations::ViolationEvidence>> {
    state
        .violation_repo
        .add_evidence(violation_id, req, user.user_id)
        .await
        .map(Json)
        .map_err(|e| internal_error(&format!("Failed to add evidence: {}", e)))
}

async fn list_evidence(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(violation_id): Path<Uuid>,
) -> ApiResult<Json<Vec<db::models::violations::ViolationEvidence>>> {
    state
        .violation_repo
        .list_evidence(violation_id)
        .await
        .map(Json)
        .map_err(|e| internal_error(&format!("Failed to list evidence: {}", e)))
}

async fn delete_evidence(
    State(state): State<AppState>,
    user: AuthUser,
    Path((_violation_id, evidence_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<StatusCode> {
    let org_id = user
        .tenant_id
        .ok_or_else(|| forbidden_error("No organization context"))?;

    let deleted = state
        .violation_repo
        .delete_evidence(evidence_id, org_id)
        .await
        .map_err(|e| internal_error(&format!("Failed to delete evidence: {}", e)))?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(not_found_error("Evidence not found"))
    }
}

// =============================================================================
// ENFORCEMENT ACTIONS
// =============================================================================

async fn create_action(
    State(state): State<AppState>,
    user: AuthUser,
    Path(violation_id): Path<Uuid>,
    Json(req): Json<CreateEnforcementAction>,
) -> ApiResult<Json<db::models::violations::EnforcementAction>> {
    let org_id = user
        .tenant_id
        .ok_or_else(|| forbidden_error("No organization context"))?;

    state
        .violation_repo
        .create_enforcement_action(violation_id, org_id, req, user.user_id)
        .await
        .map(Json)
        .map_err(|e| internal_error(&format!("Failed to create action: {}", e)))
}

async fn get_action(
    State(state): State<AppState>,
    user: AuthUser,
    Path((_violation_id, action_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<db::models::violations::EnforcementAction>> {
    let org_id = user
        .tenant_id
        .ok_or_else(|| forbidden_error("No organization context"))?;

    state
        .violation_repo
        .get_enforcement_action_for_org(action_id, org_id)
        .await
        .map_err(|e| internal_error(&format!("Failed to get action: {}", e)))?
        .ok_or_else(|| not_found_error("Action not found"))
        .map(Json)
}

async fn list_actions(
    State(state): State<AppState>,
    user: AuthUser,
    Path(violation_id): Path<Uuid>,
    Query(mut query): Query<EnforcementQuery>,
) -> ApiResult<Json<Vec<db::models::violations::EnforcementAction>>> {
    let org_id = user
        .tenant_id
        .ok_or_else(|| forbidden_error("No organization context"))?;
    query.violation_id = Some(violation_id);

    state
        .violation_repo
        .list_enforcement_actions(org_id, query)
        .await
        .map(Json)
        .map_err(|e| internal_error(&format!("Failed to list actions: {}", e)))
}

async fn update_action(
    State(state): State<AppState>,
    user: AuthUser,
    Path((_violation_id, action_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdateEnforcementAction>,
) -> ApiResult<Json<db::models::violations::EnforcementAction>> {
    let org_id = user
        .tenant_id
        .ok_or_else(|| forbidden_error("No organization context"))?;

    state
        .violation_repo
        .update_enforcement_action(action_id, org_id, req)
        .await
        .map(Json)
        .map_err(|e| internal_error(&format!("Failed to update action: {}", e)))
}

#[derive(Debug, Deserialize)]
struct SendNoticeRequest {
    method: String,
}

async fn mark_action_sent(
    State(state): State<AppState>,
    user: AuthUser,
    Path((_violation_id, action_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<SendNoticeRequest>,
) -> ApiResult<Json<db::models::violations::EnforcementAction>> {
    let org_id = user
        .tenant_id
        .ok_or_else(|| forbidden_error("No organization context"))?;

    state
        .violation_repo
        .mark_action_sent(action_id, org_id, &req.method)
        .await
        .map(Json)
        .map_err(|e| internal_error(&format!("Failed to mark action sent: {}", e)))
}

async fn record_payment(
    State(state): State<AppState>,
    user: AuthUser,
    Path((_violation_id, action_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<RecordFinePayment>,
) -> ApiResult<Json<db::models::violations::FinePayment>> {
    let org_id = user
        .tenant_id
        .ok_or_else(|| forbidden_error("No organization context"))?;

    state
        .violation_repo
        .record_payment(action_id, org_id, req, user.user_id)
        .await
        .map(Json)
        .map_err(|e| internal_error(&format!("Failed to record payment: {}", e)))
}

async fn list_payments(
    State(state): State<AppState>,
    _user: AuthUser,
    Path((_violation_id, action_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<Vec<db::models::violations::FinePayment>>> {
    state
        .violation_repo
        .list_payments(action_id)
        .await
        .map(Json)
        .map_err(|e| internal_error(&format!("Failed to list payments: {}", e)))
}

// =============================================================================
// APPEALS
// =============================================================================

async fn create_appeal(
    State(state): State<AppState>,
    user: AuthUser,
    Path(violation_id): Path<Uuid>,
    Json(req): Json<CreateViolationAppeal>,
) -> ApiResult<Json<db::models::violations::ViolationAppeal>> {
    let org_id = user
        .tenant_id
        .ok_or_else(|| forbidden_error("No organization context"))?;

    state
        .violation_repo
        .create_appeal(violation_id, org_id, req, user.user_id)
        .await
        .map(Json)
        .map_err(|e| internal_error(&format!("Failed to create appeal: {}", e)))
}

async fn get_appeal(
    State(state): State<AppState>,
    user: AuthUser,
    Path(appeal_id): Path<Uuid>,
) -> ApiResult<Json<db::models::violations::ViolationAppeal>> {
    let org_id = user
        .tenant_id
        .ok_or_else(|| forbidden_error("No organization context"))?;

    state
        .violation_repo
        .get_appeal_for_org(appeal_id, org_id)
        .await
        .map_err(|e| internal_error(&format!("Failed to get appeal: {}", e)))?
        .ok_or_else(|| not_found_error("Appeal not found"))
        .map(Json)
}

async fn list_appeals(
    State(state): State<AppState>,
    user: AuthUser,
    Query(query): Query<AppealQuery>,
) -> ApiResult<Json<Vec<db::models::violations::ViolationAppeal>>> {
    let org_id = user
        .tenant_id
        .ok_or_else(|| forbidden_error("No organization context"))?;

    state
        .violation_repo
        .list_appeals(org_id, query)
        .await
        .map(Json)
        .map_err(|e| internal_error(&format!("Failed to list appeals: {}", e)))
}

async fn update_appeal(
    State(state): State<AppState>,
    user: AuthUser,
    Path(appeal_id): Path<Uuid>,
    Json(req): Json<UpdateViolationAppeal>,
) -> ApiResult<Json<db::models::violations::ViolationAppeal>> {
    let org_id = user
        .tenant_id
        .ok_or_else(|| forbidden_error("No organization context"))?;

    state
        .violation_repo
        .update_appeal(appeal_id, org_id, req, Some(user.user_id))
        .await
        .map(Json)
        .map_err(|e| internal_error(&format!("Failed to update appeal: {}", e)))
}

#[derive(Debug, Deserialize)]
struct DecideAppealRequest {
    approved: bool,
    decision: String,
    fine_adjustment: Option<Decimal>,
}

async fn decide_appeal(
    State(state): State<AppState>,
    user: AuthUser,
    Path(appeal_id): Path<Uuid>,
    Json(req): Json<DecideAppealRequest>,
) -> ApiResult<Json<db::models::violations::ViolationAppeal>> {
    let org_id = user
        .tenant_id
        .ok_or_else(|| forbidden_error("No organization context"))?;

    state
        .violation_repo
        .decide_appeal(
            appeal_id,
            org_id,
            req.approved,
            &req.decision,
            req.fine_adjustment,
            user.user_id,
        )
        .await
        .map(Json)
        .map_err(|e| internal_error(&format!("Failed to decide appeal: {}", e)))
}

// =============================================================================
// COMMENTS
// =============================================================================

async fn add_comment(
    State(state): State<AppState>,
    user: AuthUser,
    Path(violation_id): Path<Uuid>,
    Json(req): Json<CreateViolationComment>,
) -> ApiResult<Json<db::models::violations::ViolationComment>> {
    state
        .violation_repo
        .add_comment(violation_id, req, user.user_id)
        .await
        .map(Json)
        .map_err(|e| internal_error(&format!("Failed to add comment: {}", e)))
}

#[derive(Debug, Deserialize)]
struct ListCommentsQuery {
    include_internal: Option<bool>,
}

async fn list_comments(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(violation_id): Path<Uuid>,
    Query(query): Query<ListCommentsQuery>,
) -> ApiResult<Json<Vec<db::models::violations::ViolationComment>>> {
    state
        .violation_repo
        .list_comments(violation_id, query.include_internal.unwrap_or(false))
        .await
        .map(Json)
        .map_err(|e| internal_error(&format!("Failed to list comments: {}", e)))
}

// =============================================================================
// DASHBOARD & REPORTS
// =============================================================================

async fn get_dashboard(
    State(state): State<AppState>,
    user: AuthUser,
) -> ApiResult<Json<db::models::violations::ViolationDashboard>> {
    let org_id = user
        .tenant_id
        .ok_or_else(|| forbidden_error("No organization context"))?;

    state
        .violation_repo
        .get_dashboard(org_id)
        .await
        .map(Json)
        .map_err(|e| internal_error(&format!("Failed to get dashboard: {}", e)))
}

#[derive(Debug, Deserialize)]
struct ViolatorHistoryQuery {
    violator_id: Option<Uuid>,
    unit_id: Option<Uuid>,
}

async fn get_violator_history(
    State(state): State<AppState>,
    user: AuthUser,
    Query(query): Query<ViolatorHistoryQuery>,
) -> ApiResult<Json<db::models::violations::ViolatorHistory>> {
    let org_id = user
        .tenant_id
        .ok_or_else(|| forbidden_error("No organization context"))?;

    state
        .violation_repo
        .get_violator_history(org_id, query.violator_id, query.unit_id)
        .await
        .map(Json)
        .map_err(|e| internal_error(&format!("Failed to get violator history: {}", e)))
}

#[derive(Debug, Deserialize)]
struct StatisticsQuery {
    building_id: Option<Uuid>,
    period_type: Option<String>,
}

async fn get_statistics(
    State(state): State<AppState>,
    user: AuthUser,
    Query(query): Query<StatisticsQuery>,
) -> ApiResult<Json<Option<db::models::violations::ViolationStatistics>>> {
    let org_id = user
        .tenant_id
        .ok_or_else(|| forbidden_error("No organization context"))?;

    state
        .violation_repo
        .get_statistics(
            org_id,
            query.building_id,
            &query.period_type.unwrap_or_else(|| "monthly".to_string()),
        )
        .await
        .map(Json)
        .map_err(|e| internal_error(&format!("Failed to get statistics: {}", e)))
}
