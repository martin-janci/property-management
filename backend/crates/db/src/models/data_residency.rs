//! Data Residency models (Epic 146).
//!
//! Types for configuring and managing data residency controls
//! to meet regional compliance requirements (GDPR, etc.).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

// ============================================================================
// STORY 146.1: DATA RESIDENCY CONFIGURATION
// ============================================================================

/// Available data regions for storage.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema, Default,
)]
#[sqlx(type_name = "data_region", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum DataRegion {
    /// European Union (Frankfurt)
    #[default]
    EuWest,
    /// European Union (Paris)
    EuCentral,
    /// United States (Virginia)
    UsEast,
    /// United States (Oregon)
    UsWest,
    /// Asia Pacific (Singapore)
    ApacSoutheast,
    /// Asia Pacific (Sydney)
    ApacSouth,
    /// United Kingdom (London)
    UkSouth,
    /// Switzerland (Zurich)
    ChNorth,
}

impl DataRegion {
    /// Get the display name for this region.
    pub fn display_name(&self) -> &'static str {
        match self {
            DataRegion::EuWest => "EU West (Frankfurt)",
            DataRegion::EuCentral => "EU Central (Paris)",
            DataRegion::UsEast => "US East (Virginia)",
            DataRegion::UsWest => "US West (Oregon)",
            DataRegion::ApacSoutheast => "APAC Southeast (Singapore)",
            DataRegion::ApacSouth => "APAC South (Sydney)",
            DataRegion::UkSouth => "UK South (London)",
            DataRegion::ChNorth => "CH North (Zurich)",
        }
    }

    /// Get the compliance frameworks this region satisfies.
    pub fn compliance_frameworks(&self) -> Vec<&'static str> {
        match self {
            DataRegion::EuWest | DataRegion::EuCentral => {
                vec!["GDPR", "EU Data Residency", "Schrems II Compliant"]
            }
            DataRegion::UsEast | DataRegion::UsWest => {
                vec!["SOC 2", "HIPAA Eligible", "FedRAMP"]
            }
            DataRegion::ApacSoutheast | DataRegion::ApacSouth => {
                vec!["PDPA (Singapore)", "Privacy Act (Australia)"]
            }
            DataRegion::UkSouth => {
                vec!["UK GDPR", "Data Protection Act 2018"]
            }
            DataRegion::ChNorth => {
                vec!["Swiss FADP", "FINMA Compliant", "Banking Secrecy"]
            }
        }
    }

    /// Get the geographic location code.
    pub fn location_code(&self) -> &'static str {
        match self {
            DataRegion::EuWest => "eu-west-1",
            DataRegion::EuCentral => "eu-central-1",
            DataRegion::UsEast => "us-east-1",
            DataRegion::UsWest => "us-west-2",
            DataRegion::ApacSoutheast => "ap-southeast-1",
            DataRegion::ApacSouth => "ap-south-1",
            DataRegion::UkSouth => "uk-south-1",
            DataRegion::ChNorth => "ch-north-1",
        }
    }
}

/// Type of data for residency purposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "data_type_category", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum DataTypeCategory {
    /// Personal identifiable information (PII)
    PersonalData,
    /// Financial records and transactions
    FinancialData,
    /// Documents and attachments
    Documents,
    /// Audit logs and system records
    AuditLogs,
    /// Communication records (messages, emails)
    Communications,
    /// Analytics and reporting data
    Analytics,
    /// All data types
    All,
}

