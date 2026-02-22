//! API Ecosystem Expansion models (Epic 150).
//!
//! Models for integration marketplace, connector framework, webhooks, and developer portal.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

// ============================================
// Story 150.1: Integration Marketplace
// ============================================

/// Integration category.
pub mod integration_category {
    pub const ACCOUNTING: &str = "accounting";
    pub const CRM: &str = "crm";
    pub const CALENDAR: &str = "calendar";
    pub const COMMUNICATION: &str = "communication";
    pub const PAYMENT: &str = "payment";
    pub const PROPERTY_PORTAL: &str = "property_portal";
    pub const IOT: &str = "iot";
    pub const ANALYTICS: &str = "analytics";
    pub const DOCUMENT_MANAGEMENT: &str = "document_management";
    pub const OTHER: &str = "other";
}

/// Integration status.
pub mod marketplace_integration_status {
    pub const AVAILABLE: &str = "available";
    pub const COMING_SOON: &str = "coming_soon";
    pub const DEPRECATED: &str = "deprecated";
    pub const MAINTENANCE: &str = "maintenance";
}

/// Installation status.
pub mod installation_status {
    pub const INSTALLED: &str = "installed";
    pub const PENDING: &str = "pending";
    pub const FAILED: &str = "failed";
    pub const UNINSTALLED: &str = "uninstalled";
}

/// Marketplace integration definition.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct MarketplaceIntegration {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub description: String,
    pub long_description: Option<String>,
    pub category: String,
    pub icon_url: Option<String>,
    pub banner_url: Option<String>,
    pub vendor_name: String,
    pub vendor_url: Option<String>,
    pub documentation_url: Option<String>,
    pub support_url: Option<String>,
    pub version: String,
    pub status: String,
    pub features: Option<serde_json::Value>,
    pub requirements: Option<serde_json::Value>,
    pub pricing_info: Option<serde_json::Value>,
    pub rating_average: Option<f64>,
    pub rating_count: i32,
    pub install_count: i32,
    pub is_featured: bool,
    pub is_premium: bool,
    pub required_scopes: Option<Vec<String>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Integration summary for list views.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct MarketplaceIntegrationSummary {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub icon_url: Option<String>,
    pub vendor_name: String,
    pub status: String,
    pub rating_average: Option<f64>,
    pub rating_count: i32,
    pub install_count: i32,
    pub is_featured: bool,
    pub is_premium: bool,
}

/// Create marketplace integration request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateMarketplaceIntegration {
    pub slug: String,
    pub name: String,
    pub description: String,
    pub long_description: Option<String>,
    pub category: String,
    pub icon_url: Option<String>,
    pub banner_url: Option<String>,
    pub vendor_name: String,
    pub vendor_url: Option<String>,
    pub documentation_url: Option<String>,
    pub support_url: Option<String>,
    pub version: String,
    pub features: Option<serde_json::Value>,
    pub requirements: Option<serde_json::Value>,
    pub pricing_info: Option<serde_json::Value>,
    pub is_premium: Option<bool>,
    pub required_scopes: Option<Vec<String>>,
}

/// Update marketplace integration request.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct UpdateMarketplaceIntegration {
    pub name: Option<String>,
    pub description: Option<String>,
    pub long_description: Option<String>,
    pub category: Option<String>,
    pub icon_url: Option<String>,
    pub banner_url: Option<String>,
    pub vendor_url: Option<String>,
    pub documentation_url: Option<String>,
    pub support_url: Option<String>,
    pub version: Option<String>,
    pub status: Option<String>,
    pub features: Option<serde_json::Value>,
    pub requirements: Option<serde_json::Value>,
    pub pricing_info: Option<serde_json::Value>,
    pub is_featured: Option<bool>,
    pub is_premium: Option<bool>,
    pub required_scopes: Option<Vec<String>>,
}

