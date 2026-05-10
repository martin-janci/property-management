//! GDPR compliance routes (Epic 9, Stories 9.3-9.5).
//!
//! Handles data export, data deletion, and privacy settings.

use crate::state::AppState;
use api_core::extractors::{AuthUser, RlsConnection};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use db::models::{
    AuditAction, CreateAuditLog, CreateDataExportRequest, DataExportRequestResponse,
    DataExportStatus, DataExportStatusResponse, ExportCategories, ExportCategory, ExportFormat,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Create the GDPR router.
pub fn router() -> Router<AppState> {
    Router::new()
        // Data Export (Story 9.3)
        .route("/export/request", post(request_data_export))
        .route("/export/status/{request_id}", get(get_export_status))
        .route("/export/download/{token}", get(download_export))
        .route("/export/categories", get(get_export_categories))
        .route("/export/history", get(get_export_history))
        // Data Deletion (Story 9.4) - will be implemented next
        .route("/deletion/request", post(request_data_deletion))
        .route("/deletion/status", get(get_deletion_status))
        .route("/deletion/cancel", post(cancel_deletion_request))
        // Privacy Settings (Story 9.5)
        .route("/privacy", get(get_privacy_settings))
        .route("/privacy", post(update_privacy_settings))
}

// ============================================================================
// DATA EXPORT ENDPOINTS (Story 9.3)
// ============================================================================

/// Request for data export.
#[derive(Debug, Deserialize)]
pub struct RequestDataExport {
    /// Export format (json or csv)
    #[serde(default)]
    pub format: ExportFormat,
    /// Optional categories to include (null = all)
    pub categories: Option<Vec<String>>,
}

/// Request a data export.
async fn request_data_export(
    State(state): State<AppState>,
    user: AuthUser,
    Json(req): Json<RequestDataExport>,
) -> Result<Json<DataExportRequestResponse>, (StatusCode, String)> {
    // Check if user already has a pending/processing export
    let has_active = state
        .data_export_repo
        .has_active_request(user.user_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if has_active {
        return Err((
            StatusCode::CONFLICT,
            "You already have an active export request. Please wait for it to complete."
                .to_string(),
        ));
    }

    // Create the export request
    let export_request = state
        .data_export_repo
        .create(CreateDataExportRequest {
            user_id: user.user_id,
            format: req.format,
            include_categories: req.categories,
        })
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Log the data export request
    if let Err(e) = state
        .audit_log_repo
        .create(CreateAuditLog {
            user_id: Some(user.user_id),
            action: AuditAction::DataExportRequested,
            resource_type: Some("data_export".to_string()),
            resource_id: Some(export_request.id),
            org_id: None,
            details: None,
            old_values: None,
            new_values: None,
            ip_address: None,
            user_agent: None,
        })
        .await
    {
        tracing::error!(error = %e, user_id = %user.user_id, export_id = %export_request.id, "Failed to create audit log for data export request");
    }

    Ok(Json(DataExportRequestResponse {
        request_id: export_request.id,
        estimated_time: "5-10 minutes".to_string(),
        status: export_request.status,
    }))
}

/// Get export request status.
async fn get_export_status(
    State(state): State<AppState>,
    user: AuthUser,
    Path(request_id): Path<Uuid>,
) -> Result<Json<DataExportStatusResponse>, (StatusCode, String)> {
    let export_request = state
        .data_export_repo
        .get_by_id(request_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((
            StatusCode::NOT_FOUND,
            "Export request not found".to_string(),
        ))?;

    // Ensure user owns this export request
    if export_request.user_id != user.user_id {
        return Err((StatusCode::FORBIDDEN, "Access denied".to_string()));
    }

    let download_url = if export_request.status == DataExportStatus::Ready {
        export_request
            .download_token
            .map(|token| format!("/api/v1/gdpr/export/download/{}", token))
    } else {
        None
    };

    Ok(Json(DataExportStatusResponse {
        request_id: export_request.id,
        status: export_request.status,
        estimated_ready_at: None, // Could calculate based on queue position
        download_url,
        file_size_bytes: export_request.file_size_bytes,
        expires_at: export_request.expires_at,
        error_message: export_request.error_message,
    }))
}

/// Download an export file.
async fn download_export(
    State(state): State<AppState>,
    user: AuthUser,
    Path(token): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let export_request = state
        .data_export_repo
        .get_by_token(token)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((
            StatusCode::NOT_FOUND,
            "Export not found or expired".to_string(),
        ))?;

    // SECURITY: Validate file ownership - user must own this export
    if export_request.user_id != user.user_id {
        return Err((StatusCode::FORBIDDEN, "Access denied".to_string()));
    }

    // Check if expired
    if export_request.expires_at < chrono::Utc::now() {
        return Err((StatusCode::GONE, "Export link has expired".to_string()));
    }

    // Log the download
    if let Err(e) = state
        .audit_log_repo
        .create(CreateAuditLog {
            user_id: Some(export_request.user_id),
            action: AuditAction::DataExportDownloaded,
            resource_type: Some("data_export".to_string()),
            resource_id: Some(export_request.id),
            org_id: None,
            details: None,
            old_values: None,
            new_values: None,
            ip_address: None,
            user_agent: None,
        })
        .await
    {
        tracing::error!(error = %e, user_id = %user.user_id, export_id = %export_request.id, "Failed to create audit log for data export download");
    }

    // Mark as downloaded
    let _ = state
        .data_export_repo
        .mark_downloaded(export_request.id)
        .await;

    // In a real implementation, this would stream the file from S3
    // For now, we collect the data and return it directly
    let categories: Option<Vec<String>> = export_request
        .include_categories
        .and_then(|v| serde_json::from_value(v).ok());

    let user_data = state
        .data_export_repo
        .collect_user_data(export_request.user_id, categories)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(serde_json::to_value(user_data).unwrap_or_default()))
}

/// Get available export categories.
async fn get_export_categories() -> Json<ExportCategories> {
    Json(ExportCategories {
        categories: vec![
            ExportCategory {
                id: "profile".to_string(),
                name: "Profile Information".to_string(),
                description: "Your account details, email, name, and settings".to_string(),
                default_included: true,
            },
            ExportCategory {
                id: "organizations".to_string(),
                name: "Organization Memberships".to_string(),
                description: "Organizations you belong to and your roles".to_string(),
                default_included: true,
            },
            ExportCategory {
                id: "residencies".to_string(),
                name: "Residency History".to_string(),
                description: "Units and buildings you've lived in".to_string(),
                default_included: true,
            },
            ExportCategory {
                id: "activity".to_string(),
                name: "Activity Log".to_string(),
                description: "Your actions and login history".to_string(),
                default_included: true,
            },
            ExportCategory {
                id: "documents".to_string(),
                name: "Documents".to_string(),
                description: "Documents you've uploaded".to_string(),
                default_included: true,
            },
            ExportCategory {
                id: "votes".to_string(),
                name: "Voting History".to_string(),
                description: "Your votes and ballot submissions".to_string(),
                default_included: true,
            },
            ExportCategory {
                id: "faults".to_string(),
                name: "Fault Reports".to_string(),
                description: "Issues you've reported".to_string(),
                default_included: true,
            },
            ExportCategory {
                id: "messages".to_string(),
                name: "Messages".to_string(),
                description: "Messages you've sent".to_string(),
                default_included: true,
            },
            ExportCategory {
                id: "announcements".to_string(),
                name: "Announcements".to_string(),
                description: "Announcements you've created".to_string(),
                default_included: true,
            },
        ],
    })
}

/// Export history entry.
#[derive(Debug, Serialize)]
pub struct ExportHistoryEntry {
    pub id: Uuid,
    pub status: DataExportStatus,
    pub format: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub downloaded: bool,
}

/// Get user's export history.
async fn get_export_history(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<Vec<ExportHistoryEntry>>, (StatusCode, String)> {
    let exports = state
        .data_export_repo
        .get_by_user_id(user.user_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let history: Vec<ExportHistoryEntry> = exports
        .into_iter()
        .map(|e| ExportHistoryEntry {
            id: e.id,
            status: e.status,
            format: e.format,
            created_at: e.created_at,
            completed_at: e.completed_at,
            expires_at: e.expires_at,
            downloaded: e.downloaded_at.is_some(),
        })
        .collect();

    Ok(Json(history))
}

// ============================================================================
// DATA DELETION ENDPOINTS (Story 9.4)
// ============================================================================

/// Request for data deletion.
#[derive(Debug, Deserialize)]
pub struct RequestDataDeletion {
    /// Confirmation text (must match email)
    pub confirmation: String,
    /// Reason for deletion (optional)
    pub reason: Option<String>,
}

/// Data deletion status response.
#[derive(Debug, Serialize)]
pub struct DeletionStatusResponse {
    /// Whether a deletion request is active
    pub has_active_request: bool,
    /// When deletion was requested
    pub requested_at: Option<chrono::DateTime<chrono::Utc>>,
    /// When account will be deleted (30-day grace period)
    pub scheduled_for: Option<chrono::DateTime<chrono::Utc>>,
    /// Days remaining to cancel
    pub days_remaining: Option<i64>,
    /// Whether deletion can still be cancelled
    pub can_cancel: bool,
}

/// Request account deletion (GDPR Article 17).
async fn request_data_deletion(
    State(state): State<AppState>,
    user: AuthUser,
    Json(req): Json<RequestDataDeletion>,
) -> Result<Json<DeletionStatusResponse>, (StatusCode, String)> {
    // Get user to verify confirmation
    let db_user = state
        .user_repo
        .find_by_id(user.user_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "User not found".to_string()))?;

    // Confirmation must match email
    if req.confirmation != db_user.email {
        return Err((
            StatusCode::BAD_REQUEST,
            "Confirmation text must match your email address".to_string(),
        ));
    }

    // Check if already scheduled for deletion
    if db_user.scheduled_deletion_at.is_some() {
        return Err((
            StatusCode::CONFLICT,
            "Account is already scheduled for deletion".to_string(),
        ));
    }

    // Schedule deletion for 30 days from now (GDPR grace period)
    let scheduled_for = chrono::Utc::now() + chrono::Duration::days(30);

    // Update user record via repository
    state
        .user_repo
        .schedule_deletion(user.user_id, scheduled_for)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Log the deletion request
    if let Err(e) = state
        .audit_log_repo
        .create(CreateAuditLog {
            user_id: Some(user.user_id),
            action: AuditAction::DataDeletionRequested,
            resource_type: Some("user".to_string()),
            resource_id: Some(user.user_id),
            org_id: None,
            details: req.reason.map(|r| serde_json::json!({ "reason": r })),
            old_values: None,
            new_values: Some(serde_json::json!({ "scheduled_deletion_at": scheduled_for })),
            ip_address: None,
            user_agent: None,
        })
        .await
    {
        tracing::error!(error = %e, user_id = %user.user_id, "Failed to create audit log for data deletion request");
    }

    Ok(Json(DeletionStatusResponse {
        has_active_request: true,
        requested_at: Some(chrono::Utc::now()),
        scheduled_for: Some(scheduled_for),
        days_remaining: Some(30),
        can_cancel: true,
    }))
}

/// Get deletion request status.
async fn get_deletion_status(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<DeletionStatusResponse>, (StatusCode, String)> {
    let db_user = state
        .user_repo
        .find_by_id(user.user_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "User not found".to_string()))?;

    match db_user.scheduled_deletion_at {
        Some(scheduled_for) => {
            let now = chrono::Utc::now();
            let days_remaining = (scheduled_for - now).num_days().max(0);

            Ok(Json(DeletionStatusResponse {
                has_active_request: true,
                requested_at: None, // Would need to track this separately
                scheduled_for: Some(scheduled_for),
                days_remaining: Some(days_remaining),
                can_cancel: days_remaining > 0,
            }))
        }
        None => Ok(Json(DeletionStatusResponse {
            has_active_request: false,
            requested_at: None,
            scheduled_for: None,
            days_remaining: None,
            can_cancel: false,
        })),
    }
}

/// Cancel a pending deletion request.
async fn cancel_deletion_request(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<DeletionStatusResponse>, (StatusCode, String)> {
    let db_user = state
        .user_repo
        .find_by_id(user.user_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "User not found".to_string()))?;

    if db_user.scheduled_deletion_at.is_none() {
        return Err((
            StatusCode::NOT_FOUND,
            "No active deletion request found".to_string(),
        ));
    }

    // Clear the scheduled deletion via repository
    state
        .user_repo
        .cancel_scheduled_deletion(user.user_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Log the cancellation
    if let Err(e) = state
        .audit_log_repo
        .create(CreateAuditLog {
            user_id: Some(user.user_id),
            action: AuditAction::DataDeletionCancelled,
            resource_type: Some("user".to_string()),
            resource_id: Some(user.user_id),
            org_id: None,
            details: None,
            old_values: Some(
                serde_json::json!({ "scheduled_deletion_at": db_user.scheduled_deletion_at }),
            ),
            new_values: None,
            ip_address: None,
            user_agent: None,
        })
        .await
    {
        tracing::error!(error = %e, user_id = %user.user_id, "Failed to create audit log for data deletion cancellation");
    }

    Ok(Json(DeletionStatusResponse {
        has_active_request: false,
        requested_at: None,
        scheduled_for: None,
        days_remaining: None,
        can_cancel: false,
    }))
}

// ============================================================================
// PRIVACY SETTINGS ENDPOINTS (Story 9.5)
// ============================================================================

/// Privacy settings response.
#[derive(Debug, Serialize, Deserialize)]
pub struct PrivacySettingsResponse {
    /// Profile visibility setting
    pub profile_visibility: String,
    /// Whether to show contact info to neighbors
    pub show_contact_info: bool,
    /// Whether email is verified
    pub email_verified: bool,
    /// Available visibility options
    pub visibility_options: Vec<VisibilityOption>,
}

/// A visibility option with description.
#[derive(Debug, Serialize, Deserialize)]
pub struct VisibilityOption {
    pub value: String,
    pub label: String,
    pub description: String,
}

/// Get current privacy settings.
async fn get_privacy_settings(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<PrivacySettingsResponse>, (StatusCode, String)> {
    let db_user = state
        .user_repo
        .find_by_id(user.user_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "User not found".to_string()))?;

    let email_verified = db_user.email_verified();
    Ok(Json(PrivacySettingsResponse {
        profile_visibility: db_user.profile_visibility,
        show_contact_info: db_user.show_contact_info,
        email_verified,
        visibility_options: vec![
            VisibilityOption {
                value: "visible".to_string(),
                label: "Visible".to_string(),
                description: "Your full name and unit are shown to neighbors".to_string(),
            },
            VisibilityOption {
                value: "hidden".to_string(),
                label: "Hidden".to_string(),
                description: "You appear as 'Resident of Unit X'".to_string(),
            },
            VisibilityOption {
                value: "contacts_only".to_string(),
                label: "Contacts Only".to_string(),
                description: "Name shown but no contact info unless connected".to_string(),
            },
        ],
    }))
}

/// Update privacy settings request.
#[derive(Debug, Deserialize)]
pub struct UpdatePrivacySettingsRequest {
    /// New profile visibility setting
    pub profile_visibility: Option<String>,
    /// Whether to show contact info
    pub show_contact_info: Option<bool>,
}

/// Update privacy settings.
async fn update_privacy_settings(
    State(state): State<AppState>,
    user: AuthUser,
    mut rls: RlsConnection,
    Json(req): Json<UpdatePrivacySettingsRequest>,
) -> Result<Json<PrivacySettingsResponse>, (StatusCode, String)> {
    // Validate visibility if provided
    if let Some(ref visibility) = req.profile_visibility {
        if !["visible", "hidden", "contacts_only"].contains(&visibility.as_str()) {
            return Err((
                StatusCode::BAD_REQUEST,
                "Invalid visibility option".to_string(),
            ));
        }
    }

    // Get current settings for audit log
    let old_user = state
        .user_repo
        .find_by_id(user.user_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "User not found".to_string()))?;

    let old_values = serde_json::json!({
        "profile_visibility": old_user.profile_visibility,
        "show_contact_info": old_user.show_contact_info
    });

    // Update settings
    let new_visibility = req
        .profile_visibility
        .as_ref()
        .unwrap_or(&old_user.profile_visibility);
    let new_show_contact = req.show_contact_info.unwrap_or(old_user.show_contact_info);

    sqlx::query(
        r#"
        UPDATE users
        SET profile_visibility = $1, show_contact_info = $2, updated_at = NOW()
        WHERE id = $3
        "#,
    )
    .bind(new_visibility)
    .bind(new_show_contact)
    .bind(user.user_id)
    .execute(&mut **rls.conn())
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let new_values = serde_json::json!({
        "profile_visibility": new_visibility,
        "show_contact_info": new_show_contact
    });

    // Log the privacy settings update
    if let Err(e) = state
        .audit_log_repo
        .create(CreateAuditLog {
            user_id: Some(user.user_id),
            action: AuditAction::PrivacySettingsUpdated,
            resource_type: Some("user".to_string()),
            resource_id: Some(user.user_id),
            org_id: None,
            details: None,
            old_values: Some(old_values),
            new_values: Some(new_values.clone()),
            ip_address: None,
            user_agent: None,
        })
        .await
    {
        tracing::error!(error = %e, user_id = %user.user_id, "Failed to create audit log for privacy settings update");
    }

    // Return updated settings
    get_privacy_settings(State(state), user).await
}
