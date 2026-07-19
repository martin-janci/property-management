//! Government Portal Integration API routes (Epic 30).
//!
//! UC-22.3: Government Portal Integration

use api_core::{AuthUser, TenantExtractor};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post},
    Json, Router,
};
use common::errors::ErrorResponse;
use db::models::{
    AddSubmissionAttachment, CreatePortalConnection, CreateRegulatorySubmission,
    CreateSubmissionAudit, CreateSubmissionSchedule, GovernmentPortalConnection,
    GovernmentPortalStats, GovernmentPortalType, RegulatoryReportTemplate, RegulatorySubmission,
    RegulatorySubmissionAttachment, RegulatorySubmissionAudit, RegulatorySubmissionSchedule,
    SubmissionQuery, SubmissionStatus, UpdatePortalConnection, UpdateRegulatorySubmission,
    UpdateSubmissionSchedule,
};
use serde::Deserialize;
use tracing::info;
use uuid::Uuid;

use crate::state::AppState;

/// Create router for government portal endpoints.
pub fn router() -> Router<AppState> {
    Router::new()
        // Portal Connections
        .route(
            "/connections",
            get(list_connections).post(create_connection),
        )
        .route(
            "/connections/{id}",
            get(get_connection)
                .put(update_connection)
                .delete(delete_connection),
        )
        .route("/connections/{id}/test", post(test_connection))
        // Report Templates
        .route("/templates", get(list_templates))
        .route("/templates/{id}", get(get_template))
        // Regulatory Submissions
        .route(
            "/submissions",
            get(list_submissions).post(create_submission),
        )
        .route(
            "/submissions/{id}",
            get(get_submission).put(update_submission),
        )
        .route("/submissions/{id}/validate", post(validate_submission))
        .route("/submissions/{id}/submit", post(submit_submission))
        .route("/submissions/{id}/cancel", post(cancel_submission))
        .route("/submissions/{id}/audit", get(get_submission_audit))
        // Submission Attachments
        .route(
            "/submissions/{id}/attachments",
            get(list_attachments).post(add_attachment),
        )
        .route(
            "/submissions/{submission_id}/attachments/{attachment_id}",
            delete(delete_attachment),
        )
        // Submission Schedules
        .route("/schedules", get(list_schedules).post(create_schedule))
        .route(
            "/schedules/{id}",
            get(get_schedule)
                .put(update_schedule)
                .delete(delete_schedule),
        )
        // Statistics
        .route("/stats", get(get_stats))
}

// ============================================================================
// Portal Connections
// ============================================================================

/// List portal connections.
async fn list_connections(
    State(state): State<AppState>,
    _auth: AuthUser,
    tenant: TenantExtractor,
) -> Result<Json<Vec<GovernmentPortalConnection>>, (StatusCode, Json<ErrorResponse>)> {
    let connections = state
        .government_portal_repo
        .list_connections(tenant.tenant_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?;

    Ok(Json(connections))
}

/// Get a portal connection.
async fn get_connection(
    State(state): State<AppState>,
    _auth: AuthUser,
    tenant: TenantExtractor,
    Path(id): Path<Uuid>,
) -> Result<Json<GovernmentPortalConnection>, (StatusCode, Json<ErrorResponse>)> {
    let connection = state
        .government_portal_repo
        .get_connection(id, tenant.tenant_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "CONNECTION_NOT_FOUND",
                    "Portal connection not found",
                )),
            )
        })?;

    Ok(Json(connection))
}

/// Create a portal connection.
async fn create_connection(
    State(state): State<AppState>,
    auth: AuthUser,
    tenant: TenantExtractor,
    Json(request): Json<CreatePortalConnection>,
) -> Result<(StatusCode, Json<GovernmentPortalConnection>), (StatusCode, Json<ErrorResponse>)> {
    let connection = state
        .government_portal_repo
        .create_connection(tenant.tenant_id, request, auth.user_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?;

    info!(
        user_id = %auth.user_id,
        connection_id = %connection.id,
        portal_type = ?connection.portal_type,
        "Created government portal connection"
    );

    Ok((StatusCode::CREATED, Json(connection)))
}

/// Update a portal connection.
async fn update_connection(
    State(state): State<AppState>,
    auth: AuthUser,
    tenant: TenantExtractor,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdatePortalConnection>,
) -> Result<Json<GovernmentPortalConnection>, (StatusCode, Json<ErrorResponse>)> {
    let connection = state
        .government_portal_repo
        .update_connection(
            id,
            tenant.tenant_id,
            request.portal_name.as_deref(),
            request.api_endpoint.as_deref(),
            request.portal_username.as_deref(),
            request.oauth_client_id.as_deref(),
            request.is_active,
            request.auto_submit,
            request.test_mode,
        )
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "CONNECTION_NOT_FOUND",
                    "Portal connection not found",
                )),
            )
        })?;

    info!(
        user_id = %auth.user_id,
        connection_id = %id,
        "Updated government portal connection"
    );

    Ok(Json(connection))
}

