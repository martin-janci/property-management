//! External Integrations models (Epic 61).
//!
//! Models for calendar sync, accounting exports, e-signatures, video conferencing, and webhooks.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::fmt;
use std::str::FromStr;
use utoipa::ToSchema;
use uuid::Uuid;

// ============================================
// Provider Enums with Validation
// ============================================

/// Calendar provider enum with validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CalendarProvider {
    Google,
    Outlook,
    Apple,
    Caldav,
}

impl fmt::Display for CalendarProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CalendarProvider::Google => write!(f, "google"),
            CalendarProvider::Outlook => write!(f, "outlook"),
            CalendarProvider::Apple => write!(f, "apple"),
            CalendarProvider::Caldav => write!(f, "caldav"),
        }
    }
}

impl FromStr for CalendarProvider {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "google" => Ok(CalendarProvider::Google),
            "outlook" => Ok(CalendarProvider::Outlook),
            "apple" => Ok(CalendarProvider::Apple),
            "caldav" => Ok(CalendarProvider::Caldav),
            _ => Err(format!(
                "Invalid calendar provider '{}'. Valid values: google, outlook, apple, caldav",
                s
            )),
        }
    }
}

impl CalendarProvider {
    /// Get all valid calendar provider values.
    pub fn all_values() -> &'static [&'static str] {
        &["google", "outlook", "apple", "caldav"]
    }
}

/// Accounting system enum with validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AccountingSystem {
    Pohoda,
    MoneyS3,
    Quickbooks,
    Xero,
}

impl fmt::Display for AccountingSystem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AccountingSystem::Pohoda => write!(f, "pohoda"),
            AccountingSystem::MoneyS3 => write!(f, "money_s3"),
            AccountingSystem::Quickbooks => write!(f, "quickbooks"),
            AccountingSystem::Xero => write!(f, "xero"),
        }
    }
}

impl FromStr for AccountingSystem {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pohoda" => Ok(AccountingSystem::Pohoda),
            "money_s3" => Ok(AccountingSystem::MoneyS3),
            "quickbooks" => Ok(AccountingSystem::Quickbooks),
            "xero" => Ok(AccountingSystem::Xero),
            _ => Err(format!(
                "Invalid accounting system '{}'. Valid values: pohoda, money_s3, quickbooks, xero",
                s
            )),
        }
    }
}

impl AccountingSystem {
    /// Get all valid accounting system values.
    pub fn all_values() -> &'static [&'static str] {
        &["pohoda", "money_s3", "quickbooks", "xero"]
    }
}

/// E-signature provider enum with validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ESignatureProvider {
    Docusign,
    AdobeSign,
    Hellosign,
    Internal,
}

impl fmt::Display for ESignatureProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ESignatureProvider::Docusign => write!(f, "docusign"),
            ESignatureProvider::AdobeSign => write!(f, "adobe_sign"),
            ESignatureProvider::Hellosign => write!(f, "hellosign"),
            ESignatureProvider::Internal => write!(f, "internal"),
        }
    }
}

impl FromStr for ESignatureProvider {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "docusign" => Ok(ESignatureProvider::Docusign),
            "adobe_sign" => Ok(ESignatureProvider::AdobeSign),
            "hellosign" => Ok(ESignatureProvider::Hellosign),
            "internal" => Ok(ESignatureProvider::Internal),
            _ => Err(format!(
                "Invalid e-signature provider '{}'. Valid values: docusign, adobe_sign, hellosign, internal",
                s
            )),
        }
    }
}

impl ESignatureProvider {
    /// Get all valid e-signature provider values.
    pub fn all_values() -> &'static [&'static str] {
        &["docusign", "adobe_sign", "hellosign", "internal"]
    }
}

/// Video conferencing provider enum with validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum VideoProvider {
    Zoom,
    Teams,
    GoogleMeet,
    Webex,
}

impl fmt::Display for VideoProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VideoProvider::Zoom => write!(f, "zoom"),
            VideoProvider::Teams => write!(f, "teams"),
            VideoProvider::GoogleMeet => write!(f, "google_meet"),
            VideoProvider::Webex => write!(f, "webex"),
        }
    }
}