/// Marketplace integration query parameters.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema, IntoParams)]
pub struct MarketplaceIntegrationQuery {
    pub category: Option<String>,
    pub status: Option<String>,
    pub search: Option<String>,
    pub featured_only: Option<bool>,
    pub premium_only: Option<bool>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

/// Organization integration installation.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct OrganizationIntegration {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub integration_id: Uuid,
    pub status: String,
    pub configuration: Option<serde_json::Value>,
    pub credentials_encrypted: Option<String>,
    pub enabled: bool,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub installed_by: Uuid,
    pub installed_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Install integration request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InstallIntegration {
    pub integration_id: Uuid,
    pub configuration: Option<serde_json::Value>,
    pub credentials: Option<serde_json::Value>,
}

/// Update organization integration request.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct UpdateOrganizationIntegration {
    pub configuration: Option<serde_json::Value>,
    pub credentials: Option<serde_json::Value>,
    pub enabled: Option<bool>,
}

/// Integration rating.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct IntegrationRating {
    pub id: Uuid,
    pub integration_id: Uuid,
    pub organization_id: Uuid,
    pub user_id: Uuid,
    pub rating: i32,
    pub review: Option<String>,
    pub helpful_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create integration rating request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateIntegrationRating {
    pub rating: i32,
    pub review: Option<String>,
}

/// Integration rating with user info.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct IntegrationRatingWithUser {
    pub id: Uuid,
    pub integration_id: Uuid,
    pub user_name: String,
    pub rating: i32,
    pub review: Option<String>,
    pub helpful_count: i32,
    pub created_at: DateTime<Utc>,
}

/// Category count for filtering.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct IntegrationCategoryCount {
    pub category: String,
    pub count: i64,
}

// ============================================
// Story 150.2: Pre-Built Connector Framework
// ============================================

/// Connector authentication type.
pub mod connector_auth_type {
    pub const OAUTH2: &str = "oauth2";
    pub const API_KEY: &str = "api_key";
    pub const BASIC: &str = "basic";
    pub const BEARER_TOKEN: &str = "bearer_token";
    pub const CUSTOM: &str = "custom";
}

/// Connector status.
pub mod connector_status {
    pub const ACTIVE: &str = "active";
    pub const INACTIVE: &str = "inactive";
    pub const ERROR: &str = "error";
    pub const RATE_LIMITED: &str = "rate_limited";
}

/// Pre-built connector definition.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct Connector {
    pub id: Uuid,
    pub integration_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub auth_type: String,
    pub auth_config: Option<serde_json::Value>,
    pub base_url: String,
    pub rate_limit_requests: Option<i32>,
    pub rate_limit_window_seconds: Option<i32>,
    pub retry_max_attempts: i32,
    pub retry_initial_delay_ms: i32,
    pub retry_max_delay_ms: i32,
    pub timeout_ms: i32,
    pub headers: Option<serde_json::Value>,
    pub supported_actions: Vec<String>,
    pub error_mapping: Option<serde_json::Value>,
    pub data_transformations: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create connector request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateConnector {
    pub integration_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub auth_type: String,
    pub auth_config: Option<serde_json::Value>,
    pub base_url: String,
    pub rate_limit_requests: Option<i32>,
    pub rate_limit_window_seconds: Option<i32>,
    pub retry_max_attempts: Option<i32>,
    pub retry_initial_delay_ms: Option<i32>,
    pub retry_max_delay_ms: Option<i32>,
    pub timeout_ms: Option<i32>,
    pub headers: Option<serde_json::Value>,
    pub supported_actions: Vec<String>,
    pub error_mapping: Option<serde_json::Value>,
    pub data_transformations: Option<serde_json::Value>,
}

/// Update connector request.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct UpdateConnector {
    pub name: Option<String>,
    pub description: Option<String>,
    pub auth_config: Option<serde_json::Value>,
    pub base_url: Option<String>,
    pub rate_limit_requests: Option<i32>,
    pub rate_limit_window_seconds: Option<i32>,
    pub retry_max_attempts: Option<i32>,
    pub retry_initial_delay_ms: Option<i32>,
    pub retry_max_delay_ms: Option<i32>,
    pub timeout_ms: Option<i32>,
    pub headers: Option<serde_json::Value>,
    pub supported_actions: Option<Vec<String>>,
    pub error_mapping: Option<serde_json::Value>,
    pub data_transformations: Option<serde_json::Value>,
}

