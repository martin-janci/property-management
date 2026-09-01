//! Compliance reporting routes (Epic 9, Story 9.7).
//!
//! Handles compliance report generation for GDPR, audit trails, and security.

#![allow(clippy::type_complexity)]

use crate::state::AppState;
use api_core::extractors::{AuthUser, RlsConnection};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Utc};
use db::models::{AuditAction, AuditLogQuery};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Require platform-tier (SuperAdmin **or** PlatformAdmin) privileges.
///
/// These compliance reports (audit trails, GDPR, security) are platform-wide,
/// so the whole platform tier must reach them. Previously this gated on an
/// exact `SuperAdmin` match, which locked out `PlatformAdmin` — the
/// infrastructure/operations role that sits directly below `SuperAdmin` in the
/// role hierarchy (`TenantRole::level()`: 100 vs 95) and is classified as
/// platform-tier by [`AuthUser::is_platform_admin`]. Reuse that canonical
/// helper rather than hard-coding the variant list so the two cannot drift
/// (mirrors `aml_dsa::shared::require_compliance_role`).
fn require_platform_admin(user: &AuthUser) -> Result<(), (StatusCode, String)> {
    if user.is_platform_admin() {
        Ok(())
    } else {
        Err((
            StatusCode::FORBIDDEN,
            "This endpoint requires platform administrator privileges".to_string(),
        ))
    }
}

/// Require the exact top-tier `SuperAdmin` role.
///
/// Stricter than [`require_platform_admin`]: the audit-log endpoints expose the
/// platform-wide tamper-evident audit trail (who did what, integrity proofs),
/// which must be readable only by `SuperAdmin` — not the operations-tier
/// `PlatformAdmin` that `is_platform_admin` also admits. Match the exact variant
/// (`TenantRole::SuperAdmin`, `level() == 100`) rather than a level threshold so
/// no lower role can ever satisfy this gate.
fn require_super_admin(user: &AuthUser) -> Result<(), (StatusCode, String)> {
    if matches!(user.role, Some(common::TenantRole::SuperAdmin)) {
        Ok(())
    } else {
        Err((
            StatusCode::FORBIDDEN,
            "This endpoint requires super administrator privileges".to_string(),
        ))
    }
}

/// Create the compliance router.
pub fn router() -> Router<AppState> {
    Router::new()
        // Audit Log Reports
        .route("/audit-logs", get(get_audit_logs))
        .route("/audit-logs/summary", get(get_audit_summary))
        .route("/audit-logs/user/{user_id}", get(get_user_audit_logs))
        .route("/audit-logs/integrity", get(verify_audit_integrity))
        // GDPR Reports
        .route("/gdpr/data-exports", get(get_data_export_report))
        .route("/gdpr/deletion-requests", get(get_deletion_requests_report))
        .route("/gdpr/privacy-report", get(get_privacy_settings_report))
        // Security Reports
        .route("/security/login-activity", get(get_login_activity_report))
        .route("/security/mfa-status", get(get_mfa_status_report))
        .route("/security/failed-logins", get(get_failed_logins_report))
}

// ============================================================================
// QUERY PARAMETERS
// ============================================================================

/// Query parameters for audit log listing.
#[derive(Debug, Deserialize)]
pub struct AuditLogQueryParams {
    /// Filter by user ID
    pub user_id: Option<Uuid>,
    /// Filter by action type
    pub action: Option<String>,
    /// Filter by resource type
    pub resource_type: Option<String>,
    /// Filter by organization
    pub org_id: Option<Uuid>,
    /// Start date filter
    pub from_date: Option<DateTime<Utc>>,
    /// End date filter
    pub to_date: Option<DateTime<Utc>>,
    /// Page limit (max 1000)
    pub limit: Option<i64>,
    /// Page offset
    pub offset: Option<i64>,
}

/// Query parameters for reports.
#[derive(Debug, Deserialize)]
pub struct ReportQueryParams {
    /// Start date filter
    pub from_date: Option<DateTime<Utc>>,
    /// End date filter
    pub to_date: Option<DateTime<Utc>>,
    /// Organization filter
    pub org_id: Option<Uuid>,
}

// ============================================================================
// AUDIT LOG ENDPOINTS
// ============================================================================

