//! Story 67.4: Content Moderation Dashboard endpoints.

use crate::state::AppState;
use api_core::extractors::AuthUser;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use db::models::compliance::{
    CreateModerationCase, ModeratedContentType, ModerationActionType, ModerationCase,
    ModerationStatus, TakeModerationAction, ViolationType,
};
use db::models::AuditAction;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::dsa::ViolationTypeCountResponse;
use super::shared::*;

/// Content owner info.
#[derive(Debug, Serialize)]
pub struct ContentOwnerInfo {
    pub user_id: Uuid,
    pub name: String,
    pub previous_violations: i32,
}

/// Moderation case response.
#[derive(Debug, Serialize)]
pub struct ModerationCaseResponse {
    pub id: Uuid,
    pub content_type: ModeratedContentType,
    pub content_id: Uuid,
    pub content_preview: Option<String>,
    pub content_owner: ContentOwnerInfo,
    pub report_source: String,
    pub violation_type: Option<ViolationType>,
    pub report_reason: Option<String>,
    pub status: ModerationStatus,
    pub priority: i32,
    pub assigned_to_name: Option<String>,
    pub decision: Option<ModerationActionType>,
    pub decision_rationale: Option<String>,
    pub appeal_filed: bool,
    pub appeal_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub age_hours: f64,
}

impl ModerationCaseResponse {
    /// Build a response DTO from a persisted case.
    ///
    /// Centralizes the `age_hours` computation and the content-owner /
    /// assignee presentation fields that were previously duplicated verbatim
    /// across every moderation handler. `now` is passed in (rather than read
    /// from `Utc::now()` internally) so callers that map a whole page share a
    /// single timestamp and so the age math is unit-testable.
    ///
    /// Behavior is identical to the former inline construction: `owner_name`
    /// and `previous_violations` populate `content_owner`, `assigned_to_name`
    /// is passed through as-is, and `age_hours` is minutes-since-`created_at`
    /// divided by 60.
    fn from_case(
        case: ModerationCase,
        now: DateTime<Utc>,
        owner_name: &str,
        previous_violations: i32,
        assigned_to_name: Option<String>,
    ) -> Self {
        let age_hours = (now - case.created_at).num_minutes() as f64 / 60.0;
        Self {
            id: case.id,
            content_type: case.content_type,
            content_id: case.content_id,
            content_preview: case.content_preview,
            content_owner: ContentOwnerInfo {
                user_id: case.content_owner_id,
                name: owner_name.to_string(),
                previous_violations,
            },
            report_source: case.report_source.to_string(),
            violation_type: case.violation_type,
            report_reason: case.report_reason,
            status: case.status,
            priority: case.priority,
            assigned_to_name,
            decision: case.decision,
            decision_rationale: case.decision_rationale,
            appeal_filed: case.appeal_filed,
            appeal_reason: case.appeal_reason,
            created_at: case.created_at,
            age_hours,
        }
    }
}