/// Connector action for the SDK.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct ConnectorAction {
    pub id: Uuid,
    pub connector_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub http_method: String,
    pub endpoint_path: String,
    pub request_schema: Option<serde_json::Value>,
    pub response_schema: Option<serde_json::Value>,
    pub request_transformations: Option<serde_json::Value>,
    pub response_transformations: Option<serde_json::Value>,
    pub pagination_config: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create connector action request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateConnectorAction {
    pub connector_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub http_method: String,
    pub endpoint_path: String,
    pub request_schema: Option<serde_json::Value>,
    pub response_schema: Option<serde_json::Value>,
    pub request_transformations: Option<serde_json::Value>,
    pub response_transformations: Option<serde_json::Value>,
    pub pagination_config: Option<serde_json::Value>,
}

/// Connector execution log.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct ConnectorExecutionLog {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub connector_id: Uuid,
    pub action_name: String,
    pub status: String,
    pub request_payload: Option<serde_json::Value>,
    pub response_payload: Option<serde_json::Value>,
    pub error_message: Option<String>,
    pub error_code: Option<String>,
    pub duration_ms: i32,
    pub retry_count: i32,
    pub rate_limited: bool,
    pub created_at: DateTime<Utc>,
}

/// Connector execution log query.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema, IntoParams)]
pub struct ConnectorExecutionQuery {
    pub connector_id: Option<Uuid>,
    pub action_name: Option<String>,
    pub status: Option<String>,
    pub from_date: Option<DateTime<Utc>>,
    pub to_date: Option<DateTime<Utc>>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

// ============================================
// Story 150.3: Webhook Management (Enhanced)
// ============================================

/// Webhook authentication type.
pub mod webhook_auth_type {
    pub const HMAC_SHA256: &str = "hmac_sha256";
    pub const HMAC_SHA512: &str = "hmac_sha512";
    pub const BEARER_TOKEN: &str = "bearer_token";
    pub const BASIC_AUTH: &str = "basic_auth";
    pub const NONE: &str = "none";
}

/// Enhanced webhook subscription with more configuration options.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct EnhancedWebhookSubscription {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub url: String,
    pub auth_type: String,
    pub auth_config: Option<serde_json::Value>,
    pub events: Vec<String>,
    pub filters: Option<serde_json::Value>,
    pub payload_template: Option<serde_json::Value>,
    pub status: String,
    pub headers: Option<serde_json::Value>,
    pub retry_policy: Option<serde_json::Value>,
    pub rate_limit_requests: Option<i32>,
    pub rate_limit_window_seconds: Option<i32>,
    pub timeout_ms: i32,
    pub verify_ssl: bool,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create enhanced webhook subscription request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateEnhancedWebhookSubscription {
    pub name: String,
    pub description: Option<String>,
    pub url: String,
    pub auth_type: String,
    pub auth_config: Option<serde_json::Value>,
    pub events: Vec<String>,
    pub filters: Option<serde_json::Value>,
    pub payload_template: Option<serde_json::Value>,
    pub headers: Option<serde_json::Value>,
    pub retry_policy: Option<WebhookRetryPolicyConfig>,
    pub rate_limit_requests: Option<i32>,
    pub rate_limit_window_seconds: Option<i32>,
    pub timeout_ms: Option<i32>,
    pub verify_ssl: Option<bool>,
}

/// Update enhanced webhook subscription request.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct UpdateEnhancedWebhookSubscription {
    pub name: Option<String>,
    pub description: Option<String>,
    pub url: Option<String>,
    pub auth_type: Option<String>,
    pub auth_config: Option<serde_json::Value>,
    pub events: Option<Vec<String>>,
    pub filters: Option<serde_json::Value>,
    pub payload_template: Option<serde_json::Value>,
    pub headers: Option<serde_json::Value>,
    pub status: Option<String>,
    pub retry_policy: Option<WebhookRetryPolicyConfig>,
    pub rate_limit_requests: Option<i32>,
    pub rate_limit_window_seconds: Option<i32>,
    pub timeout_ms: Option<i32>,
    pub verify_ssl: Option<bool>,
}