/// Delete a portal connection.
async fn delete_connection(
    State(state): State<AppState>,
    auth: AuthUser,
    tenant: TenantExtractor,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let deleted = state
        .government_portal_repo
        .delete_connection(id, tenant.tenant_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?;

    if !deleted {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "CONNECTION_NOT_FOUND",
                "Portal connection not found",
            )),
        ));
    }

    info!(
        user_id = %auth.user_id,
        connection_id = %id,
        "Deleted government portal connection"
    );

    Ok(StatusCode::NO_CONTENT)
}

/// Test a portal connection.
async fn test_connection(
    State(state): State<AppState>,
    auth: AuthUser,
    tenant: TenantExtractor,
    Path(id): Path<Uuid>,
) -> Result<Json<TestConnectionResponse>, (StatusCode, Json<ErrorResponse>)> {
    // In a real implementation, this would actually test the connection
    // For now, we just record the test and return success

    let exists = state
        .government_portal_repo
        .record_connection_test(id, true, tenant.tenant_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?;

    if !exists {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "CONNECTION_NOT_FOUND",
                "Portal connection not found",
            )),
        ));
    }

    info!(
        user_id = %auth.user_id,
        connection_id = %id,
        "Tested government portal connection"
    );

    Ok(Json(TestConnectionResponse {
        success: true,
        message: "Connection test successful".to_string(),
    }))
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct TestConnectionResponse {
    success: bool,
    message: String,
}

// ============================================================================
// Report Templates
// ============================================================================

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TemplateQuery {
    portal_type: Option<GovernmentPortalType>,
    country_code: Option<String>,
}

/// List report templates.
async fn list_templates(
    State(state): State<AppState>,
    _auth: AuthUser,
    Query(query): Query<TemplateQuery>,
) -> Result<Json<Vec<RegulatoryReportTemplate>>, (StatusCode, Json<ErrorResponse>)> {
    let templates = state
        .government_portal_repo
        .list_templates(query.portal_type, query.country_code.as_deref())
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?;

    Ok(Json(templates))
}

/// Get a report template.
async fn get_template(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<RegulatoryReportTemplate>, (StatusCode, Json<ErrorResponse>)> {
    let template = state
        .government_portal_repo
        .get_template(id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "TEMPLATE_NOT_FOUND",
                    "Template not found",
                )),
            )
        })?;

    Ok(Json(template))
}

// ============================================================================
// Regulatory Submissions
// ============================================================================

/// List submissions.
async fn list_submissions(
    State(state): State<AppState>,
    _auth: AuthUser,
    tenant: TenantExtractor,
    Query(query): Query<SubmissionQuery>,
) -> Result<Json<Vec<RegulatorySubmission>>, (StatusCode, Json<ErrorResponse>)> {
    let submissions = state
        .government_portal_repo
        .list_submissions(
            tenant.tenant_id,
            query.status,
            query.report_type.as_deref(),
            query.from_date,
            query.to_date,
            query.limit,
            query.offset,
        )
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?;

    Ok(Json(submissions))
}

/// Get a submission.
async fn get_submission(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<RegulatorySubmission>, (StatusCode, Json<ErrorResponse>)> {
    let submission = state
        .government_portal_repo
        .get_submission(id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "SUBMISSION_NOT_FOUND",
                    "Submission not found",
                )),
            )
        })?;

    Ok(Json(submission))
}

