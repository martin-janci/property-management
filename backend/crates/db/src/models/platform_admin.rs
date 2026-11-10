//! Platform Admin models (Epic 10B).
//!
//! Models for platform-wide administrative operations including
//! organization management, feature flags, system health, and announcements.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

// ==================== Organization Admin Models ====================

/// Organization metrics from admin view.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct OrganizationMetrics {
    pub organization_id: Uuid,
    pub name: String,
    pub slug: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub suspended_at: Option<DateTime<Utc>>,
    pub suspended_by: Option<Uuid>,
    pub suspension_reason: Option<String>,
    pub member_count: i64,
    pub active_member_count: i64,
    pub building_count: i64,
    pub unit_count: i64,
}

/// Summary view for organization list in admin dashboard.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AdminOrganizationSummary {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub status: String,
    pub member_count: i64,
    pub building_count: i64,
    pub created_at: DateTime<Utc>,
}

impl From<OrganizationMetrics> for AdminOrganizationSummary {
    fn from(m: OrganizationMetrics) -> Self {
        Self {
            id: m.organization_id,
            name: m.name,
            slug: m.slug,
            status: m.status,
            member_count: m.member_count,
            building_count: m.building_count,
            created_at: m.created_at,
        }
    }
}

/// Detailed organization view for admin drill-down.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AdminOrganizationDetail {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub contact_email: String,
    pub logo_url: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub suspended_at: Option<DateTime<Utc>>,
    pub suspended_by: Option<Uuid>,
    pub suspension_reason: Option<String>,
    /// Aggregated metrics
    pub metrics: OrganizationDetailMetrics,
}

/// Metrics subset for organization detail.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OrganizationDetailMetrics {
    pub member_count: i64,
    pub active_member_count: i64,
    pub building_count: i64,
    pub unit_count: i64,
}

/// Request to suspend an organization.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct SuspendOrganizationRequest {
    /// Reason for suspension (required for audit)
    pub reason: String,
    /// Whether to notify organization members
    #[serde(default)]
    pub notify_members: bool,
}

/// Request to reactivate an organization.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ReactivateOrganizationRequest {
    /// Optional note for reactivation
    pub note: Option<String>,
}

// ==================== Platform Settings Models ====================

/// Global, operator-controlled platform settings (singleton).
///
/// Backs the `platform_settings` table (migration 00229). Unlike the
/// per-tenant `tenant_settings` store, this is a single row shared across the
/// whole deployment, written via `PATCH /api/v1/platform-admin/settings`.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct PlatformSettings {
    /// Whether the platform is in maintenance mode.
    pub maintenance_mode: bool,
    /// Whether public sign-up is enabled.
    pub signup_enabled: bool,
    /// Operator support contact email (may be empty).
    pub support_email: String,
    /// When the settings were last written.
    pub updated_at: DateTime<Utc>,
    /// Operator who last wrote the settings (null after operator deletion).
    pub updated_by: Option<Uuid>,
}

// ==================== Feature Flag Models ====================

/// Feature flag entity.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct FeatureFlag {
    pub id: Uuid,
    /// Unique key for the flag (e.g., "new_dashboard")
    pub key: String,
    /// Human-readable name
    pub name: String,
    /// Description of what this flag controls
    pub description: Option<String>,
    /// Default enabled state
    pub is_enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Feature flag override for targeted enablement.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct FeatureFlagOverride {
    pub id: Uuid,
    pub flag_id: Uuid,
    /// Type of scope: 'organization', 'user', 'role'
    pub scope_type: String,
    /// ID of the scoped entity
    pub scope_id: Uuid,
    /// Override enabled state
    pub is_enabled: bool,
    pub created_at: DateTime<Utc>,
}

/// Scope type for feature flag overrides.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum FeatureFlagScope {
    Organization,
    User,
    Role,
}

impl FeatureFlagScope {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Organization => "organization",
            Self::User => "user",
            Self::Role => "role",
        }
    }
}

impl std::fmt::Display for FeatureFlagScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Request to create a new feature flag.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateFeatureFlagRequest {
    pub key: String,
    pub name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub is_enabled: bool,
}

/// Request to create a feature flag override.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateFeatureFlagOverrideRequest {
    pub scope_type: FeatureFlagScope,
    pub scope_id: Uuid,
    pub is_enabled: bool,
}