/// Webhook retry policy configuration.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WebhookRetryPolicyConfig {
    pub max_retries: i32,
    pub initial_delay_ms: i32,
    pub max_delay_ms: i32,
    pub exponential_backoff: bool,
    pub retry_on_status_codes: Vec<i32>,
}

/// Webhook event types for the ecosystem.
pub mod ecosystem_webhook_event {
    // Integration events
    pub const INTEGRATION_INSTALLED: &str = "integration.installed";
    pub const INTEGRATION_UNINSTALLED: &str = "integration.uninstalled";
    pub const INTEGRATION_CONFIGURED: &str = "integration.configured";
    pub const INTEGRATION_ERROR: &str = "integration.error";
    pub const INTEGRATION_SYNCED: &str = "integration.synced";

    // Connector events
    pub const CONNECTOR_EXECUTED: &str = "connector.executed";
    pub const CONNECTOR_FAILED: &str = "connector.failed";
    pub const CONNECTOR_RATE_LIMITED: &str = "connector.rate_limited";

    // Data sync events
    pub const DATA_IMPORTED: &str = "data.imported";
    pub const DATA_EXPORTED: &str = "data.exported";
    pub const DATA_SYNC_STARTED: &str = "data.sync_started";
    pub const DATA_SYNC_COMPLETED: &str = "data.sync_completed";
    pub const DATA_SYNC_FAILED: &str = "data.sync_failed";

    // API events
    pub const API_KEY_CREATED: &str = "api_key.created";
    pub const API_KEY_ROTATED: &str = "api_key.rotated";
    pub const API_KEY_REVOKED: &str = "api_key.revoked";
    pub const API_RATE_LIMIT_EXCEEDED: &str = "api.rate_limit_exceeded";
}

/// Enhanced webhook delivery log.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct EnhancedWebhookDeliveryLog {
    pub id: Uuid,
    pub subscription_id: Uuid,
    pub event_type: String,
    pub event_id: Uuid,
    pub payload: serde_json::Value,
    pub transformed_payload: Option<serde_json::Value>,
    pub status: String,
    pub attempts: i32,
    pub last_attempt_at: Option<DateTime<Utc>>,
    pub next_retry_at: Option<DateTime<Utc>>,
    pub request_headers: Option<serde_json::Value>,
    pub response_status: Option<i32>,
    pub response_headers: Option<serde_json::Value>,
    pub response_body: Option<String>,
    pub error_message: Option<String>,
    pub error_code: Option<String>,
    pub duration_ms: Option<i32>,
    pub ip_address: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Webhook delivery statistics.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EnhancedWebhookStatistics {
    pub subscription_id: Uuid,
    pub total_deliveries: i64,
    pub successful_deliveries: i64,
    pub failed_deliveries: i64,
    pub pending_deliveries: i64,
    pub retrying_deliveries: i64,
    pub average_response_time_ms: Option<f64>,
    pub success_rate: f64,
    pub last_24h_deliveries: i64,
    pub last_24h_failures: i64,
    pub events_by_type: serde_json::Value,
}

// ============================================
// Story 150.4: Popular Integrations (Batch 1)
// ============================================

/// QuickBooks connection configuration.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct QuickBooksConfig {
    pub realm_id: String,
    pub company_name: Option<String>,
    pub sync_invoices: bool,
    pub sync_payments: bool,
    pub sync_customers: bool,
    pub default_income_account: Option<String>,
    pub default_expense_account: Option<String>,
}