impl FromStr for VideoProvider {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "zoom" => Ok(VideoProvider::Zoom),
            "teams" => Ok(VideoProvider::Teams),
            "google_meet" => Ok(VideoProvider::GoogleMeet),
            "webex" => Ok(VideoProvider::Webex),
            _ => Err(format!(
                "Invalid video provider '{}'. Valid values: zoom, teams, google_meet, webex",
                s
            )),
        }
    }
}

impl VideoProvider {
    /// Get all valid video provider values.
    pub fn all_values() -> &'static [&'static str] {
        &["zoom", "teams", "google_meet", "webex"]
    }
}

// ============================================
// Story 61.1: Calendar Integration
// ============================================

/// Calendar provider type constants (deprecated, use CalendarProvider enum).
pub mod calendar_provider {
    pub const GOOGLE: &str = "google";
    pub const OUTLOOK: &str = "outlook";
    pub const APPLE: &str = "apple";
    pub const CALDAV: &str = "caldav";
}

/// Calendar sync status.
pub mod calendar_sync_status {
    pub const ACTIVE: &str = "active";
    pub const PAUSED: &str = "paused";
    pub const ERROR: &str = "error";
    pub const DISCONNECTED: &str = "disconnected";
}

/// Calendar connection entity.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct CalendarConnection {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub user_id: Uuid,
    pub provider: String,
    pub provider_account_id: Option<String>,
    pub calendar_id: Option<String>,
    pub calendar_name: Option<String>,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub token_expires_at: Option<DateTime<Utc>>,
    pub sync_status: String,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub sync_direction: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create calendar connection request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateCalendarConnection {
    pub provider: String,
    pub auth_code: Option<String>,
    pub calendar_id: Option<String>,
    pub sync_direction: Option<String>,
}

/// Update calendar connection request.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct UpdateCalendarConnection {
    pub calendar_id: Option<String>,
    pub sync_direction: Option<String>,
    pub sync_status: Option<String>,
}

/// Calendar event entity.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct CalendarEvent {
    pub id: Uuid,
    pub connection_id: Uuid,
    pub external_event_id: Option<String>,
    pub source_type: String,
    pub source_id: Option<Uuid>,
    pub title: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub all_day: bool,
    pub recurrence_rule: Option<String>,
    pub attendees: Option<serde_json::Value>,
    pub last_synced_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create calendar event request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateCalendarEvent {
    pub connection_id: Uuid,
    pub external_event_id: Option<String>,
    pub source_type: String,
    pub source_id: Option<Uuid>,
    pub title: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub all_day: Option<bool>,
    pub recurrence_rule: Option<String>,
    pub attendees: Option<serde_json::Value>,
}

/// Sync calendar request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SyncCalendarRequest {
    pub full_sync: Option<bool>,
    pub date_range_start: Option<DateTime<Utc>>,
    pub date_range_end: Option<DateTime<Utc>>,
}

/// Calendar sync result.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CalendarSyncResult {
    pub events_created: i32,
    pub events_updated: i32,
    pub events_deleted: i32,
    pub errors: Vec<String>,
    pub synced_at: DateTime<Utc>,
}

// ============================================
// Story 61.2: Accounting System Export
// ============================================

/// Accounting system type constants (deprecated, use AccountingSystem enum).
pub mod accounting_system {
    pub const POHODA: &str = "pohoda";
    pub const MONEY_S3: &str = "money_s3";
    pub const QUICKBOOKS: &str = "quickbooks";
    pub const XERO: &str = "xero";
}

/// Export status.
pub mod export_status {
    pub const PENDING: &str = "pending";
    pub const PROCESSING: &str = "processing";
    pub const COMPLETED: &str = "completed";
    pub const FAILED: &str = "failed";
}

/// Accounting export entity.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct AccountingExport {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub system_type: String,
    pub export_type: String,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub status: String,
    pub file_path: Option<String>,
    pub file_size: Option<i64>,
    pub record_count: Option<i32>,
    pub error_message: Option<String>,
    pub exported_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Create accounting export request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateAccountingExport {
    pub system_type: String,
    pub export_type: String,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub include_attachments: Option<bool>,
    pub cost_center_mapping: Option<serde_json::Value>,
}