// ==================== Epic 107: Feature Descriptors & Catalog ====================

/// Feature category for organizing features.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct FeatureCategory {
    pub id: Uuid,
    /// Unique key for the category (e.g., "ai_features")
    pub key: String,
    pub name: String,
    pub description: Option<String>,
    /// Icon identifier (e.g., "sparkles", "chart-bar")
    pub icon: Option<String>,
    /// Hex color for visual grouping (e.g., "#3B82F6")
    pub color: Option<String>,
    /// Parent category for hierarchy
    pub parent_id: Option<Uuid>,
    /// Display order within parent/root
    pub display_order: i32,
    /// Localization data
    pub translations: serde_json::Value,
    /// Additional metadata
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Feature descriptor with rich metadata for display.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct FeatureDescriptor {
    pub id: Uuid,
    /// Reference to the feature flag
    pub feature_flag_id: Uuid,
    /// Display name for UI
    pub display_name: String,
    /// Short description (max 255 chars)
    pub short_description: Option<String>,
    /// Full description with details
    pub full_description: Option<String>,
    /// Icon identifier
    pub icon: Option<String>,
    /// Preview image URL
    pub preview_image_url: Option<String>,
    /// Category reference
    pub category_id: Option<Uuid>,
    /// Subcategory key
    pub subcategory: Option<String>,
    /// Tags for filtering
    pub tags: serde_json::Value,
    /// Localization data
    pub translations: serde_json::Value,
    /// Marketing benefits list
    pub benefits: serde_json::Value,
    /// Use case examples
    pub use_cases: serde_json::Value,
    /// Required OAuth scopes
    pub api_scopes: serde_json::Value,
    /// Feature dependencies (flag keys)
    pub depends_on: serde_json::Value,
    /// Conflicting features (flag keys)
    pub conflicts_with: serde_json::Value,
    /// Display order in UI
    pub display_order: i32,
    /// Whether to highlight this feature
    pub is_highlighted: bool,
    /// Badge text (e.g., "NEW", "BETA")
    pub badge_text: Option<String>,
    /// Show as locked/teaser when disabled
    pub show_teaser_when_disabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Access state for user type feature matrix.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "feature_access_state", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum FeatureAccessState {
    /// Feature is always included for this user type
    Included,
    /// Feature is optional - user can toggle
    Optional,
    /// Feature is not available for this user type
    Excluded,
}

impl FeatureAccessState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Included => "included",
            Self::Optional => "optional",
            Self::Excluded => "excluded",
        }
    }
}

impl std::fmt::Display for FeatureAccessState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// User type-based feature access configuration.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct FeatureUserTypeAccess {
    pub id: Uuid,
    pub feature_flag_id: Uuid,
    /// User type (e.g., "owner", "tenant", "manager")
    pub user_type: String,
    /// Access state for this user type
    pub access_state: FeatureAccessState,
    /// Can user override their preference for optional features
    pub can_override: bool,
    /// Default enabled state for optional features
    pub default_enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// User's preference for an optional feature.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct UserFeaturePreference {
    pub id: Uuid,
    pub user_id: Uuid,
    pub feature_flag_id: Uuid,
    /// User's preference
    pub is_enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request to create a feature category.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateFeatureCategoryRequest {
    pub key: String,
    pub name: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub parent_id: Option<Uuid>,
    #[serde(default)]
    pub display_order: i32,
    #[serde(default)]
    pub translations: serde_json::Value,
}

/// Request to update a feature category.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UpdateFeatureCategoryRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub parent_id: Option<Uuid>,
    pub display_order: Option<i32>,
    pub translations: Option<serde_json::Value>,
}

/// Request to create or update a feature descriptor.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UpsertFeatureDescriptorRequest {
    pub display_name: String,
    pub short_description: Option<String>,
    pub full_description: Option<String>,
    pub icon: Option<String>,
    pub preview_image_url: Option<String>,
    pub category_id: Option<Uuid>,
    pub subcategory: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub translations: serde_json::Value,
    #[serde(default)]
    pub benefits: Vec<String>,
    #[serde(default)]
    pub use_cases: serde_json::Value,
    #[serde(default)]
    pub api_scopes: Vec<String>,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub conflicts_with: Vec<String>,
    #[serde(default)]
    pub display_order: i32,
    #[serde(default)]
    pub is_highlighted: bool,
    pub badge_text: Option<String>,
    #[serde(default)]
    pub show_teaser_when_disabled: bool,
}