impl DataTypeCategory {
    /// Get the display name for this data type.
    pub fn display_name(&self) -> &'static str {
        match self {
            DataTypeCategory::PersonalData => "Personal Data (PII)",
            DataTypeCategory::FinancialData => "Financial Records",
            DataTypeCategory::Documents => "Documents & Attachments",
            DataTypeCategory::AuditLogs => "Audit Logs",
            DataTypeCategory::Communications => "Communications",
            DataTypeCategory::Analytics => "Analytics Data",
            DataTypeCategory::All => "All Data Types",
        }
    }

    /// Get the description for this data type.
    pub fn description(&self) -> &'static str {
        match self {
            DataTypeCategory::PersonalData => {
                "Names, addresses, contact information, identification numbers"
            }
            DataTypeCategory::FinancialData => {
                "Invoices, payments, bank details, financial statements"
            }
            DataTypeCategory::Documents => "Leases, contracts, uploaded files, photos",
            DataTypeCategory::AuditLogs => "System activity logs, access records, change history",
            DataTypeCategory::Communications => "Messages, emails, notifications, chat history",
            DataTypeCategory::Analytics => "Reports, dashboards, aggregated statistics",
            DataTypeCategory::All => "All organization data including backups",
        }
    }

    /// Check if this data type requires special handling.
    pub fn requires_special_handling(&self) -> bool {
        matches!(
            self,
            DataTypeCategory::PersonalData | DataTypeCategory::FinancialData
        )
    }

    /// Stable snake_case name matching the `serde(rename_all = "snake_case")`
    /// convention. Use this instead of `{:?}` in audit/log records so the
    /// output does not expose Rust-internal PascalCase Debug output.
    pub fn as_str(&self) -> &'static str {
        match self {
            DataTypeCategory::PersonalData => "personal_data",
            DataTypeCategory::FinancialData => "financial_data",
            DataTypeCategory::Documents => "documents",
            DataTypeCategory::AuditLogs => "audit_logs",
            DataTypeCategory::Communications => "communications",
            DataTypeCategory::Analytics => "analytics",
            DataTypeCategory::All => "all",
        }
    }
}

impl std::fmt::Display for DataTypeCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Data residency configuration status.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema, Default,
)]
#[sqlx(type_name = "residency_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ResidencyStatus {
    /// Configuration is active and enforced
    #[default]
    Active,
    /// Migration in progress to new region
    Migrating,
    /// Configuration is pending activation
    Pending,
    /// Configuration has been suspended
    Suspended,
}

/// Data residency configuration for an organization.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct DataResidencyConfig {
    /// Configuration ID
    pub id: Uuid,
    /// Organization this configuration applies to
    pub organization_id: Uuid,
    /// Primary data storage region
    pub primary_region: String,
    /// Backup data storage region
    pub backup_region: Option<String>,
    /// Configuration status
    pub status: String,
    /// Whether cross-region access is allowed (for disaster recovery)
    pub allow_cross_region_access: bool,
    /// Data types with specific region requirements (JSON)
    pub data_type_overrides: Option<serde_json::Value>,
    /// Compliance notes
    pub compliance_notes: Option<String>,
    /// Last compliance verification timestamp
    pub last_verified_at: Option<DateTime<Utc>>,
    /// User who created the configuration
    pub created_by: Uuid,
    /// When configuration was created
    pub created_at: DateTime<Utc>,
    /// When configuration was last updated
    pub updated_at: DateTime<Utc>,
}

/// Request to configure data residency.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ConfigureDataResidency {
    /// Primary data storage region
    pub primary_region: DataRegion,
    /// Backup data storage region (optional)
    pub backup_region: Option<DataRegion>,
    /// Whether to allow cross-region access for disaster recovery
    #[serde(default)]
    pub allow_cross_region_access: bool,
    /// Override regions for specific data types
    pub data_type_overrides: Option<Vec<DataTypeOverride>>,
    /// Compliance notes
    pub compliance_notes: Option<String>,
}

/// Override for a specific data type's region.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DataTypeOverride {
    /// Data type category
    pub data_type: DataTypeCategory,
    /// Region for this data type
    pub region: DataRegion,
    /// Reason for override
    pub reason: Option<String>,
}

/// Response for data residency configuration.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DataResidencyConfigResponse {
    /// Configuration ID
    pub id: Uuid,
    /// Organization ID
    pub organization_id: Uuid,
    /// Primary region
    pub primary_region: DataRegion,
    /// Primary region display name
    pub primary_region_display: String,
    /// Backup region
    pub backup_region: Option<DataRegion>,
    /// Backup region display name
    pub backup_region_display: Option<String>,
    /// Configuration status
    pub status: ResidencyStatus,
    /// Whether cross-region access is allowed
    pub allow_cross_region_access: bool,
    /// Data type specific overrides
    pub data_type_overrides: Vec<DataTypeOverride>,
    /// Compliance frameworks satisfied
    pub compliance_frameworks: Vec<String>,
    /// Compliance implications
    pub compliance_implications: Vec<ComplianceImplication>,
    /// Last verification timestamp
    pub last_verified_at: Option<DateTime<Utc>>,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Updated timestamp
    pub updated_at: DateTime<Utc>,
}

/// Compliance implication for a configuration choice.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ComplianceImplication {
    /// Implication category (info, warning, requirement)
    pub level: ImplicationLevel,
    /// Title of the implication
    pub title: String,
    /// Detailed description
    pub description: String,
    /// Related regulation or standard
    pub regulation: Option<String>,
}