/// Xero connection configuration.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct XeroConfig {
    pub tenant_id: String,
    pub organization_name: Option<String>,
    pub sync_invoices: bool,
    pub sync_payments: bool,
    pub sync_contacts: bool,
    pub default_account_code: Option<String>,
    pub tracking_categories: Option<Vec<String>>,
}

/// Salesforce connection configuration.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SalesforceConfig {
    pub instance_url: String,
    pub sync_contacts: bool,
    pub sync_leads: bool,
    pub sync_opportunities: bool,
    pub custom_object_mappings: Option<serde_json::Value>,
    pub field_mappings: Option<serde_json::Value>,
}

/// HubSpot connection configuration.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HubSpotConfig {
    pub portal_id: String,
    pub sync_contacts: bool,
    pub sync_companies: bool,
    pub sync_deals: bool,
    pub pipeline_id: Option<String>,
    pub property_mappings: Option<serde_json::Value>,
}

/// Google Calendar connection configuration.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GoogleCalendarConfig {
    pub calendar_id: String,
    pub calendar_name: Option<String>,
    pub sync_direction: String,
    pub event_prefix: Option<String>,
    pub include_private: bool,
}

/// Outlook Calendar connection configuration.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OutlookCalendarConfig {
    pub calendar_id: String,
    pub calendar_name: Option<String>,
    pub sync_direction: String,
    pub categories: Option<Vec<String>>,
    pub include_recurring: bool,
}

/// Slack connection configuration.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SlackConfig {
    pub team_id: String,
    pub team_name: Option<String>,
    pub default_channel: Option<String>,
    pub notification_channels: Option<serde_json::Value>,
    pub mention_users: bool,
    pub include_links: bool,
}

/// Microsoft Teams connection configuration.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TeamsConfig {
    pub tenant_id: String,
    pub team_id: Option<String>,
    pub team_name: Option<String>,
    pub default_channel: Option<String>,
    pub notification_channels: Option<serde_json::Value>,
    pub adaptive_cards: bool,
}

/// Pre-built integration type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PreBuiltIntegrationType {
    QuickBooks,
    Xero,
    Salesforce,
    HubSpot,
    GoogleCalendar,
    OutlookCalendar,
    Slack,
    MicrosoftTeams,
}

impl PreBuiltIntegrationType {
    pub fn as_str(&self) -> &'static str {
        match self {
            PreBuiltIntegrationType::QuickBooks => "quickbooks",
            PreBuiltIntegrationType::Xero => "xero",
            PreBuiltIntegrationType::Salesforce => "salesforce",
            PreBuiltIntegrationType::HubSpot => "hubspot",
            PreBuiltIntegrationType::GoogleCalendar => "google_calendar",
            PreBuiltIntegrationType::OutlookCalendar => "outlook_calendar",
            PreBuiltIntegrationType::Slack => "slack",
            PreBuiltIntegrationType::MicrosoftTeams => "microsoft_teams",
        }
    }

    pub fn category(&self) -> &'static str {
        match self {
            PreBuiltIntegrationType::QuickBooks | PreBuiltIntegrationType::Xero => {
                integration_category::ACCOUNTING
            }
            PreBuiltIntegrationType::Salesforce | PreBuiltIntegrationType::HubSpot => {
                integration_category::CRM
            }
            PreBuiltIntegrationType::GoogleCalendar | PreBuiltIntegrationType::OutlookCalendar => {
                integration_category::CALENDAR
            }
            PreBuiltIntegrationType::Slack | PreBuiltIntegrationType::MicrosoftTeams => {
                integration_category::COMMUNICATION
            }
        }
    }
}

/// Pre-built integration connection.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct PreBuiltIntegrationConnection {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub integration_type: String,
    pub status: String,
    pub configuration: serde_json::Value,
    pub access_token_encrypted: Option<String>,
    pub refresh_token_encrypted: Option<String>,
    pub token_expires_at: Option<DateTime<Utc>>,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub sync_enabled: bool,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create pre-built integration connection request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreatePreBuiltIntegrationConnection {
    pub integration_type: String,
    pub configuration: serde_json::Value,
    pub auth_code: Option<String>,
}