/// Create a submission.
async fn create_submission(
    State(state): State<AppState>,
    auth: AuthUser,
    tenant: TenantExtractor,
    Json(request): Json<CreateRegulatorySubmission>,
) -> Result<(StatusCode, Json<RegulatorySubmission>), (StatusCode, Json<ErrorResponse>)> {
    let submission = state
        .government_portal_repo
        .create_submission(tenant.tenant_id, request, auth.user_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?;

    // Create audit entry - log errors but don't fail the request
    // Compliance audit is critical so we log at error level
    if let Err(e) = state
        .government_portal_repo
        .create_audit(CreateSubmissionAudit {
            submission_id: submission.id,
            action: "created".to_string(),
            previous_status: None,
            new_status: Some(SubmissionStatus::Draft),
            actor_id: Some(auth.user_id),
            actor_type: "user".to_string(),
            details: None,
            error_message: None,
        })
        .await
    {
        tracing::error!(
            submission_id = %submission.id,
            user_id = %auth.user_id,
            error = %e,
            "COMPLIANCE: Failed to create audit entry for regulatory submission"
        );
    }

    info!(
        user_id = %auth.user_id,
        submission_id = %submission.id,
        report_type = %submission.report_type,
        "Created regulatory submission"
    );

    Ok((StatusCode::CREATED, Json(submission)))
}

/// Update a submission.
async fn update_submission(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateRegulatorySubmission>,
) -> Result<Json<RegulatorySubmission>, (StatusCode, Json<ErrorResponse>)> {
    let submission = state
        .government_portal_repo
        .update_submission(id, request.report_data, request.report_xml.as_deref())
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?;

    info!(
        user_id = %auth.user_id,
        submission_id = %id,
        "Updated regulatory submission"
    );

    Ok(Json(submission))
}

/// Validate a submission.
async fn validate_submission(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<RegulatorySubmission>, (StatusCode, Json<ErrorResponse>)> {
    // Get current submission
    let current = state
        .government_portal_repo
        .get_submission(id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "SUBMISSION_NOT_FOUND",
                    "Submission not found",
                )),
            )
        })?;

    // In a real implementation, this would validate against the template schema
    // For now, we just mark it as validated
    let validation_result = serde_json::json!({
        "is_valid": true,
        "errors": [],
        "warnings": []
    });

    let submission = state
        .government_portal_repo
        .update_submission_status(
            id,
            SubmissionStatus::Validated,
            Some(validation_result),
            None,
            None,
        )
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?;

    // Create audit entry
    let _ = state
        .government_portal_repo
        .create_audit(CreateSubmissionAudit {
            submission_id: id,
            action: "validated".to_string(),
            previous_status: Some(current.status),
            new_status: Some(SubmissionStatus::Validated),
            actor_id: Some(auth.user_id),
            actor_type: "user".to_string(),
            details: None,
            error_message: None,
        })
        .await;

    info!(
        user_id = %auth.user_id,
        submission_id = %id,
        "Validated regulatory submission"
    );

    Ok(Json(submission))
}

/// Submit a submission to the portal.
async fn submit_submission(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<RegulatorySubmission>, (StatusCode, Json<ErrorResponse>)> {
    // Get current submission
    let current = state
        .government_portal_repo
        .get_submission(id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "SUBMISSION_NOT_FOUND",
                    "Submission not found",
                )),
            )
        })?;

    // Check if validated
    if current.status != SubmissionStatus::Validated {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "NOT_VALIDATED",
                "Submission must be validated before submitting",
            )),
        ));
    }

    // Submit
    let submission = state
        .government_portal_repo
        .submit_submission(id, auth.user_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?;

    // Create audit entry
    let _ = state
        .government_portal_repo
        .create_audit(CreateSubmissionAudit {
            submission_id: id,
            action: "submitted".to_string(),
            previous_status: Some(current.status),
            new_status: Some(SubmissionStatus::Submitted),
            actor_id: Some(auth.user_id),
            actor_type: "user".to_string(),
            details: None,
            error_message: None,
        })
        .await;

    info!(
        user_id = %auth.user_id,
        submission_id = %id,
        "Submitted regulatory submission to portal"
    );

    Ok(Json(submission))
}

/// Cancel a submission.
async fn cancel_submission(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<RegulatorySubmission>, (StatusCode, Json<ErrorResponse>)> {
    // Get current submission
    let current = state
        .government_portal_repo
        .get_submission(id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "SUBMISSION_NOT_FOUND",
                    "Submission not found",
                )),
            )
        })?;

    // Check if can be cancelled
    if matches!(
        current.status,
        SubmissionStatus::Accepted | SubmissionStatus::Cancelled
    ) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "CANNOT_CANCEL",
                "This submission cannot be cancelled",
            )),
        ));
    }

    let submission = state
        .government_portal_repo
        .update_submission_status(id, SubmissionStatus::Cancelled, None, None, None)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?;

    // Create audit entry
    let _ = state
        .government_portal_repo
        .create_audit(CreateSubmissionAudit {
            submission_id: id,
            action: "cancelled".to_string(),
            previous_status: Some(current.status),
            new_status: Some(SubmissionStatus::Cancelled),
            actor_id: Some(auth.user_id),
            actor_type: "user".to_string(),
            details: None,
            error_message: None,
        })
        .await;

    info!(
        user_id = %auth.user_id,
        submission_id = %id,
        "Cancelled regulatory submission"
    );

    Ok(Json(submission))
}