/// Accounting export settings.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct AccountingExportSettings {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub system_type: String,
    pub default_cost_center: Option<String>,
    pub account_mappings: Option<serde_json::Value>,
    pub vat_settings: Option<serde_json::Value>,
    pub auto_export_enabled: bool,
    pub auto_export_schedule: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Update accounting export settings request.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct UpdateAccountingExportSettings {
    pub default_cost_center: Option<String>,
    pub account_mappings: Option<serde_json::Value>,
    pub vat_settings: Option<serde_json::Value>,
    pub auto_export_enabled: Option<bool>,
    pub auto_export_schedule: Option<String>,
}

/// POHODA XML export data.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PohodaExportData {
    pub invoices: Vec<PohodaInvoice>,
    pub payments: Vec<PohodaPayment>,
}

/// POHODA invoice structure.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PohodaInvoice {
    pub number: String,
    pub date: NaiveDate,
    pub due_date: NaiveDate,
    pub partner_name: String,
    pub partner_ico: Option<String>,
    pub partner_dic: Option<String>,
    pub items: Vec<PohodaInvoiceItem>,
    pub total_without_vat: f64,
    pub total_vat: f64,
    pub total_with_vat: f64,
}

/// POHODA invoice item.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PohodaInvoiceItem {
    pub description: String,
    pub quantity: f64,
    pub unit_price: f64,
    pub vat_rate: f64,
    pub total: f64,
}

/// POHODA payment structure.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PohodaPayment {
    pub date: NaiveDate,
    pub amount: f64,
    pub invoice_number: Option<String>,
    pub payment_type: String,
}

// ============================================
// Story 61.3: E-Signature Integration
// ============================================

/// E-signature provider type constants (deprecated, use ESignatureProvider enum).
pub mod esignature_provider {
    pub const DOCUSIGN: &str = "docusign";
    pub const ADOBE_SIGN: &str = "adobe_sign";
    pub const HELLOSIGN: &str = "hellosign";
    pub const INTERNAL: &str = "internal";
}

/// E-signature workflow status.
pub mod esignature_status {
    pub const DRAFT: &str = "draft";
    pub const SENT: &str = "sent";
    pub const VIEWED: &str = "viewed";
    pub const SIGNED: &str = "signed";
    pub const COMPLETED: &str = "completed";
    pub const DECLINED: &str = "declined";
    pub const VOIDED: &str = "voided";
    pub const EXPIRED: &str = "expired";

    /// Terminal statuses — once a workflow reaches one of these it must not be
    /// mutated by a (possibly re-delivered) provider webhook event.
    pub const TERMINAL: [&str; 3] = [COMPLETED, VOIDED, DECLINED];

    /// Returns `true` when `status` is a terminal e-signature workflow status.
    ///
    /// Terminal statuses (`completed`, `voided`, `declined`) represent a final
    /// outcome of the signing process. Provider webhooks can be re-delivered,
    /// so any status transition that would overwrite a terminal state must be
    /// treated as a safe no-op for idempotency.
    pub fn is_terminal(status: &str) -> bool {
        TERMINAL.contains(&status)
    }
}

/// E-signature workflow entity.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct ESignatureWorkflow {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub document_id: Uuid,
    pub provider: String,
    pub external_envelope_id: Option<String>,
    pub title: String,
    pub message: Option<String>,
    pub status: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub reminder_enabled: bool,
    pub reminder_days: Option<i32>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// E-signature recipient entity.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct ESignatureRecipient {
    pub id: Uuid,
    pub workflow_id: Uuid,
    pub email: String,
    pub name: String,
    pub role: String,
    pub signing_order: i32,
    pub status: String,
    pub signed_at: Option<DateTime<Utc>>,
    pub declined_at: Option<DateTime<Utc>>,
    pub decline_reason: Option<String>,
    pub reminder_sent_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create e-signature workflow request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateESignatureWorkflow {
    pub document_id: Uuid,
    pub provider: Option<String>,
    pub title: String,
    pub message: Option<String>,
    pub recipients: Vec<CreateESignatureRecipient>,
    pub expires_in_days: Option<i32>,
    pub reminder_enabled: Option<bool>,
    pub reminder_days: Option<i32>,
}