/// Update pre-built integration connection request.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct UpdatePreBuiltIntegrationConnection {
    pub configuration: Option<serde_json::Value>,
    pub sync_enabled: Option<bool>,
}

/// Sync pre-built integration request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SyncPreBuiltIntegrationRequest {
    pub full_sync: Option<bool>,
    pub entity_types: Option<Vec<String>>,
    pub from_date: Option<DateTime<Utc>>,
    pub to_date: Option<DateTime<Utc>>,
}

/// Pre-built integration sync result.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PreBuiltIntegrationSyncResult {
    pub integration_type: String,
    pub records_created: i32,
    pub records_updated: i32,
    pub records_deleted: i32,
    pub errors: Vec<String>,
    pub synced_at: DateTime<Utc>,
    pub duration_ms: i32,
}

// ============================================
// Story 150.5: API Developer Portal
// ============================================

/// Developer registration.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct DeveloperRegistration {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub email: String,
    pub company_name: Option<String>,
    pub website: Option<String>,
    pub use_case: Option<String>,
    pub status: String,
    pub approved_at: Option<DateTime<Utc>>,
    pub approved_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Developer registration status.
pub mod developer_status {
    pub const PENDING: &str = "pending";
    pub const APPROVED: &str = "approved";
    pub const REJECTED: &str = "rejected";
    pub const SUSPENDED: &str = "suspended";
}

/// Create developer registration request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateDeveloperRegistration {
    pub email: String,
    pub company_name: Option<String>,
    pub website: Option<String>,
    pub use_case: Option<String>,
}

/// Review developer registration request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReviewDeveloperRegistration {
    pub status: String,
    pub notes: Option<String>,
}

/// Developer API key for the portal.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct DeveloperApiKey {
    pub id: Uuid,
    pub developer_id: Uuid,
    pub name: String,
    pub key_prefix: String,
    pub key_hash: String,
    pub scopes: Vec<String>,
    pub rate_limit_tier: String,
    pub is_sandbox: bool,
    pub status: String,
    pub last_used_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}

/// Developer API key display (for listing).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DeveloperApiKeyDisplay {
    pub id: Uuid,
    pub name: String,
    pub key_prefix: String,
    pub scopes: Vec<String>,
    pub rate_limit_tier: String,
    pub is_sandbox: bool,
    pub status: String,
    pub last_used_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Create developer API key request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateDeveloperApiKey {
    pub name: String,
    pub scopes: Vec<String>,
    pub is_sandbox: Option<bool>,
    pub expires_in_days: Option<i32>,
}

/// Create developer API key response (returns full key once).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateDeveloperApiKeyResponse {
    pub id: Uuid,
    pub name: String,
    pub key: String,
    pub scopes: Vec<String>,
    pub rate_limit_tier: String,
    pub is_sandbox: bool,
    pub expires_at: Option<DateTime<Utc>>,
}

/// API documentation article.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct ApiDocumentation {
    pub id: Uuid,
    pub slug: String,
    pub title: String,
    pub content: String,
    pub category: String,
    pub order_index: i32,
    pub is_published: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// API documentation category.
pub mod api_doc_category {
    pub const GETTING_STARTED: &str = "getting_started";
    pub const AUTHENTICATION: &str = "authentication";
    pub const ENDPOINTS: &str = "endpoints";
    pub const WEBHOOKS: &str = "webhooks";
    pub const RATE_LIMITS: &str = "rate_limits";
    pub const CODE_SAMPLES: &str = "code_samples";
    pub const BEST_PRACTICES: &str = "best_practices";
    pub const CHANGELOG: &str = "changelog";
}

/// Create API documentation request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateApiDocumentation {
    pub slug: String,
    pub title: String,
    pub content: String,
    pub category: String,
    pub order_index: Option<i32>,
    pub is_published: Option<bool>,
}