/// Audit log entry response.
#[derive(Debug, Serialize)]
pub struct AuditLogResponse {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub action: String,
    pub resource_type: Option<String>,
    pub resource_id: Option<Uuid>,
    pub org_id: Option<Uuid>,
    pub details: Option<serde_json::Value>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Paginated audit log list response.
#[derive(Debug, Serialize)]
pub struct AuditLogListResponse {
    pub logs: Vec<AuditLogResponse>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

/// Get paginated audit logs.
async fn get_audit_logs(
    State(state): State<AppState>,
    user: AuthUser,
    Query(params): Query<AuditLogQueryParams>,
) -> Result<Json<AuditLogListResponse>, (StatusCode, String)> {
    require_super_admin(&user)?;
    let action = params.action.and_then(|a| parse_audit_action(&a));

    let query = AuditLogQuery {
        user_id: params.user_id,
        action,
        resource_type: params.resource_type,
        resource_id: None,
        org_id: params.org_id,
        from_date: params.from_date,
        to_date: params.to_date,
        limit: params.limit,
        offset: params.offset,
    };

    let logs = state
        .audit_log_repo
        .query(query.clone())
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let total = state
        .audit_log_repo
        .count(query)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let log_responses: Vec<AuditLogResponse> = logs
        .into_iter()
        .map(|log| AuditLogResponse {
            id: log.id,
            user_id: log.user_id,
            // P1-04: canonical_name() gives stable output, not Rust Debug repr.
            action: log.action.canonical_name().to_string(),
            resource_type: log.resource_type,
            resource_id: log.resource_id,
            org_id: log.org_id,
            details: log.details,
            ip_address: log.ip_address,
            user_agent: log.user_agent,
            created_at: log.created_at,
        })
        .collect();

    Ok(Json(AuditLogListResponse {
        logs: log_responses,
        total,
        limit: params.limit.unwrap_or(100),
        offset: params.offset.unwrap_or(0),
    }))
}

/// Audit summary response.
#[derive(Debug, Serialize)]
pub struct AuditSummaryResponse {
    pub total_entries: i64,
    pub action_counts: Vec<ActionCountResponse>,
    pub from_date: Option<DateTime<Utc>>,
    pub to_date: Option<DateTime<Utc>>,
    pub integrity_verified: bool,
}

/// Action count item.
#[derive(Debug, Serialize)]
pub struct ActionCountResponse {
    pub action: String,
    pub count: i64,
}

/// Get audit log summary.
async fn get_audit_summary(
    State(state): State<AppState>,
    user: AuthUser,
    Query(params): Query<ReportQueryParams>,
) -> Result<Json<AuditSummaryResponse>, (StatusCode, String)> {
    require_super_admin(&user)?;
    let action_counts = state
        .audit_log_repo
        .get_action_counts(params.from_date, params.to_date, params.org_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let total_entries: i64 = action_counts.iter().map(|c| c.count).sum();

    let count_responses: Vec<ActionCountResponse> = action_counts
        .into_iter()
        .map(|c| ActionCountResponse {
            action: c.action,
            count: c.count,
        })
        .collect();

    // Verify integrity (sample check)
    let integrity_ok = state
        .audit_log_repo
        .verify_integrity(Some(1000))
        .await
        .unwrap_or(false);

    Ok(Json(AuditSummaryResponse {
        total_entries,
        action_counts: count_responses,
        from_date: params.from_date,
        to_date: params.to_date,
        integrity_verified: integrity_ok,
    }))
}

/// Get audit logs for a specific user.
async fn get_user_audit_logs(
    State(state): State<AppState>,
    user: AuthUser,
    Path(user_id): Path<Uuid>,
    Query(params): Query<AuditLogQueryParams>,
) -> Result<Json<AuditLogListResponse>, (StatusCode, String)> {
    require_super_admin(&user)?;
    let logs = state
        .audit_log_repo
        .get_user_logs(
            user_id,
            params.limit.unwrap_or(100),
            params.offset.unwrap_or(0),
        )
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let log_responses: Vec<AuditLogResponse> = logs
        .into_iter()
        .map(|log| AuditLogResponse {
            id: log.id,
            user_id: log.user_id,
            // P1-04: canonical_name() gives stable output, not Rust Debug repr.
            action: log.action.canonical_name().to_string(),
            resource_type: log.resource_type,
            resource_id: log.resource_id,
            org_id: log.org_id,
            details: log.details,
            ip_address: log.ip_address,
            user_agent: log.user_agent,
            created_at: log.created_at,
        })
        .collect();

    let total = log_responses.len() as i64;

    Ok(Json(AuditLogListResponse {
        logs: log_responses,
        total,
        limit: params.limit.unwrap_or(100),
        offset: params.offset.unwrap_or(0),
    }))
}

/// Integrity verification response.
#[derive(Debug, Serialize)]
pub struct IntegrityResponse {
    pub verified: bool,
    pub entries_checked: i64,
    pub checked_at: DateTime<Utc>,
    pub message: String,
}

/// Verify audit log integrity.
async fn verify_audit_integrity(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<IntegrityResponse>, (StatusCode, String)> {
    require_super_admin(&user)?;
    let entries_to_check: i64 = 10000;

    let verified = state
        .audit_log_repo
        .verify_integrity(Some(entries_to_check))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let message = if verified {
        "All audit log entries passed integrity verification".to_string()
    } else {
        "WARNING: Integrity violations detected in audit log chain".to_string()
    };

    Ok(Json(IntegrityResponse {
        verified,
        entries_checked: entries_to_check,
        checked_at: Utc::now(),
        message,
    }))
}

// ============================================================================
// GDPR REPORTS
// ============================================================================

/// Data export report entry.
#[derive(Debug, Serialize)]
pub struct DataExportReportEntry {
    pub id: Uuid,
    pub user_id: Uuid,
    pub status: String,
    pub format: String,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub downloaded: bool,
}

/// Data export report response.
#[derive(Debug, Serialize)]
pub struct DataExportReportResponse {
    pub exports: Vec<DataExportReportEntry>,
    pub total_requests: i64,
    pub completed_count: i64,
    pub pending_count: i64,
    pub downloaded_count: i64,
}

/// Get data export report.
async fn get_data_export_report(
    State(state): State<AppState>,
    user: AuthUser,
    Query(params): Query<ReportQueryParams>,
) -> Result<Json<DataExportReportResponse>, (StatusCode, String)> {
    require_platform_admin(&user)?;

    // GDPR compliance figures come straight from `data_export_requests` so the
    // report reflects true platform activity (this endpoint previously returned
    // an empty list and hard-coded zeros for the completed/downloaded counts).
    // `org_id` is not applied here — export requests are user-scoped, not
    // org-scoped; `from_date`/`to_date` bound `created_at`.
    let summary = state
        .data_export_repo
        .report_summary(params.from_date, params.to_date, 1000)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let exports = summary
        .entries
        .into_iter()
        .map(|r| DataExportReportEntry {
            id: r.id,
            user_id: r.user_id,
            status: r.status.as_str().to_string(),
            format: r.format,
            created_at: r.created_at,
            completed_at: r.completed_at,
            downloaded: r.downloaded_at.is_some(),
        })
        .collect();

    Ok(Json(DataExportReportResponse {
        exports,
        total_requests: summary.total_requests,
        completed_count: summary.completed_count,
        pending_count: summary.pending_count,
        downloaded_count: summary.downloaded_count,
    }))
}

/// Deletion request report entry.
#[derive(Debug, Serialize)]
pub struct DeletionRequestReportEntry {
    pub user_id: Uuid,
    pub user_email: String,
    pub scheduled_for: DateTime<Utc>,
    pub days_remaining: i64,
}

/// Deletion requests report response.
#[derive(Debug, Serialize)]
pub struct DeletionRequestsReportResponse {
    pub requests: Vec<DeletionRequestReportEntry>,
    pub total_pending: i64,
}

/// Get deletion requests report.
async fn get_deletion_requests_report(
    State(_state): State<AppState>,
    user: AuthUser,
    mut rls: RlsConnection,
) -> Result<Json<DeletionRequestsReportResponse>, (StatusCode, String)> {
    require_platform_admin(&user)?;
    // Query users with scheduled_deletion_at set
    let rows: Vec<(Uuid, String, DateTime<Utc>)> = sqlx::query_as(
        r#"
        SELECT id, email, scheduled_deletion_at
        FROM users
        WHERE scheduled_deletion_at IS NOT NULL
        ORDER BY scheduled_deletion_at ASC
        "#,
    )
    .fetch_all(&mut **rls.conn())
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let now = Utc::now();
    let requests: Vec<DeletionRequestReportEntry> = rows
        .into_iter()
        .map(|(user_id, user_email, scheduled_for)| {
            let days_remaining = (scheduled_for - now).num_days().max(0);
            DeletionRequestReportEntry {
                user_id,
                user_email,
                scheduled_for,
                days_remaining,
            }
        })
        .collect();

    let total_pending = requests.len() as i64;

    rls.release().await;
    Ok(Json(DeletionRequestsReportResponse {
        requests,
        total_pending,
    }))
}

/// Privacy settings report entry.
#[derive(Debug, Serialize)]
pub struct PrivacySettingsReportEntry {
    pub visibility: String,
    pub count: i64,
}

/// Privacy settings report response.
#[derive(Debug, Serialize)]
pub struct PrivacySettingsReportResponse {
    pub visibility_distribution: Vec<PrivacySettingsReportEntry>,
    pub show_contact_info_count: i64,
    pub total_users: i64,
}

/// Get privacy settings report.
async fn get_privacy_settings_report(
    State(_state): State<AppState>,
    user: AuthUser,
    mut rls: RlsConnection,
) -> Result<Json<PrivacySettingsReportResponse>, (StatusCode, String)> {
    require_platform_admin(&user)?;
    // Get visibility distribution
    let visibility_rows: Vec<(String, i64)> = sqlx::query_as(
        r#"
        SELECT profile_visibility, COUNT(*) as count
        FROM users
        WHERE status = 'active'
        GROUP BY profile_visibility
        "#,
    )
    .fetch_all(&mut **rls.conn())
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let visibility_distribution: Vec<PrivacySettingsReportEntry> = visibility_rows
        .into_iter()
        .map(|(visibility, count)| PrivacySettingsReportEntry { visibility, count })
        .collect();

    // Get show_contact_info count
    let (show_contact_count,): (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*) FROM users
        WHERE status = 'active' AND show_contact_info = true
        "#,
    )
    .fetch_one(&mut **rls.conn())
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Get total users
    let (total_users,): (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*) FROM users WHERE status = 'active'
        "#,
    )
    .fetch_one(&mut **rls.conn())
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    rls.release().await;
    Ok(Json(PrivacySettingsReportResponse {
        visibility_distribution,
        show_contact_info_count: show_contact_count,
        total_users,
    }))
}

// ============================================================================
// SECURITY REPORTS
// ============================================================================

/// Login activity report entry.
#[derive(Debug, Serialize)]
pub struct LoginActivityEntry {
    pub date: String,
    pub successful_logins: i64,
    pub failed_logins: i64,
}

/// Login activity report response.
#[derive(Debug, Serialize)]
pub struct LoginActivityReportResponse {
    pub activity: Vec<LoginActivityEntry>,
    pub total_logins: i64,
    pub total_failed: i64,
    pub period_days: i64,
}

/// Get login activity report.
async fn get_login_activity_report(
    State(_state): State<AppState>,
    user: AuthUser,
    mut rls: RlsConnection,
    Query(params): Query<ReportQueryParams>,
) -> Result<Json<LoginActivityReportResponse>, (StatusCode, String)> {
    require_platform_admin(&user)?;
    let from_date = params
        .from_date
        .unwrap_or_else(|| Utc::now() - chrono::Duration::days(30));
    let to_date = params.to_date.unwrap_or_else(Utc::now);

    // Query login activity from audit logs
    let rows: Vec<(String, String, i64)> = sqlx::query_as(
        r#"
        SELECT DATE(created_at)::text as date, action::text, COUNT(*) as count
        FROM audit_logs
        WHERE action IN ('login', 'login_failed')
          AND created_at >= $1
          AND created_at <= $2
        GROUP BY DATE(created_at), action
        ORDER BY DATE(created_at) DESC
        "#,
    )
    .bind(from_date)
    .bind(to_date)
    .fetch_all(&mut **rls.conn())
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Aggregate by date
    let mut activity_map: std::collections::HashMap<String, (i64, i64)> =
        std::collections::HashMap::new();

    let mut total_logins = 0i64;
    let mut total_failed = 0i64;

    for (date, action, count) in rows {
        let entry = activity_map.entry(date).or_insert((0, 0));
        if action == "login" {
            entry.0 += count;
            total_logins += count;
        } else {
            entry.1 += count;
            total_failed += count;
        }
    }

    let mut activity: Vec<LoginActivityEntry> = activity_map
        .into_iter()
        .map(|(date, (successful, failed))| LoginActivityEntry {
            date,
            successful_logins: successful,
            failed_logins: failed,
        })
        .collect();

    activity.sort_by(|a, b| b.date.cmp(&a.date));

    let period_days = (to_date - from_date).num_days();

    rls.release().await;
    Ok(Json(LoginActivityReportResponse {
        activity,
        total_logins,
        total_failed,
        period_days,
    }))
}

/// MFA status report entry.
#[derive(Debug, Serialize)]
pub struct MfaStatusEntry {
    pub status: String,
    pub count: i64,
}

/// MFA status report response.
#[derive(Debug, Serialize)]
pub struct MfaStatusReportResponse {
    pub mfa_enabled_count: i64,
    pub mfa_disabled_count: i64,
    pub total_users: i64,
    pub adoption_rate: f64,
}

/// Get MFA status report.
async fn get_mfa_status_report(
    State(_state): State<AppState>,
    user: AuthUser,
    mut rls: RlsConnection,
) -> Result<Json<MfaStatusReportResponse>, (StatusCode, String)> {
    require_platform_admin(&user)?;
    // Count users with MFA enabled
    let (mfa_enabled,): (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*) FROM user_2fa WHERE enabled = true
        "#,
    )
    .fetch_one(&mut **rls.conn())
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Count total active users
    let (total_users,): (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*) FROM users WHERE status = 'active'
        "#,
    )
    .fetch_one(&mut **rls.conn())
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mfa_disabled = total_users - mfa_enabled;
    let adoption_rate = if total_users > 0 {
        (mfa_enabled as f64 / total_users as f64) * 100.0
    } else {
        0.0
    };

