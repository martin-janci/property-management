//! Audit log models (Epic 9, Story 9.6).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Audit action types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "audit_action", rename_all = "snake_case")]
pub enum AuditAction {
    // Authentication actions
    Login,
    Logout,
    LoginFailed,
    PasswordChanged,
    PasswordResetRequested,
    PasswordResetCompleted,
    // 2FA actions
    MfaEnabled,
    MfaDisabled,
    MfaBackupCodeUsed,
    MfaBackupCodesRegenerated,
    // Account actions
    AccountCreated,
    AccountUpdated,
    AccountSuspended,
    AccountReactivated,
    AccountDeleted,
    // GDPR actions
    DataExportRequested,
    DataExportDownloaded,
    DataDeletionRequested,
    DataDeletionCancelled,
    DataDeletionCompleted,
    // Privacy actions
    PrivacySettingsUpdated,
    // Role/permission actions
    RoleAssigned,
    RoleRemoved,
    PermissionsChanged,
    // Organization actions
    OrgMemberAdded,
    OrgMemberRemoved,
    OrgSettingsChanged,
    // Generic CRUD
    ResourceCreated,
    ResourceUpdated,
    ResourceDeleted,
    ResourceAccessed,
    // OAuth actions (Epic 10A) — explicit rename: `OAuth` snake_cases to
    // `o_auth`, but the Postgres enum values were defined as `oauth_*`.
    #[sqlx(rename = "oauth_authorize")]
    OAuthAuthorize,
    #[sqlx(rename = "oauth_revoke")]
    OAuthRevoke,
    #[sqlx(rename = "oauth_client_create")]
    OAuthClientCreate,
    #[sqlx(rename = "oauth_client_revoke")]
    OAuthClientRevoke,
    #[sqlx(rename = "oauth_client_secret_regenerate")]
    OAuthClientSecretRegenerate,
    // Phase 6 C17: principal_kind enforcement at token issuance
    #[sqlx(rename = "oauth_token_denied_principal_kind")]
    OAuthTokenDeniedPrincipalKind,
    // Dev-team review P1-01: RFC 9700 refresh-token replay detection.
    // Emitted when a refresh token whose hash matches an already-revoked
    // row is replayed; on emit, every active refresh token for the
    // affected user is revoked.
    RefreshTokenReplayDetected,
}

/// An audit log entry.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct AuditLog {
    /// Log entry ID
    pub id: Uuid,
    /// User who performed the action (None for system actions)
    pub user_id: Option<Uuid>,
    /// Type of action performed
    pub action: AuditAction,
    /// Type of resource affected
    pub resource_type: Option<String>,
    /// ID of the affected resource
    pub resource_id: Option<Uuid>,
    /// Organization context
    pub org_id: Option<Uuid>,
    /// Additional details about the action
    pub details: Option<serde_json::Value>,
    /// Previous state (for updates)
    pub old_values: Option<serde_json::Value>,
    /// New state (for updates)
    pub new_values: Option<serde_json::Value>,
    /// IP address of the request
    pub ip_address: Option<String>,
    /// User agent of the request
    pub user_agent: Option<String>,
    /// Hash for integrity verification
    pub integrity_hash: Option<String>,
    /// Previous entry hash for chain verification
    pub previous_hash: Option<String>,
    /// When the action occurred
    pub created_at: DateTime<Utc>,
}

/// Data for creating a new audit log entry.
#[derive(Debug, Clone)]
pub struct CreateAuditLog {
    /// User who performed the action
    pub user_id: Option<Uuid>,
    /// Type of action performed
    pub action: AuditAction,
    /// Type of resource affected
    pub resource_type: Option<String>,
    /// ID of the affected resource
    pub resource_id: Option<Uuid>,
    /// Organization context
    pub org_id: Option<Uuid>,
    /// Additional details
    pub details: Option<serde_json::Value>,
    /// Previous state
    pub old_values: Option<serde_json::Value>,
    /// New state
    pub new_values: Option<serde_json::Value>,
    /// IP address
    pub ip_address: Option<String>,
    /// User agent
    pub user_agent: Option<String>,
}

/// Query parameters for listing audit logs.
#[derive(Debug, Clone, Default)]
pub struct AuditLogQuery {
    /// Filter by user
    pub user_id: Option<Uuid>,
    /// Filter by action type
    pub action: Option<AuditAction>,
    /// Filter by resource type
    pub resource_type: Option<String>,
    /// Filter by resource ID
    pub resource_id: Option<Uuid>,
    /// Filter by organization
    pub org_id: Option<Uuid>,
    /// Filter by start date
    pub from_date: Option<DateTime<Utc>>,
    /// Filter by end date
    pub to_date: Option<DateTime<Utc>>,
    /// Pagination limit
    pub limit: Option<i64>,
    /// Pagination offset
    pub offset: Option<i64>,
}

/// Summary of audit logs for compliance reports.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogSummary {
    /// Total number of entries
    pub total_count: i64,
    /// Count by action type
    pub action_counts: Vec<ActionCount>,
    /// Date range
    pub from_date: Option<DateTime<Utc>>,
    pub to_date: Option<DateTime<Utc>>,
}

/// Count of actions by type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionCount {
    pub action: String,
    pub count: i64,
}