/// Create e-signature recipient.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateESignatureRecipient {
    pub email: String,
    pub name: String,
    pub role: String,
    pub signing_order: Option<i32>,
}

/// E-signature workflow with recipients.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ESignatureWorkflowWithRecipients {
    #[serde(flatten)]
    pub workflow: ESignatureWorkflow,
    pub recipients: Vec<ESignatureRecipient>,
}

/// E-signature event (webhook).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ESignatureEvent {
    pub event_type: String,
    pub envelope_id: String,
    pub recipient_email: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub data: Option<serde_json::Value>,
}

// ============================================
// Story 61.4: Video Conferencing
// ============================================

/// Video conferencing provider type constants (deprecated, use VideoProvider enum).
pub mod video_provider {
    pub const ZOOM: &str = "zoom";
    pub const TEAMS: &str = "teams";
    pub const GOOGLE_MEET: &str = "google_meet";
    pub const WEBEX: &str = "webex";
}

/// Meeting status.
pub mod meeting_status {
    pub const SCHEDULED: &str = "scheduled";
    pub const STARTED: &str = "started";
    pub const ENDED: &str = "ended";
    pub const CANCELLED: &str = "cancelled";
}

/// Video conference connection entity.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct VideoConferenceConnection {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub user_id: Uuid,
    pub provider: String,
    pub provider_user_id: Option<String>,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub token_expires_at: Option<DateTime<Utc>>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create video conference connection request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateVideoConferenceConnection {
    pub provider: String,
    pub auth_code: String,
}

/// Video meeting entity.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct VideoMeeting {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub connection_id: Uuid,
    pub external_meeting_id: Option<String>,
    pub source_type: String,
    pub source_id: Option<Uuid>,
    pub title: String,
    pub description: Option<String>,
    pub start_time: DateTime<Utc>,
    pub duration_minutes: i32,
    pub timezone: Option<String>,
    pub join_url: Option<String>,
    pub host_url: Option<String>,
    pub password: Option<String>,
    pub status: String,
    pub recording_url: Option<String>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create video meeting request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateVideoMeeting {
    pub connection_id: Option<Uuid>,
    pub provider: Option<String>,
    pub source_type: String,
    pub source_id: Option<Uuid>,
    pub title: String,
    pub description: Option<String>,
    pub start_time: DateTime<Utc>,
    pub duration_minutes: i32,
    pub timezone: Option<String>,
    pub participants: Option<Vec<MeetingParticipant>>,
    pub settings: Option<MeetingSettings>,
}

/// Meeting participant.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MeetingParticipant {
    pub email: String,
    pub name: Option<String>,
    pub is_host: Option<bool>,
}

/// Meeting settings.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MeetingSettings {
    pub waiting_room: Option<bool>,
    pub mute_on_entry: Option<bool>,
    pub auto_recording: Option<bool>,
    pub allow_screen_share: Option<bool>,
}

/// Update video meeting request.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct UpdateVideoMeeting {
    pub title: Option<String>,
    pub description: Option<String>,
    pub start_time: Option<DateTime<Utc>>,
    pub duration_minutes: Option<i32>,
    pub status: Option<String>,
}

// ============================================
// Story 61.5: Webhook Notifications
// ============================================

/// Webhook status.
pub mod webhook_status {
    pub const ACTIVE: &str = "active";
    pub const PAUSED: &str = "paused";
    pub const DISABLED: &str = "disabled";
}

/// Delivery status.
pub mod delivery_status {
    pub const PENDING: &str = "pending";
    pub const DELIVERED: &str = "delivered";
    pub const FAILED: &str = "failed";
    pub const RETRYING: &str = "retrying";
}

/// Webhook subscription entity.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct WebhookSubscription {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub url: String,
    pub secret: Option<String>,
    pub events: Vec<String>,
    pub status: String,
    pub headers: Option<serde_json::Value>,
    pub retry_policy: Option<serde_json::Value>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create webhook subscription request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateWebhookSubscription {
    pub name: String,
    pub description: Option<String>,
    pub url: String,
    pub events: Vec<String>,
    pub secret: Option<String>,
    pub headers: Option<serde_json::Value>,
    pub retry_policy: Option<WebhookRetryPolicy>,
}