/// Level of compliance implication.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ImplicationLevel {
    /// Informational note
    Info,
    /// Warning that should be reviewed
    Warning,
    /// Requirement that must be addressed
    Requirement,
}

/// Available regions response.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AvailableRegionsResponse {
    /// List of available regions
    pub regions: Vec<RegionInfo>,
}

/// Information about a data region.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RegionInfo {
    /// Region identifier
    pub region: DataRegion,
    /// Display name
    pub display_name: String,
    /// Location code
    pub location_code: String,
    /// Compliance frameworks
    pub compliance_frameworks: Vec<String>,
    /// Whether region is available for selection
    pub available: bool,
}

// ============================================================================
// STORY 146.2: REGIONAL DATA ROUTING
// ============================================================================

/// Data routing configuration.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct DataRoutingRule {
    /// Rule ID
    pub id: Uuid,
    /// Organization ID
    pub organization_id: Uuid,
    /// Data type this rule applies to
    pub data_type: String,
    /// Target region for writes
    pub write_region: String,
    /// Preferred region for reads
    pub read_region: String,
    /// Whether rule is active
    pub is_active: bool,
    /// Priority (lower = higher priority)
    pub priority: i32,
    /// When rule was created
    pub created_at: DateTime<Utc>,
    /// When rule was last updated
    pub updated_at: DateTime<Utc>,
}

/// Cross-region data access log entry.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct CrossRegionAccessLog {
    /// Log entry ID
    pub id: Uuid,
    /// Organization ID
    pub organization_id: Uuid,
    /// User who accessed data
    pub user_id: Uuid,
    /// Type of data accessed
    pub data_type: String,
    /// Source region of the data
    pub source_region: String,
    /// Region from which access was made
    pub access_region: String,
    /// Type of access (read, write)
    pub access_type: String,
    /// Resource identifier
    pub resource_id: Option<String>,
    /// Reason for cross-region access
    pub reason: Option<String>,
    /// IP address of requester
    pub ip_address: Option<String>,
    /// When access occurred
    pub accessed_at: DateTime<Utc>,
}

/// Log a cross-region access.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct LogCrossRegionAccess {
    /// Type of data accessed
    pub data_type: DataTypeCategory,
    /// Source region of the data
    pub source_region: DataRegion,
    /// Region from which access was made
    pub access_region: DataRegion,
    /// Type of access
    pub access_type: AccessType,
    /// Resource identifier
    pub resource_id: Option<String>,
    /// Reason for cross-region access
    pub reason: Option<String>,
}

/// Type of data access.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "access_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum AccessType {
    /// Read access
    Read,
    /// Write access
    Write,
    /// Delete access
    Delete,
    /// Migration access
    Migration,
}

impl AccessType {
    /// Stable snake_case name. Use instead of `{:?}` in audit/log records.
    pub fn as_str(&self) -> &'static str {
        match self {
            AccessType::Read => "read",
            AccessType::Write => "write",
            AccessType::Delete => "delete",
            AccessType::Migration => "migration",
        }
    }
}

impl std::fmt::Display for AccessType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Data routing status response.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DataRoutingStatus {
    /// Organization ID
    pub organization_id: Uuid,
    /// Primary region
    pub primary_region: DataRegion,
    /// Backup region
    pub backup_region: Option<DataRegion>,
    /// Routing rules in effect
    pub routing_rules: Vec<RoutingRuleSummary>,
    /// Recent cross-region accesses
    pub recent_cross_region_accesses: i64,
    /// Status summary
    pub status: String,
}

/// Summary of a routing rule.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RoutingRuleSummary {
    /// Data type
    pub data_type: DataTypeCategory,
    /// Write region
    pub write_region: DataRegion,
    /// Read region
    pub read_region: DataRegion,
    /// Is active
    pub is_active: bool,
}

// ============================================================================
// STORY 146.3: DATA RESIDENCY COMPLIANCE VERIFICATION
// ============================================================================

/// Request to run compliance verification.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct RunComplianceVerification {
    /// Include detailed location breakdown
    #[serde(default)]
    pub include_details: bool,
    /// Check specific data types only
    pub data_types: Option<Vec<DataTypeCategory>>,
}