/// Update API documentation request.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct UpdateApiDocumentation {
    pub title: Option<String>,
    pub content: Option<String>,
    pub category: Option<String>,
    pub order_index: Option<i32>,
    pub is_published: Option<bool>,
}

/// API code sample.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct ApiCodeSample {
    pub id: Uuid,
    pub endpoint_path: String,
    pub http_method: String,
    pub language: String,
    pub code: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Code sample language.
pub mod code_sample_language {
    pub const CURL: &str = "curl";
    pub const JAVASCRIPT: &str = "javascript";
    pub const TYPESCRIPT: &str = "typescript";
    pub const PYTHON: &str = "python";
    pub const RUBY: &str = "ruby";
    pub const PHP: &str = "php";
    pub const GO: &str = "go";
    pub const JAVA: &str = "java";
    pub const CSHARP: &str = "csharp";
    pub const RUST: &str = "rust";
}

/// Create API code sample request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateApiCodeSample {
    pub endpoint_path: String,
    pub http_method: String,
    pub language: String,
    pub code: String,
    pub description: Option<String>,
}

/// Sandbox environment configuration.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct SandboxConfig {
    pub id: Uuid,
    pub developer_id: Uuid,
    pub name: String,
    pub configuration: serde_json::Value,
    pub test_data_seeded: bool,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create sandbox environment request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateSandboxConfig {
    pub name: String,
    pub seed_test_data: Option<bool>,
    pub expires_in_days: Option<i32>,
}

/// Sandbox test request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SandboxTestRequestPayload {
    pub endpoint: String,
    pub method: String,
    pub headers: Option<serde_json::Value>,
    pub body: Option<serde_json::Value>,
}

/// Sandbox test response.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SandboxTestResponsePayload {
    pub status_code: i32,
    pub headers: serde_json::Value,
    pub body: serde_json::Value,
    pub duration_ms: i32,
}

/// Developer portal statistics.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DeveloperPortalStatistics {
    pub total_developers: i64,
    pub active_developers: i64,
    pub pending_registrations: i64,
    pub total_api_keys: i64,
    pub sandbox_api_keys: i64,
    pub production_api_keys: i64,
    pub api_calls_today: i64,
    pub api_calls_this_month: i64,
    pub top_endpoints: Vec<EndpointUsageStats>,
}

/// Endpoint usage statistics.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EndpointUsageStats {
    pub endpoint: String,
    pub method: String,
    pub call_count: i64,
    pub avg_response_time_ms: f64,
    pub error_rate: f64,
}

/// Developer usage statistics.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DeveloperUsageStats {
    pub developer_id: Uuid,
    pub api_calls_today: i64,
    pub api_calls_this_month: i64,
    pub rate_limit_exceeded_count: i64,
    pub error_count: i64,
    pub avg_response_time_ms: f64,
    pub most_used_endpoints: Vec<EndpointUsageStats>,
}

// ============================================
// Dashboard & Statistics
// ============================================

/// API ecosystem dashboard data.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApiEcosystemDashboard {
    pub installed_integrations: i32,
    pub active_integrations: i32,
    pub pending_sync: i32,
    pub failed_integrations: i32,
    pub webhook_subscriptions: i32,
    pub webhooks_delivered_today: i64,
    pub webhook_success_rate: f64,
    pub connector_executions_today: i64,
    pub connector_success_rate: f64,
    pub recent_activity: Vec<IntegrationActivityLog>,
}

/// Integration activity log entry.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct IntegrationActivityLog {
    pub id: Uuid,
    pub integration_name: String,
    pub event_type: String,
    pub description: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

/// API ecosystem statistics.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApiEcosystemStatistics {
    pub total_integrations: i64,
    pub integrations_by_category: serde_json::Value,
    pub active_connections: i64,
    pub sync_operations_today: i64,
    pub sync_operations_this_month: i64,
    pub data_transferred_bytes: i64,
    pub average_sync_duration_ms: f64,
    pub error_rate: f64,
}