/// Update webhook subscription request.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct UpdateWebhookSubscription {
    pub name: Option<String>,
    pub description: Option<String>,
    pub url: Option<String>,
    pub events: Option<Vec<String>>,
    pub secret: Option<String>,
    pub headers: Option<serde_json::Value>,
    pub status: Option<String>,
    pub retry_policy: Option<WebhookRetryPolicy>,
}

/// Webhook retry policy.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WebhookRetryPolicy {
    pub max_retries: i32,
    pub retry_interval_seconds: i32,
    pub exponential_backoff: bool,
}

/// Webhook delivery log entity.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct WebhookDeliveryLog {
    pub id: Uuid,
    pub subscription_id: Uuid,
    pub event_type: String,
    pub event_id: Uuid,
    pub payload: serde_json::Value,
    pub status: String,
    pub attempts: i32,
    pub last_attempt_at: Option<DateTime<Utc>>,
    pub next_retry_at: Option<DateTime<Utc>>,
    pub response_status: Option<i32>,
    pub response_body: Option<String>,
    pub error_message: Option<String>,
    pub duration_ms: Option<i32>,
    pub created_at: DateTime<Utc>,
}

/// Webhook delivery log query.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WebhookDeliveryQuery {
    pub subscription_id: Option<Uuid>,
    pub event_type: Option<String>,
    pub status: Option<String>,
    pub from_date: Option<DateTime<Utc>>,
    pub to_date: Option<DateTime<Utc>>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

/// Webhook event types.
pub mod webhook_event {
    // Fault events
    pub const FAULT_CREATED: &str = "fault.created";
    pub const FAULT_UPDATED: &str = "fault.updated";
    pub const FAULT_RESOLVED: &str = "fault.resolved";

    // Document events
    pub const DOCUMENT_UPLOADED: &str = "document.uploaded";
    pub const DOCUMENT_SIGNED: &str = "document.signed";

    // Payment events
    pub const PAYMENT_RECEIVED: &str = "payment.received";
    pub const PAYMENT_OVERDUE: &str = "payment.overdue";

    // Meeting events
    pub const MEETING_SCHEDULED: &str = "meeting.scheduled";
    pub const MEETING_STARTED: &str = "meeting.started";
    pub const MEETING_ENDED: &str = "meeting.ended";

    // Announcement events
    pub const ANNOUNCEMENT_PUBLISHED: &str = "announcement.published";

    // Vote events
    pub const VOTE_STARTED: &str = "vote.started";
    pub const VOTE_ENDED: &str = "vote.ended";

    // Visitor events
    pub const VISITOR_CHECKED_IN: &str = "visitor.checked_in";
    pub const VISITOR_CHECKED_OUT: &str = "visitor.checked_out";

    // Package events
    pub const PACKAGE_RECEIVED: &str = "package.received";
    pub const PACKAGE_PICKED_UP: &str = "package.picked_up";
}

/// Webhook statistics.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WebhookStatistics {
    pub total_deliveries: i64,
    pub successful_deliveries: i64,
    pub failed_deliveries: i64,
    pub pending_deliveries: i64,
    pub average_response_time_ms: Option<f64>,
    pub success_rate: f64,
}

/// Test webhook request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TestWebhookRequest {
    pub event_type: String,
    pub payload: Option<serde_json::Value>,
}

/// Test webhook response.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TestWebhookResponse {
    pub success: bool,
    pub status_code: Option<i32>,
    pub response_time_ms: Option<i32>,
    pub error: Option<String>,
}

// ============================================
// Integration Statistics
// ============================================

/// Integration dashboard statistics.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct IntegrationStatistics {
    pub calendar_connections: i32,
    pub active_calendar_syncs: i32,
    pub accounting_exports_this_month: i32,
    pub esignature_workflows_pending: i32,
    pub esignature_workflows_completed: i32,
    pub video_meetings_scheduled: i32,
    pub webhook_subscriptions: i32,
    pub webhook_deliveries_today: i64,
    pub webhook_success_rate: f64,
}