/// Moderation queue query parameters.
#[derive(Debug, Deserialize)]
pub struct ModerationQueueQuery {
    pub status: Option<ModerationStatus>,
    pub content_type: Option<ModeratedContentType>,
    pub violation_type: Option<ViolationType>,
    pub priority: Option<i32>,
    pub assigned_to: Option<Uuid>,
    pub unassigned_only: Option<bool>,
    /// When true, restrict to still-open cases that have breached the 24h SLA
    /// (`status IN (pending, under_review) AND created_at < NOW() - 24h`).
    /// Applied server-side so the result matches `overdue_count` in
    /// `get_moderation_queue_stats` instead of narrowing a truncated page.
    pub overdue: Option<bool>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Get moderation queue.
pub(super) async fn get_moderation_queue(
    State(state): State<AppState>,
    user: AuthUser,
    Query(params): Query<ModerationQueueQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    require_moderator_role(&user)?;

    let org_id = user.tenant_id.ok_or((
        StatusCode::BAD_REQUEST,
        "Organization context required".to_string(),
    ))?;

    let limit = clamp_limit(params.limit);
    let offset = sanitize_offset(params.offset);
    let unassigned_only = params.unassigned_only.unwrap_or(false);
    let overdue_only = params.overdue.unwrap_or(false);

    let (cases, total) = state
        .compliance_repo
        .list_moderation_cases(
            org_id,
            params.status,
            params.content_type,
            params.violation_type,
            params.priority,
            params.assigned_to,
            unassigned_only,
            overdue_only,
            params.sort_by.as_deref(),
            params.sort_order.as_deref(),
            limit,
            offset,
        )
        .await
        .map_err(|e| {
            tracing::error!("Failed to list moderation cases: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to list cases".to_string(),
            )
        })?;

    let now = Utc::now();
    let responses: Vec<ModerationCaseResponse> = cases
        .into_iter()
        // name / previous_violations / assigned_to_name would be enriched from
        // the user repo; kept as placeholders here as in the original handler.
        .map(|c| ModerationCaseResponse::from_case(c, now, "User", 0, None))
        .collect();

    Ok(Json(serde_json::json!({
        "cases": responses,
        "total": total,
        "limit": limit,
        "offset": offset
    })))
}

/// Priority count.
#[derive(Debug, Serialize)]
pub struct PriorityCount {
    pub priority: i32,
    pub count: i64,
}

/// Moderation queue statistics.
#[derive(Debug, Serialize)]
pub struct ModerationQueueStatsResponse {
    pub pending_count: i64,
    pub under_review_count: i64,
    pub by_priority: Vec<PriorityCount>,
    pub by_violation_type: Vec<ViolationTypeCountResponse>,
    pub avg_resolution_time_hours: f64,
    pub overdue_count: i64,
}

/// Get moderation queue statistics.
pub(super) async fn get_moderation_stats(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<ModerationQueueStatsResponse>, (StatusCode, String)> {
    require_moderator_role(&user)?;

    let org_id = user.tenant_id.ok_or((
        StatusCode::BAD_REQUEST,
        "Organization context required".to_string(),
    ))?;

    let stats = state
        .compliance_repo
        .get_moderation_queue_stats(org_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get moderation stats: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get stats".to_string(),
            )
        })?;

    Ok(Json(ModerationQueueStatsResponse {
        pending_count: stats.pending_count,
        under_review_count: stats.under_review_count,
        by_priority: stats
            .by_priority
            .into_iter()
            .map(|p| PriorityCount {
                priority: p.priority,
                count: p.count,
            })
            .collect(),
        by_violation_type: stats
            .by_violation_type
            .into_iter()
            .map(|v| ViolationTypeCountResponse {
                violation_type: v.violation_type,
                count: v.count,
            })
            .collect(),
        avg_resolution_time_hours: stats.avg_resolution_time_hours,
        overdue_count: stats.overdue_count,
    }))
}

/// Get a specific moderation case.
pub(super) async fn get_moderation_case(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<ModerationCaseResponse>, (StatusCode, String)> {
    require_moderator_role(&user)?;

    let org_id = user.tenant_id.ok_or((
        StatusCode::BAD_REQUEST,
        "Organization context required".to_string(),
    ))?;

    let case = state
        .compliance_repo
        .get_moderation_case(id, org_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get moderation case: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get case".to_string(),
            )
        })?
        .ok_or((StatusCode::NOT_FOUND, format!("Case {} not found", id)))?;

    // Get violation count for content owner
    let violation_count = state
        .compliance_repo
        .get_user_violation_count(case.content_owner_id)
        .await
        .unwrap_or(0);

    Ok(Json(ModerationCaseResponse::from_case(
        case,
        Utc::now(),
        "User",
        violation_count,
        None,
    )))
}

/// Assign case request.
#[derive(Debug, Deserialize)]
pub struct AssignCaseRequest {
    pub moderator_id: Option<Uuid>, // None = assign to self
}