/// Request to configure user type feature access.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct SetFeatureUserTypeAccessRequest {
    pub user_type: String,
    pub access_state: FeatureAccessState,
    #[serde(default)]
    pub can_override: bool,
    #[serde(default = "default_true_fn")]
    pub default_enabled: bool,
}

fn default_true_fn() -> bool {
    true
}

/// Request to set user feature preference.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct SetUserFeaturePreferenceRequest {
    pub feature_key: String,
    pub is_enabled: bool,
}

/// Feature with descriptor for catalog display.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FeatureCatalogItem {
    pub key: String,
    pub descriptor: FeatureDescriptorDisplay,
    pub state: FeatureState,
    pub dependencies: Vec<String>,
    pub conflicts_with: Vec<String>,
}

/// Descriptor fields for display.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FeatureDescriptorDisplay {
    pub display_name: String,
    pub short_description: Option<String>,
    pub full_description: Option<String>,
    pub icon: Option<String>,
    pub preview_image_url: Option<String>,
    pub category: Option<FeatureCategorySummary>,
    pub tags: Vec<String>,
    pub benefits: Vec<String>,
    pub badge_text: Option<String>,
    pub is_highlighted: bool,
}

/// Feature state for the current user.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FeatureState {
    pub is_enabled: bool,
    pub access_state: FeatureAccessState,
    pub can_toggle: bool,
}

/// Category summary for display.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FeatureCategorySummary {
    pub key: String,
    pub name: String,
    pub icon: Option<String>,
    pub color: Option<String>,
}

/// Feature catalog query parameters.
#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct FeatureCatalogQuery {
    pub category: Option<String>,
    pub user_type: Option<String>,
    pub package_id: Option<Uuid>,
    pub search: Option<String>,
    pub locale: Option<String>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

/// Feature catalog response.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FeatureCatalogResponse {
    pub features: Vec<FeatureCatalogItem>,
    pub categories: Vec<CategoryWithCount>,
    pub pagination: CatalogPagination,
}

/// Category with feature count.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CategoryWithCount {
    pub key: String,
    pub name: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub count: i64,
}

/// Pagination info for catalog.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CatalogPagination {
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}

// ==================== System Announcement Models ====================

/// Severity levels for system announcements.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "VARCHAR", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum AnnouncementSeverity {
    Info,
    Warning,
    Critical,
}

impl AnnouncementSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Critical => "critical",
        }
    }
}

impl std::fmt::Display for AnnouncementSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// System announcement entity.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct SystemAnnouncement {
    pub id: Uuid,
    pub title: String,
    pub message: String,
    pub severity: String,
    pub start_at: DateTime<Utc>,
    pub end_at: Option<DateTime<Utc>>,
    pub is_dismissible: bool,
    pub requires_acknowledgment: bool,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// User acknowledgment of a system announcement.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct SystemAnnouncementAcknowledgment {
    pub id: Uuid,
    pub announcement_id: Uuid,
    pub user_id: Uuid,
    pub acknowledged_at: DateTime<Utc>,
}

/// Scheduled maintenance window.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct ScheduledMaintenance {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub start_at: DateTime<Utc>,
    pub end_at: DateTime<Utc>,
    pub is_read_only_mode: bool,
    pub announcement_id: Option<Uuid>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
}

/// Request to create a system announcement.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateSystemAnnouncementRequest {
    pub title: String,
    pub message: String,
    pub severity: AnnouncementSeverity,
    pub start_at: DateTime<Utc>,
    pub end_at: Option<DateTime<Utc>>,
    #[serde(default = "default_dismissible")]
    pub is_dismissible: bool,
    #[serde(default)]
    pub requires_acknowledgment: bool,
}

fn default_dismissible() -> bool {
    true
}

/// Request to schedule maintenance.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateMaintenanceRequest {
    pub title: String,
    pub description: Option<String>,
    pub start_at: DateTime<Utc>,
    pub end_at: DateTime<Utc>,
    #[serde(default)]
    pub is_read_only_mode: bool,
    /// Whether to create a system announcement for this maintenance
    #[serde(default = "default_true")]
    pub create_announcement: bool,
}

fn default_true() -> bool {
    true
}

// ==================== Health Monitoring Models ====================