// ============================================
// URL Validation for Webhooks
// ============================================

/// Webhook URL validation result.
#[derive(Debug, Clone)]
pub struct WebhookUrlValidation {
    pub is_valid: bool,
    pub error: Option<String>,
}

impl WebhookUrlValidation {
    pub fn valid() -> Self {
        Self {
            is_valid: true,
            error: None,
        }
    }

    pub fn invalid(error: impl Into<String>) -> Self {
        Self {
            is_valid: false,
            error: Some(error.into()),
        }
    }
}

/// Validates a webhook URL for security requirements.
///
/// Requirements:
/// - Must be a valid URL format
/// - Must use HTTPS (HTTP allowed only for localhost in development)
/// - Must not target private IP ranges (10.x, 172.16-31.x, 192.168.x, 127.x)
/// - Must not target localhost in production
pub fn validate_webhook_url(url: &str, is_production: bool) -> WebhookUrlValidation {
    // Parse URL
    let parsed = match url::Url::parse(url) {
        Ok(u) => u,
        Err(e) => {
            return WebhookUrlValidation::invalid(format!("Invalid URL format: {}", e));
        }
    };

    // Check scheme
    let scheme = parsed.scheme();
    let host = match parsed.host_str() {
        Some(h) => h,
        None => {
            return WebhookUrlValidation::invalid("URL must have a host");
        }
    };

    let is_localhost = host == "localhost" || host == "127.0.0.1" || host == "::1";

    // Require HTTPS except for localhost in development
    if scheme != "https" {
        if scheme == "http" && is_localhost && !is_production {
            // Allow HTTP for localhost in development
        } else {
            return WebhookUrlValidation::invalid(
                "Webhook URL must use HTTPS (HTTP only allowed for localhost in development)",
            );
        }
    }

    // Block localhost in production
    if is_production && is_localhost {
        return WebhookUrlValidation::invalid("Localhost URLs not allowed in production");
    }

    // Check for private IP ranges
    if let Some(host) = parsed.host() {
        match host {
            url::Host::Ipv4(ip) => {
                let octets = ip.octets();

                // 10.0.0.0/8 - Private
                if octets[0] == 10 {
                    return WebhookUrlValidation::invalid(
                        "Private IP addresses (10.x.x.x) are not allowed",
                    );
                }

                // 172.16.0.0/12 - Private (172.16.x.x - 172.31.x.x)
                if octets[0] == 172 && (octets[1] >= 16 && octets[1] <= 31) {
                    return WebhookUrlValidation::invalid(
                        "Private IP addresses (172.16-31.x.x) are not allowed",
                    );
                }

                // 192.168.0.0/16 - Private
                if octets[0] == 192 && octets[1] == 168 {
                    return WebhookUrlValidation::invalid(
                        "Private IP addresses (192.168.x.x) are not allowed",
                    );
                }

                // 127.0.0.0/8 - Loopback (except in dev for localhost)
                if octets[0] == 127 && (is_production || !is_localhost) {
                    return WebhookUrlValidation::invalid(
                        "Loopback addresses (127.x.x.x) are not allowed",
                    );
                }

                // 169.254.0.0/16 - Link-local
                if octets[0] == 169 && octets[1] == 254 {
                    return WebhookUrlValidation::invalid(
                        "Link-local addresses (169.254.x.x) are not allowed",
                    );
                }

                // 0.0.0.0/8 - Reserved
                if octets[0] == 0 {
                    return WebhookUrlValidation::invalid(
                        "Reserved addresses (0.x.x.x) are not allowed",
                    );
                }

                // 255.255.255.255 - Limited broadcast address
                if octets == [255, 255, 255, 255] {
                    return WebhookUrlValidation::invalid(
                        "Broadcast address (255.255.255.255) is not allowed",
                    );
                }

                // 224.0.0.0/4 - Multicast (224.x.x.x - 239.x.x.x)
                if octets[0] >= 224 && octets[0] <= 239 {
                    return WebhookUrlValidation::invalid(
                        "Multicast addresses (224.0.0.0/4) are not allowed",
                    );
                }
            }
            url::Host::Ipv6(ip) => {
                // Block IPv6 loopback and link-local
                if ip.is_loopback() && (is_production || !is_localhost) {
                    return WebhookUrlValidation::invalid(
                        "IPv6 loopback address (::1) is not allowed",
                    );
                }
                // Note: IPv6 unique local (fc00::/7) and link-local (fe80::/10) should also be blocked
                let segments = ip.segments();
                if (segments[0] & 0xfe00) == 0xfc00 {
                    return WebhookUrlValidation::invalid(
                        "IPv6 unique local addresses are not allowed",
                    );
                }
                if (segments[0] & 0xffc0) == 0xfe80 {
                    return WebhookUrlValidation::invalid(
                        "IPv6 link-local addresses are not allowed",
                    );
                }
            }
            url::Host::Domain(_) => {
                // Domain names are allowed (DNS resolution will happen at request time)
                // Note: DNS rebinding attacks are possible but mitigated by:
                // 1. Not following redirects in webhook requests
                // 2. Using short timeouts
                // For production systems, consider implementing domain allowlisting
                // or DNS resolution validation at request time
            }
        }
    }

    WebhookUrlValidation::valid()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_https_url() {
        let result = validate_webhook_url("https://example.com/webhook", true);
        assert!(result.is_valid);
    }

    #[test]
    fn test_http_url_rejected_in_production() {
        let result = validate_webhook_url("http://example.com/webhook", true);
        assert!(!result.is_valid);
        assert!(result.error.unwrap().contains("HTTPS"));
    }

    #[test]
    fn test_localhost_rejected_in_production() {
        let result = validate_webhook_url("https://localhost/webhook", true);
        assert!(!result.is_valid);
        assert!(result.error.unwrap().contains("Localhost"));
    }

    #[test]
    fn test_localhost_allowed_in_development() {
        let result = validate_webhook_url("http://localhost:3000/webhook", false);
        assert!(result.is_valid);
    }

    #[test]
    fn test_private_ip_10_rejected() {
        let result = validate_webhook_url("https://10.0.0.1/webhook", true);
        assert!(!result.is_valid);
        assert!(result.error.unwrap().contains("10.x"));
    }

    #[test]
    fn test_private_ip_172_rejected() {
        let result = validate_webhook_url("https://172.16.0.1/webhook", true);
        assert!(!result.is_valid);
        assert!(result.error.unwrap().contains("172.16"));
    }

    #[test]
    fn test_private_ip_192_rejected() {
        let result = validate_webhook_url("https://192.168.1.1/webhook", true);
        assert!(!result.is_valid);
        assert!(result.error.unwrap().contains("192.168"));
    }

    #[test]
    fn test_invalid_url_format() {
        let result = validate_webhook_url("not-a-url", true);
        assert!(!result.is_valid);
        assert!(result.error.unwrap().contains("Invalid URL"));
    }

    #[test]
    fn test_esignature_terminal_statuses() {
        use esignature_status::*;
        // Terminal: a re-delivered webhook for these must be a no-op.
        assert!(is_terminal(COMPLETED));
        assert!(is_terminal(VOIDED));
        assert!(is_terminal(DECLINED));
        assert!(is_terminal("completed"));
        assert!(is_terminal("voided"));
        assert!(is_terminal("declined"));
    }

    #[test]
    fn test_esignature_non_terminal_statuses() {
        use esignature_status::*;
        // Non-terminal: webhook updates may still transition these.
        assert!(!is_terminal(DRAFT));
        assert!(!is_terminal(SENT));
        assert!(!is_terminal(VIEWED));
        assert!(!is_terminal(SIGNED));
        // `expired` is a final state but is NOT in the webhook idempotency
        // guard set (the task scopes the guard to completed/voided/declined).
        assert!(!is_terminal(EXPIRED));
        assert!(!is_terminal("unknown"));
    }

    #[test]
    fn test_esignature_terminal_set_matches_predicate() {
        // The array used for the SQL `status <> ALL($3)` guard and the
        // `is_terminal` predicate must never drift apart.
        for s in esignature_status::TERMINAL {
            assert!(esignature_status::is_terminal(s));
        }
        assert_eq!(esignature_status::TERMINAL.len(), 3);
    }
}