/// Assign a moderation case.
pub(super) async fn assign_moderation_case(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<AssignCaseRequest>,
) -> Result<Json<ModerationCaseResponse>, (StatusCode, String)> {
    require_moderator_role(&user)?;

    let org_id = user.tenant_id.ok_or((
        StatusCode::BAD_REQUEST,
        "Organization context required".to_string(),
    ))?;

    let assignee = req.moderator_id.unwrap_or(user.user_id);

    // Validate that the target moderator belongs to the caller's org
    if assignee != user.user_id {
        let is_member = state
            .org_member_repo
            .is_member(org_id, assignee)
            .await
            .unwrap_or(false);
        if !is_member {
            return Err((
                StatusCode::FORBIDDEN,
                "Moderator does not belong to this organization".to_string(),
            ));
        }
    }

    let case = state
        .compliance_repo
        .assign_moderation_case(id, org_id, assignee)
        .await
        .map_err(|e| {
            tracing::error!("Failed to assign moderation case: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to assign case".to_string(),
            )
        })?;

    Ok(Json(ModerationCaseResponse::from_case(
        case,
        Utc::now(),
        "User",
        0,
        Some("Assigned".to_string()),
    )))
}

/// Take moderation action request.
#[derive(Debug, Deserialize)]
pub struct TakeModerationActionRequest {
    pub action: ModerationActionType,
    pub rationale: String,
    pub template_id: Option<Uuid>,
}

/// Take action on a moderation case.
pub(super) async fn take_moderation_action(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<TakeModerationActionRequest>,
) -> Result<Json<ModerationCaseResponse>, (StatusCode, String)> {
    require_moderator_role(&user)?;

    let org_id = user.tenant_id.ok_or((
        StatusCode::BAD_REQUEST,
        "Organization context required".to_string(),
    ))?;

    validate_text_field(&req.rationale, MAX_RATIONALE_LEN, "rationale")
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    // Capture the decision payload before it is moved into the repo call so it
    // can be recorded as the DSA Art. 17 statement of reasons (action + rationale).
    let moderation_action = req.action;
    let rationale = req.rationale.clone();

    let action = TakeModerationAction {
        action: req.action,
        rationale: req.rationale,
        template_id: req.template_id,
    };

    let case = state
        .compliance_repo
        .take_moderation_action(id, org_id, action, user.user_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to take moderation action: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to take action".to_string(),
            )
        })?;

    write_compliance_audit(
        &state,
        &user,
        AuditAction::ResourceUpdated,
        "moderation_case",
        case.id,
        serde_json::json!({
            "operation": "take_moderation_action",
            "action": moderation_action,
            "rationale": rationale,
            "resulting_status": case.status,
        }),
    )
    .await;

    Ok(Json(ModerationCaseResponse::from_case(
        case,
        Utc::now(),
        "User",
        0,
        None,
    )))
}

/// File appeal request.
#[derive(Debug, Deserialize)]
pub struct FileAppealRequest {
    pub reason: String,
}

/// File an appeal against moderation decision.
pub(super) async fn file_appeal(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<FileAppealRequest>,
) -> Result<Json<ModerationCaseResponse>, (StatusCode, String)> {
    validate_text_field(&req.reason, MAX_APPEAL_REASON_LEN, "reason")
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    // A content owner may appeal a moderation decision on *their own* content.
    // Load the case scoped to the caller's organization first and enforce
    // ownership before mutating it; otherwise any authenticated user could
    // appeal any case in any tenant via a global id (PAP-60/PAP-43 — F5 IDOR).
    //
    // A caller with no tenant context, or a case outside the caller's org, is
    // treated as non-existent (404) so we never leak the case's existence
    // across tenants.
    let org_id = user.tenant_id.ok_or((
        StatusCode::NOT_FOUND,
        "Moderation case not found".to_string(),
    ))?;

    let existing = state
        .compliance_repo
        .get_moderation_case(id, org_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to load moderation case: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to file appeal".to_string(),
            )
        })?
        .ok_or((
            StatusCode::NOT_FOUND,
            "Moderation case not found".to_string(),
        ))?;

    // Ownership: only the content owner may file the appeal.
    if existing.content_owner_id != user.user_id {
        return Err((
            StatusCode::FORBIDDEN,
            "You can only appeal moderation decisions on your own content".to_string(),
        ));
    }

    let case = state
        .compliance_repo
        .file_appeal(id, org_id, &req.reason)
        .await
        .map_err(|e| {
            tracing::error!("Failed to file appeal: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to file appeal".to_string(),
            )
        })?;

    tracing::info!(
        case_id = %id,
        appealed_by = %user.user_id,
        "Appeal filed"
    );

    Ok(Json(ModerationCaseResponse::from_case(
        case,
        Utc::now(),
        "User",
        0,
        None,
    )))
}