/// Compliance verification result.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct ComplianceVerificationResult {
    /// Verification ID
    pub id: Uuid,
    /// Organization ID
    pub organization_id: Uuid,
    /// Overall compliance status
    pub is_compliant: bool,
    /// Verification timestamp
    pub verified_at: DateTime<Utc>,
    /// Summary of data locations (JSON)
    pub data_locations: serde_json::Value,
    /// Any compliance issues found (JSON)
    pub issues: Option<serde_json::Value>,
    /// Verification details (JSON)
    pub details: Option<serde_json::Value>,
    /// User who ran verification
    pub verified_by: Uuid,
}

/// Compliance verification response.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ComplianceVerificationResponse {
    /// Verification ID
    pub id: Uuid,
    /// Organization ID
    pub organization_id: Uuid,
    /// Overall compliance status
    pub compliance_status: ComplianceStatus,
    /// Whether fully compliant
    pub is_compliant: bool,
    /// Verification timestamp
    pub verified_at: DateTime<Utc>,
    /// Data location breakdown
    pub data_locations: Vec<DataLocationSummary>,
    /// Any data found outside configured regions
    pub out_of_region_data: Vec<OutOfRegionData>,
    /// Recent data access by region
    pub access_by_region: Vec<RegionAccessSummary>,
    /// Compliance issues
    pub issues: Vec<ComplianceIssue>,
    /// Report can be exported
    pub report_available: bool,
}

/// Overall compliance status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ComplianceStatus {
    /// Fully compliant
    Compliant,
    /// Mostly compliant with minor issues
    PartiallyCompliant,
    /// Not compliant - action required
    NonCompliant,
    /// Verification in progress
    Verifying,
}

/// Summary of data in a specific location.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DataLocationSummary {
    /// Data type
    pub data_type: DataTypeCategory,
    /// Region where data is stored
    pub region: DataRegion,
    /// Configured region for this data
    pub configured_region: DataRegion,
    /// Whether location matches configuration
    pub is_correct_location: bool,
    /// Approximate record count
    pub record_count: i64,
    /// Last updated timestamp
    pub last_updated: Option<DateTime<Utc>>,
}

/// Data found outside configured regions.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OutOfRegionData {
    /// Data type
    pub data_type: DataTypeCategory,
    /// Current region
    pub current_region: DataRegion,
    /// Expected region
    pub expected_region: DataRegion,
    /// Record count
    pub record_count: i64,
    /// Reason (if known)
    pub reason: Option<String>,
    /// Recommended action
    pub recommended_action: String,
}

/// Summary of data access by region.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RegionAccessSummary {
    /// Region
    pub region: DataRegion,
    /// Read access count
    pub read_count: i64,
    /// Write access count
    pub write_count: i64,
    /// Cross-region access count
    pub cross_region_count: i64,
    /// Period (e.g., "last_24h", "last_7d")
    pub period: String,
}

/// Compliance issue found during verification.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ComplianceIssue {
    /// Issue severity
    pub severity: IssueSeverity,
    /// Issue title
    pub title: String,
    /// Detailed description
    pub description: String,
    /// Affected data type
    pub data_type: Option<DataTypeCategory>,
    /// Affected region
    pub region: Option<DataRegion>,
    /// Recommended resolution
    pub resolution: String,
}

/// Severity of compliance issue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum IssueSeverity {
    /// Low severity - informational
    Low,
    /// Medium severity - should be addressed
    Medium,
    /// High severity - must be addressed
    High,
    /// Critical - immediate action required
    Critical,
}

// ============================================================================
// STORY 146.4: DATA RESIDENCY AUDIT TRAIL
// ============================================================================

/// Data residency audit log entry.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct DataResidencyAuditLog {
    /// Audit log ID
    pub id: Uuid,
    /// Organization ID
    pub organization_id: Uuid,
    /// Type of audit event
    pub event_type: String,
    /// User who triggered the event
    pub user_id: Option<Uuid>,
    /// Previous configuration (JSON)
    pub previous_state: Option<serde_json::Value>,
    /// New configuration (JSON)
    pub new_state: Option<serde_json::Value>,
    /// Event details (JSON)
    pub details: Option<serde_json::Value>,
    /// IP address of requester
    pub ip_address: Option<String>,
    /// User agent
    pub user_agent: Option<String>,
    /// Hash for tamper evidence
    pub record_hash: String,
    /// Previous record hash (for chain)
    pub previous_hash: Option<String>,
    /// When event occurred
    pub created_at: DateTime<Utc>,
}