/// Platform metric types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum MetricType {
    ApiLatency,
    ErrorRate,
    ActiveUsers,
    DatabaseConnections,
    MemoryUsage,
    DiskUsage,
    QueueDepth,
}

impl MetricType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ApiLatency => "api_latency",
            Self::ErrorRate => "error_rate",
            Self::ActiveUsers => "active_users",
            Self::DatabaseConnections => "database_connections",
            Self::MemoryUsage => "memory_usage",
            Self::DiskUsage => "disk_usage",
            Self::QueueDepth => "queue_depth",
        }
    }
}

/// Platform metric record.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct PlatformMetric {
    pub id: Uuid,
    pub metric_type: String,
    pub metric_name: String,
    pub value: f64,
    pub recorded_at: DateTime<Utc>,
    pub metadata: Option<serde_json::Value>,
}

/// Metric threshold configuration.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct MetricThreshold {
    pub id: Uuid,
    pub metric_name: String,
    pub warning_threshold: f64,
    pub critical_threshold: f64,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Alert for threshold breach.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct MetricAlert {
    pub id: Uuid,
    pub metric_name: String,
    /// "warning" or "critical"
    pub threshold_type: String,
    pub value: f64,
    pub acknowledged_at: Option<DateTime<Utc>>,
    pub acknowledged_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

// ==================== Support Access Models ====================

/// Support access request status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "VARCHAR", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum SupportAccessStatus {
    Pending,
    Approved,
    Denied,
    Expired,
    Revoked,
}

impl SupportAccessStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Approved => "approved",
            Self::Denied => "denied",
            Self::Expired => "expired",
            Self::Revoked => "revoked",
        }
    }
}

/// Support access request.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct SupportAccessRequest {
    pub id: Uuid,
    pub support_user_id: Uuid,
    pub org_id: Uuid,
    pub reason: String,
    pub status: String,
    pub approved_by: Option<Uuid>,
    pub approved_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Support access action log.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct SupportAccessLog {
    pub id: Uuid,
    pub request_id: Uuid,
    pub action: String,
    pub resource_type: Option<String>,
    pub resource_id: Option<Uuid>,
    pub details: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

// ==================== Onboarding Models ====================

/// Onboarding tour definition.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct OnboardingTour {
    pub id: Uuid,
    /// Unique key for the tour (e.g., "manager_intro")
    pub key: String,
    pub name: String,
    pub description: Option<String>,
    /// Target roles as JSON array (e.g., ["manager", "org_admin"])
    pub target_roles: serde_json::Value,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Step placement options.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum StepPlacement {
    Top,
    Bottom,
    Left,
    Right,
    Center,
}

impl StepPlacement {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Top => "top",
            Self::Bottom => "bottom",
            Self::Left => "left",
            Self::Right => "right",
            Self::Center => "center",
        }
    }
}

/// Onboarding tour step.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct OnboardingStep {
    pub id: Uuid,
    pub tour_id: Uuid,
    pub step_order: i32,
    pub title: String,
    pub content: String,
    /// CSS selector for the target element
    pub target_selector: Option<String>,
    pub placement: String,
    /// Action type: "next", "skip", "complete"
    pub action_type: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// User's progress through an onboarding tour.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct UserOnboardingProgress {
    pub id: Uuid,
    pub user_id: Uuid,
    pub tour_id: Uuid,
    pub current_step: i32,
    /// Completed step IDs as JSON array
    pub completed_steps: serde_json::Value,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

// ==================== Help Article Models ====================

/// Help article entity.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct HelpArticle {
    pub id: Uuid,
    /// Unique key for the article (e.g., "faults-overview")
    pub key: String,
    pub title: String,
    /// Content in Markdown format
    pub content: String,
    /// Route pattern for contextual matching (e.g., "/faults/*")
    pub route_pattern: Option<String>,
    /// Feature key for component-level help
    pub feature_key: Option<String>,
    /// Tags as JSON array
    pub tags: serde_json::Value,
    pub is_published: bool,
    pub version: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Help article revision for history tracking.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct HelpArticleRevision {
    pub id: Uuid,
    pub article_id: Uuid,
    pub title: String,
    pub content: String,
    pub published_by: Uuid,
    pub published_at: DateTime<Utc>,
}

/// Request to create a help article.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateHelpArticleRequest {
    pub key: String,
    pub title: String,
    pub content: String,
    pub route_pattern: Option<String>,
    pub feature_key: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}