/// Decide appeal request.
#[derive(Debug, Deserialize)]
pub struct DecideAppealRequest {
    pub decision: String, // upheld, rejected
    pub rationale: String,
}

/// Decide on an appeal.
pub(super) async fn decide_appeal(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<DecideAppealRequest>,
) -> Result<Json<ModerationCaseResponse>, (StatusCode, String)> {
    require_moderator_role(&user)?;

    let org_id = user.tenant_id.ok_or((
        StatusCode::BAD_REQUEST,
        "Organization context required".to_string(),
    ))?;

    // Reject unknown decisions (typo/casing) up front: the repo maps anything
    // other than "upheld" to a rejection, so validation here prevents a silent,
    // wrong appeal outcome. Also cap the rationale free-text length.
    validate_appeal_decision(&req.decision).map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    validate_text_field(&req.rationale, MAX_RATIONALE_LEN, "rationale")
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    let case = state
        .compliance_repo
        .decide_appeal(id, org_id, &req.decision, &req.rationale, user.user_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to decide appeal: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to decide appeal".to_string(),
            )
        })?;

    write_compliance_audit(
        &state,
        &user,
        AuditAction::ResourceUpdated,
        "moderation_case",
        case.id,
        serde_json::json!({
            "operation": "decide_appeal",
            "decision": req.decision,
            "rationale": req.rationale,
            "resulting_status": case.status,
        }),
    )
    .await;

    Ok(Json(ModerationCaseResponse::from_case(
        case,
        Utc::now(),
        "User",
        0,
        None,
    )))
}

/// Report content request.
#[derive(Debug, Deserialize)]
pub struct ReportContentRequest {
    pub content_type: ModeratedContentType,
    pub content_id: Uuid,
    pub violation_type: Option<ViolationType>,
    pub reason: Option<String>,
}

/// Report content for moderation.
pub(super) async fn report_content(
    State(state): State<AppState>,
    user: AuthUser,
    Json(req): Json<ReportContentRequest>,
) -> Result<Json<ModerationCaseResponse>, (StatusCode, String)> {
    // Any authenticated user can report content, but the case must be stored
    // against the *real* content owner — a random placeholder corrupts
    // violation-history and owner-notification downstream (PAP-60/PAP-43 — F7).
    // Resolve the owner (scoped to the caller's org) and reject the report if it
    // can't be resolved rather than persisting a bogus owner.
    let content_owner_id = state
        .compliance_repo
        .resolve_content_owner(req.content_type, req.content_id, user.tenant_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to resolve content owner: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to report content".to_string(),
            )
        })?
        .ok_or((
            StatusCode::NOT_FOUND,
            "Reported content could not be found".to_string(),
        ))?;

    let create_req = CreateModerationCase {
        content_type: req.content_type,
        content_id: req.content_id,
        violation_type: req.violation_type,
        report_reason: req.reason,
    };

    let case = state
        .compliance_repo
        .create_moderation_case(create_req, user.user_id, content_owner_id, user.tenant_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create moderation case: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to report content".to_string(),
            )
        })?;

    Ok(Json(ModerationCaseResponse::from_case(
        case,
        Utc::now(),
        "Unknown",
        0,
        None,
    )))
}

/// Action template response.
#[derive(Debug, Serialize)]
pub struct ActionTemplateResponse {
    pub id: Uuid,
    pub name: String,
    pub violation_type: ViolationType,
    pub action_type: ModerationActionType,
    pub rationale_template: String,
    pub notify_owner: bool,
}