/// Audit event types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "residency_audit_event", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ResidencyAuditEvent {
    /// Configuration was created
    ConfigurationCreated,
    /// Configuration was updated
    ConfigurationUpdated,
    /// Region was changed
    RegionChanged,
    /// Data migration started
    MigrationStarted,
    /// Data migration completed
    MigrationCompleted,
    /// Compliance check performed
    ComplianceCheckPerformed,
    /// Cross-region access occurred
    CrossRegionAccess,
    /// Data type override added
    OverrideAdded,
    /// Data type override removed
    OverrideRemoved,
}

impl ResidencyAuditEvent {
    /// Stable snake_case name. Use instead of `{:?}` in audit/hash records so
    /// the output does not expose internal Rust PascalCase Debug format.
    pub fn as_str(&self) -> &'static str {
        match self {
            ResidencyAuditEvent::ConfigurationCreated => "configuration_created",
            ResidencyAuditEvent::ConfigurationUpdated => "configuration_updated",
            ResidencyAuditEvent::RegionChanged => "region_changed",
            ResidencyAuditEvent::MigrationStarted => "migration_started",
            ResidencyAuditEvent::MigrationCompleted => "migration_completed",
            ResidencyAuditEvent::ComplianceCheckPerformed => "compliance_check_performed",
            ResidencyAuditEvent::CrossRegionAccess => "cross_region_access",
            ResidencyAuditEvent::OverrideAdded => "override_added",
            ResidencyAuditEvent::OverrideRemoved => "override_removed",
        }
    }
}

impl std::fmt::Display for ResidencyAuditEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Query parameters for audit log.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct AuditLogQuery {
    /// Filter by event type
    pub event_type: Option<ResidencyAuditEvent>,
    /// Filter by user
    pub user_id: Option<Uuid>,
    /// From date
    pub from_date: Option<DateTime<Utc>>,
    /// To date
    pub to_date: Option<DateTime<Utc>>,
    /// Page limit
    pub limit: Option<i64>,
    /// Page offset
    pub offset: Option<i64>,
}

/// Audit log response.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AuditLogResponse {
    /// Audit log entries
    pub entries: Vec<AuditLogEntry>,
    /// Total count
    pub total_count: i64,
    /// Whether chain is valid (tamper-evident)
    pub chain_valid: bool,
}

/// Audit log entry for display.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AuditLogEntry {
    /// Entry ID
    pub id: Uuid,
    /// Event type
    pub event_type: ResidencyAuditEvent,
    /// Event description
    pub description: String,
    /// User who triggered event
    pub user_id: Option<Uuid>,
    /// User name (if available)
    pub user_name: Option<String>,
    /// Changes made
    pub changes: Option<Vec<AuditChange>>,
    /// Additional details
    pub details: Option<serde_json::Value>,
    /// IP address
    pub ip_address: Option<String>,
    /// Timestamp
    pub created_at: DateTime<Utc>,
    /// Is tamper-evident chain valid up to this entry
    pub chain_valid: bool,
}

/// A single change in an audit entry.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AuditChange {
    /// Field that changed
    pub field: String,
    /// Previous value
    pub old_value: Option<String>,
    /// New value
    pub new_value: Option<String>,
}

/// Create audit log entry request.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateAuditLogEntry {
    /// Event type
    pub event_type: ResidencyAuditEvent,
    /// Previous state
    pub previous_state: Option<serde_json::Value>,
    /// New state
    pub new_state: Option<serde_json::Value>,
    /// Additional details
    pub details: Option<serde_json::Value>,
}

// ============================================================================
// DASHBOARD & SUMMARY TYPES
// ============================================================================

/// Data residency dashboard summary.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DataResidencyDashboard {
    /// Organization ID
    pub organization_id: Uuid,
    /// Current configuration
    pub configuration: DataResidencyConfigResponse,
    /// Compliance status
    pub compliance_status: ComplianceStatus,
    /// Last verification result
    pub last_verification: Option<ComplianceVerificationResponse>,
    /// Recent audit events
    pub recent_events: Vec<AuditLogEntry>,
    /// Cross-region access stats
    pub cross_region_stats: CrossRegionStats,
}

/// Cross-region access statistics.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CrossRegionStats {
    /// Total cross-region accesses (last 24h)
    pub last_24h: i64,
    /// Total cross-region accesses (last 7d)
    pub last_7d: i64,
    /// Total cross-region accesses (last 30d)
    pub last_30d: i64,
    /// By access type
    pub by_type: Vec<AccessTypeCount>,
}

/// Count by access type.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AccessTypeCount {
    /// Access type
    pub access_type: AccessType,
    /// Count
    pub count: i64,
}