    rls.release().await;
    Ok(Json(MfaStatusReportResponse {
        mfa_enabled_count: mfa_enabled,
        mfa_disabled_count: mfa_disabled,
        total_users,
        adoption_rate,
    }))
}

/// Failed login report entry.
#[derive(Debug, Serialize)]
pub struct FailedLoginEntry {
    pub user_id: Option<Uuid>,
    pub ip_address: Option<String>,
    pub attempt_count: i64,
    pub last_attempt: DateTime<Utc>,
}

/// Failed logins report response.
#[derive(Debug, Serialize)]
pub struct FailedLoginsReportResponse {
    pub failed_logins: Vec<FailedLoginEntry>,
    pub total_failed: i64,
    pub unique_ips: i64,
}

/// Get failed logins report.
async fn get_failed_logins_report(
    State(_state): State<AppState>,
    user: AuthUser,
    mut rls: RlsConnection,
    Query(params): Query<ReportQueryParams>,
) -> Result<Json<FailedLoginsReportResponse>, (StatusCode, String)> {
    require_platform_admin(&user)?;
    let from_date = params
        .from_date
        .unwrap_or_else(|| Utc::now() - chrono::Duration::days(7));

    // Get failed login attempts grouped by IP
    let rows: Vec<(Option<Uuid>, Option<String>, i64, DateTime<Utc>)> = sqlx::query_as(
        r#"
        SELECT user_id, ip_address, COUNT(*) as count, MAX(created_at) as last_attempt
        FROM audit_logs
        WHERE action = 'login_failed'
          AND created_at >= $1
        GROUP BY user_id, ip_address
        HAVING COUNT(*) >= 3
        ORDER BY count DESC
        LIMIT 100
        "#,
    )
    .bind(from_date)
    .fetch_all(&mut **rls.conn())
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let failed_logins: Vec<FailedLoginEntry> = rows
        .into_iter()
        .map(
            |(user_id, ip_address, attempt_count, last_attempt)| FailedLoginEntry {
                user_id,
                ip_address,
                attempt_count,
                last_attempt,
            },
        )
        .collect();

    // Get total failed count
    let (total_failed,): (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*) FROM audit_logs
        WHERE action = 'login_failed' AND created_at >= $1
        "#,
    )
    .bind(from_date)
    .fetch_one(&mut **rls.conn())
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Get unique IPs
    let (unique_ips,): (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(DISTINCT ip_address) FROM audit_logs
        WHERE action = 'login_failed' AND created_at >= $1
        "#,
    )
    .bind(from_date)
    .fetch_one(&mut **rls.conn())
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    rls.release().await;
    Ok(Json(FailedLoginsReportResponse {
        failed_logins,
        total_failed,
        unique_ips,
    }))
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Parse audit action from string.
fn parse_audit_action(s: &str) -> Option<AuditAction> {
    match s.to_lowercase().as_str() {
        "login" => Some(AuditAction::Login),
        "logout" => Some(AuditAction::Logout),
        "login_failed" => Some(AuditAction::LoginFailed),
        "password_changed" => Some(AuditAction::PasswordChanged),
        "password_reset_requested" => Some(AuditAction::PasswordResetRequested),
        "password_reset_completed" => Some(AuditAction::PasswordResetCompleted),
        "mfa_enabled" => Some(AuditAction::MfaEnabled),
        "mfa_disabled" => Some(AuditAction::MfaDisabled),
        "mfa_backup_code_used" => Some(AuditAction::MfaBackupCodeUsed),
        "mfa_backup_codes_regenerated" => Some(AuditAction::MfaBackupCodesRegenerated),
        "account_created" => Some(AuditAction::AccountCreated),
        "account_updated" => Some(AuditAction::AccountUpdated),
        "account_suspended" => Some(AuditAction::AccountSuspended),
        "account_reactivated" => Some(AuditAction::AccountReactivated),
        "account_deleted" => Some(AuditAction::AccountDeleted),
        "data_export_requested" => Some(AuditAction::DataExportRequested),
        "data_export_downloaded" => Some(AuditAction::DataExportDownloaded),
        "data_deletion_requested" => Some(AuditAction::DataDeletionRequested),
        "data_deletion_cancelled" => Some(AuditAction::DataDeletionCancelled),
        "data_deletion_completed" => Some(AuditAction::DataDeletionCompleted),
        "privacy_settings_updated" => Some(AuditAction::PrivacySettingsUpdated),
        "role_assigned" => Some(AuditAction::RoleAssigned),
        "role_removed" => Some(AuditAction::RoleRemoved),
        "permissions_changed" => Some(AuditAction::PermissionsChanged),
        "org_member_added" => Some(AuditAction::OrgMemberAdded),
        "org_member_removed" => Some(AuditAction::OrgMemberRemoved),
        "org_settings_changed" => Some(AuditAction::OrgSettingsChanged),
        "resource_created" => Some(AuditAction::ResourceCreated),
        "resource_updated" => Some(AuditAction::ResourceUpdated),
        "resource_deleted" => Some(AuditAction::ResourceDeleted),
        "resource_accessed" => Some(AuditAction::ResourceAccessed),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::TenantRole;

    fn user_with_role(role: Option<TenantRole>) -> AuthUser {
        AuthUser {
            user_id: Uuid::nil(),
            email: "compliance@example.test".to_string(),
            name: "Compliance Officer".to_string(),
            tenant_id: None,
            role,
        }
    }

    #[test]
    fn platform_tier_roles_pass_the_guard() {
        // Both platform-tier roles must reach the platform-wide compliance
        // reports. Regression: the guard previously matched SuperAdmin exactly
        // and returned 403 for PlatformAdmin, breaking the platform-tier
        // hierarchy (SuperAdmin=100, PlatformAdmin=95).
        assert!(require_platform_admin(&user_with_role(Some(TenantRole::SuperAdmin))).is_ok());
        assert!(require_platform_admin(&user_with_role(Some(TenantRole::PlatformAdmin))).is_ok());
    }

    #[test]
    fn tenant_scoped_roles_and_anonymous_get_403() {
        for role in [
            Some(TenantRole::OrgAdmin),
            Some(TenantRole::Manager),
            Some(TenantRole::Owner),
            Some(TenantRole::Tenant),
            Some(TenantRole::Guest),
            None,
        ] {
            let err = require_platform_admin(&user_with_role(role)).unwrap_err();
            assert_eq!(err.0, StatusCode::FORBIDDEN);
        }
    }

    #[test]
    fn super_admin_guard_accepts_only_super_admin() {
        // Audit-log endpoints are super-admin only: even the platform-tier
        // PlatformAdmin must be rejected. (#2906)
        assert!(require_super_admin(&user_with_role(Some(TenantRole::SuperAdmin))).is_ok());

        for role in [
            Some(TenantRole::PlatformAdmin),
            Some(TenantRole::OrgAdmin),
            Some(TenantRole::Manager),
            Some(TenantRole::Owner),
            Some(TenantRole::Tenant),
            Some(TenantRole::Guest),
            None,
        ] {
            let err = require_super_admin(&user_with_role(role)).unwrap_err();
            assert_eq!(err.0, StatusCode::FORBIDDEN);
        }
    }
}