/// Get available action templates.
pub(super) async fn get_action_templates(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<Vec<ActionTemplateResponse>>, (StatusCode, String)> {
    require_moderator_role(&user)?;

    let templates = state
        .compliance_repo
        .list_action_templates()
        .await
        .map_err(|e| {
            tracing::error!("Failed to list action templates: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get templates".to_string(),
            )
        })?;

    let responses: Vec<ActionTemplateResponse> = templates
        .into_iter()
        .map(|t| ActionTemplateResponse {
            id: t.id,
            name: t.name,
            violation_type: t.violation_type,
            action_type: t.action_type,
            rationale_template: t.rationale_template,
            notify_owner: t.notify_owner,
        })
        .collect();

    Ok(Json(responses))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use db::models::compliance::ReportSource;

    /// A fully-populated case fixture. Individual tests override just the
    /// fields they exercise so the assertions stay focused.
    fn sample_case() -> ModerationCase {
        let now = Utc::now();
        ModerationCase {
            id: Uuid::from_u128(1),
            content_type: ModeratedContentType::Listing,
            content_id: Uuid::from_u128(2),
            content_preview: Some("preview".to_string()),
            content_owner_id: Uuid::from_u128(3),
            organization_id: Some(Uuid::from_u128(4)),
            report_source: ReportSource::User,
            reported_by: Some(Uuid::from_u128(5)),
            automated_confidence: None,
            violation_type: Some(ViolationType::Spam),
            report_reason: Some("looks like spam".to_string()),
            status: ModerationStatus::Pending,
            priority: 3,
            assigned_to: None,
            assigned_at: None,
            decision: None,
            decision_rationale: None,
            action_template_id: None,
            decided_by: None,
            decided_at: None,
            appeal_filed: false,
            appeal_reason: None,
            appeal_filed_at: None,
            appeal_decision: None,
            appeal_decided_by: None,
            appeal_decided_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn from_case_computes_age_in_hours() {
        let now = Utc::now();
        let mut case = sample_case();
        case.created_at = now - Duration::hours(2);
        let resp = ModerationCaseResponse::from_case(case, now, "User", 0, None);
        // 120 minutes / 60 == 2.0 hours.
        assert!((resp.age_hours - 2.0).abs() < 1e-9);
    }

    #[test]
    fn from_case_age_uses_minute_granularity() {
        let now = Utc::now();
        let mut case = sample_case();
        case.created_at = now - Duration::minutes(90);
        let resp = ModerationCaseResponse::from_case(case, now, "User", 0, None);
        assert!((resp.age_hours - 1.5).abs() < 1e-9);
    }

    #[test]
    fn from_case_maps_owner_name_and_violations() {
        let now = Utc::now();
        let resp = ModerationCaseResponse::from_case(sample_case(), now, "User", 7, None);
        assert_eq!(resp.content_owner.name, "User");
        assert_eq!(resp.content_owner.previous_violations, 7);
        assert_eq!(resp.content_owner.user_id, Uuid::from_u128(3));
    }

    #[test]
    fn from_case_preserves_unknown_owner_label() {
        // report_content presents the owner as "Unknown"; the other handlers
        // present "User". Lock that distinction.
        let now = Utc::now();
        let resp = ModerationCaseResponse::from_case(sample_case(), now, "Unknown", 0, None);
        assert_eq!(resp.content_owner.name, "Unknown");
    }

    #[test]
    fn from_case_passes_assigned_to_name_through() {
        let now = Utc::now();
        let assigned = ModerationCaseResponse::from_case(
            sample_case(),
            now,
            "User",
            0,
            Some("Assigned".to_string()),
        );
        assert_eq!(assigned.assigned_to_name.as_deref(), Some("Assigned"));

        let unassigned = ModerationCaseResponse::from_case(sample_case(), now, "User", 0, None);
        assert_eq!(unassigned.assigned_to_name, None);
    }

    #[test]
    fn from_case_passes_through_core_fields() {
        let now = Utc::now();
        let resp = ModerationCaseResponse::from_case(sample_case(), now, "User", 0, None);
        assert_eq!(resp.id, Uuid::from_u128(1));
        assert_eq!(resp.content_id, Uuid::from_u128(2));
        assert_eq!(resp.content_preview.as_deref(), Some("preview"));
        assert_eq!(resp.report_source, "user"); // ReportSource::User stringified
        assert_eq!(resp.priority, 3);
        assert_eq!(resp.report_reason.as_deref(), Some("looks like spam"));
        assert!(!resp.appeal_filed);
    }
}