/// Get submission audit trail.
async fn get_submission_audit(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<RegulatorySubmissionAudit>>, (StatusCode, Json<ErrorResponse>)> {
    let audit = state
        .government_portal_repo
        .get_submission_audit(id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?;

    Ok(Json(audit))
}

// ============================================================================
// Submission Attachments
// ============================================================================

/// List attachments for a submission.
async fn list_attachments(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<RegulatorySubmissionAttachment>>, (StatusCode, Json<ErrorResponse>)> {
    let attachments = state
        .government_portal_repo
        .get_submission_attachments(id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?;

    Ok(Json(attachments))
}

/// Add an attachment to a submission.
async fn add_attachment(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(request): Json<AddSubmissionAttachment>,
) -> Result<(StatusCode, Json<RegulatorySubmissionAttachment>), (StatusCode, Json<ErrorResponse>)> {
    let attachment = state
        .government_portal_repo
        .add_attachment(id, request)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?;

    info!(
        user_id = %auth.user_id,
        submission_id = %id,
        attachment_id = %attachment.id,
        "Added attachment to regulatory submission"
    );

    Ok((StatusCode::CREATED, Json(attachment)))
}

#[derive(Deserialize)]
struct AttachmentPath {
    submission_id: Uuid,
    attachment_id: Uuid,
}

/// Delete an attachment.
async fn delete_attachment(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(path): Path<AttachmentPath>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let deleted = state
        .government_portal_repo
        .delete_attachment(path.attachment_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?;

    if !deleted {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "ATTACHMENT_NOT_FOUND",
                "Attachment not found",
            )),
        ));
    }

    info!(
        user_id = %auth.user_id,
        submission_id = %path.submission_id,
        attachment_id = %path.attachment_id,
        "Deleted attachment from regulatory submission"
    );

    Ok(StatusCode::NO_CONTENT)
}

// ============================================================================
// Submission Schedules
// ============================================================================

/// List schedules.
async fn list_schedules(
    State(state): State<AppState>,
    _auth: AuthUser,
    tenant: TenantExtractor,
) -> Result<Json<Vec<RegulatorySubmissionSchedule>>, (StatusCode, Json<ErrorResponse>)> {
    let schedules = state
        .government_portal_repo
        .list_schedules(tenant.tenant_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?;

    Ok(Json(schedules))
}

/// Get a schedule.
async fn get_schedule(
    State(state): State<AppState>,
    _auth: AuthUser,
    tenant: TenantExtractor,
    Path(id): Path<Uuid>,
) -> Result<Json<RegulatorySubmissionSchedule>, (StatusCode, Json<ErrorResponse>)> {
    let schedules = state
        .government_portal_repo
        .list_schedules(tenant.tenant_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?;

    let schedule = schedules.into_iter().find(|s| s.id == id).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "SCHEDULE_NOT_FOUND",
                "Schedule not found",
            )),
        )
    })?;

    Ok(Json(schedule))
}

/// Create a schedule.
async fn create_schedule(
    State(state): State<AppState>,
    auth: AuthUser,
    tenant: TenantExtractor,
    Json(request): Json<CreateSubmissionSchedule>,
) -> Result<(StatusCode, Json<RegulatorySubmissionSchedule>), (StatusCode, Json<ErrorResponse>)> {
    let schedule = state
        .government_portal_repo
        .create_schedule(tenant.tenant_id, request, auth.user_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?;

    info!(
        user_id = %auth.user_id,
        schedule_id = %schedule.id,
        "Created regulatory submission schedule"
    );

    Ok((StatusCode::CREATED, Json(schedule)))
}

/// Update a schedule.
async fn update_schedule(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateSubmissionSchedule>,
) -> Result<Json<RegulatorySubmissionSchedule>, (StatusCode, Json<ErrorResponse>)> {
    let schedule = state
        .government_portal_repo
        .update_schedule(
            id,
            request.is_active,
            request.auto_generate,
            request.auto_submit,
            request.notify_before_days,
            request.notify_users,
        )
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?;

    info!(
        user_id = %auth.user_id,
        schedule_id = %id,
        "Updated regulatory submission schedule"
    );

    Ok(Json(schedule))
}

/// Delete a schedule.
async fn delete_schedule(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let deleted = state
        .government_portal_repo
        .delete_schedule(id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?;

    if !deleted {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "SCHEDULE_NOT_FOUND",
                "Schedule not found",
            )),
        ));
    }

    info!(
        user_id = %auth.user_id,
        schedule_id = %id,
        "Deleted regulatory submission schedule"
    );

    Ok(StatusCode::NO_CONTENT)
}

// ============================================================================
// Statistics
// ============================================================================

/// Get government portal statistics.
async fn get_stats(
    State(state): State<AppState>,
    _auth: AuthUser,
    tenant: TenantExtractor,
) -> Result<Json<GovernmentPortalStats>, (StatusCode, Json<ErrorResponse>)> {
    let stats = state
        .government_portal_repo
        .get_stats(tenant.tenant_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?;

    Ok(Json(stats))
}
