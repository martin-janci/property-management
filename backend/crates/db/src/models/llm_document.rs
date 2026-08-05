//! LLM Document models (Epic 64: Advanced AI & LLM Capabilities).
//!
//! Provides models for AI-powered document generation, including:
//! - Lease agreement generation (Story 64.1)
//! - Property listing descriptions (Story 64.2)
//! - AI photo enhancement (Story 64.4)
//! - Voice assistant integration (Story 64.5)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

// =============================================================================
// LLM Provider Types
// =============================================================================

/// LLM provider enum values.
pub mod llm_provider {
    pub const OPENAI: &str = "openai";
    pub const ANTHROPIC: &str = "anthropic";
    pub const AZURE_OPENAI: &str = "azure_openai";
    pub const LOCAL: &str = "local";
}

/// Document generation status.
pub mod generation_status {
    pub const PENDING: &str = "pending";
    pub const IN_PROGRESS: &str = "in_progress";
    pub const COMPLETED: &str = "completed";
    pub const FAILED: &str = "failed";
    pub const CANCELLED: &str = "cancelled";
}

/// Photo enhancement status.
pub mod enhancement_status {
    pub const PENDING: &str = "pending";
    pub const PROCESSING: &str = "processing";
    pub const COMPLETED: &str = "completed";
    pub const FAILED: &str = "failed";
}

/// Voice assistant platform.
pub mod voice_platform {
    pub const ALEXA: &str = "alexa";
    pub const GOOGLE_ASSISTANT: &str = "google_assistant";
}

/// Supported languages for AI generation.
pub mod supported_language {
    pub const SLOVAK: &str = "sk";
    pub const CZECH: &str = "cs";
    pub const GERMAN: &str = "de";
    pub const ENGLISH: &str = "en";
    pub const ALL: &[&str] = &[SLOVAK, CZECH, GERMAN, ENGLISH];
}

// =============================================================================
// Story 64.1: LLM-Powered Lease Agreement Generation
// =============================================================================

/// LLM generation request entity.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct LlmGenerationRequest {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub user_id: Uuid,
    pub request_type: String,
    pub provider: String,
    pub model: String,
    pub input_data: serde_json::Value,
    pub prompt_template_id: Option<Uuid>,
    pub status: String,
    pub result: Option<serde_json::Value>,
    pub error_message: Option<String>,
    pub tokens_used: Option<i32>,
    pub cost_cents: Option<i32>,
    pub latency_ms: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Lease generation request data.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LeaseGenerationInput {
    pub unit_id: Uuid,
    pub landlord_name: String,
    pub landlord_address: Option<String>,
    pub tenant_name: String,
    pub tenant_email: String,
    pub tenant_phone: Option<String>,
    pub start_date: String,
    pub end_date: String,
    pub monthly_rent: f64,
    pub security_deposit: f64,
    pub currency: String,
    pub language: String,
    pub additional_terms: Option<Vec<String>>,
    pub include_pet_clause: bool,
    pub include_parking: bool,
    pub jurisdiction: Option<String>,
}

/// Lease generation response.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LeaseGenerationResult {
    pub document_html: String,
    pub document_text: String,
    pub clauses: Vec<LeaseClause>,
    pub warnings: Vec<String>,
    pub compliance_notes: Option<String>,
}

/// Individual lease clause.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LeaseClause {
    pub title: String,
    pub content: String,
    pub is_mandatory: bool,
    pub source: Option<String>,
}

/// Request to generate a lease agreement.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GenerateLeaseRequest {
    pub unit_id: Uuid,
    pub landlord_name: String,
    pub landlord_address: Option<String>,
    pub tenant_name: String,
    pub tenant_email: String,
    pub tenant_phone: Option<String>,
    pub start_date: String,
    pub end_date: String,
    pub monthly_rent: f64,
    pub security_deposit: f64,
    #[serde(default = "default_currency")]
    pub currency: String,
    #[serde(default = "default_language")]
    pub language: String,
    pub additional_terms: Option<Vec<String>>,
    #[serde(default)]
    pub include_pet_clause: bool,
    #[serde(default)]
    pub include_parking: bool,
    pub jurisdiction: Option<String>,
    pub template_id: Option<Uuid>,
}

fn default_currency() -> String {
    "EUR".to_string()
}

fn default_language() -> String {
    "sk".to_string()
}

/// Prompt template for LLM generation.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct LlmPromptTemplate {
    pub id: Uuid,
    pub organization_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub request_type: String,
    pub system_prompt: String,
    pub user_prompt_template: String,
    pub variables: serde_json::Value,
    pub provider: String,
    pub model: String,
    pub temperature: Option<f32>,
    pub max_tokens: Option<i32>,
    pub is_system: bool,
    pub is_active: bool,
    pub version: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request to create a prompt template.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreatePromptTemplate {
    pub name: String,
    pub description: Option<String>,
    pub request_type: String,
    pub system_prompt: String,
    pub user_prompt_template: String,
    pub variables: Option<serde_json::Value>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<i32>,
}

/// Request to update a prompt template.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UpdatePromptTemplate {
    pub name: Option<String>,
    pub description: Option<String>,
    pub system_prompt: Option<String>,
    pub user_prompt_template: Option<String>,
    pub variables: Option<serde_json::Value>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<i32>,
    pub is_active: Option<bool>,
}

// =============================================================================
// Story 64.2: AI Property Listing Description Generator
// =============================================================================

/// Generated listing description entity.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct GeneratedListingDescription {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub listing_id: Option<Uuid>,
    pub user_id: Uuid,
    pub language: String,
    pub original_description: String,
    pub property_details: serde_json::Value,
    pub photo_analysis: Option<serde_json::Value>,
    pub generated_at: DateTime<Utc>,
    pub edited_description: Option<String>,
    pub edited_at: Option<DateTime<Utc>>,
    pub edited_by: Option<Uuid>,
    pub is_published: bool,
    pub generation_request_id: Uuid,
    pub created_at: DateTime<Utc>,
}

/// Request to generate a listing description.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GenerateListingDescriptionRequest {
    pub listing_id: Option<Uuid>,
    pub property_type: String,
    pub transaction_type: String,
    pub size_sqm: Option<f64>,
    pub rooms: Option<i32>,
    pub bathrooms: Option<i32>,
    pub floor: Option<i32>,
    pub total_floors: Option<i32>,
    pub features: Vec<String>,
    pub location: ListingLocation,
    pub price: f64,
    pub currency: String,
    #[serde(default = "default_language")]
    pub language: String,
    pub photo_urls: Option<Vec<String>>,
    pub style: Option<DescriptionStyle>,
    pub max_length: Option<i32>,
}

/// Location information for listing.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ListingLocation {
    pub city: String,
    pub district: Option<String>,
    pub street: Option<String>,
    pub nearby_amenities: Option<Vec<String>>,
    pub transport_access: Option<Vec<String>>,
}

/// Style options for generated descriptions.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DescriptionStyle {
    pub tone: Option<String>, // "professional", "casual", "luxury"
    pub highlight_features: Option<Vec<String>>,
    pub include_call_to_action: Option<bool>,
}

/// Response from listing description generation.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GeneratedListingDescriptionResponse {
    pub id: Uuid,
    pub description: String,
    pub key_highlights: Vec<String>,
    pub suggested_title: Option<String>,
    pub seo_keywords: Option<Vec<String>>,
    pub confidence_score: f64,
    pub tokens_used: i32,
}

// =============================================================================
// Story 64.3: Conversational AI Tenant Support (Enhanced RAG)
// =============================================================================

/// Document context for RAG.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct DocumentEmbedding {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub document_id: Uuid,
    pub chunk_index: i32,
    pub chunk_text: String,
    // The `embedding` column is JSONB (00081 — the JSONB fallback used when
    // pgvector is absent), not a FLOAT4[]. Without `#[sqlx(json)]`, FromRow
    // decodes `Vec<f32>` as FLOAT4[] and any `SELECT *`/`RETURNING *` fails with
    // ColumnDecode (JSONB vs FLOAT4[]). `json(nullable)` maps a SQL NULL
    // embedding (chunk not yet embedded) to `None`.
    #[sqlx(json(nullable))]
    pub embedding: Option<Vec<f32>>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// RAG context source.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RagContextSource {
    pub document_id: Uuid,
    pub document_title: String,
    pub chunk_text: String,
    pub relevance_score: f64,
    pub page_number: Option<i32>,
}

/// Enhanced chat message with RAG context.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EnhancedChatRequest {
    pub message: String,
    pub session_id: Option<Uuid>,
    #[serde(default = "default_language")]
    pub language: String,
    pub building_id: Option<Uuid>,
    pub include_document_context: Option<bool>,
    pub max_context_chunks: Option<i32>,
}

/// Enhanced chat response with sources.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EnhancedChatResponse {
    pub message_id: Uuid,
    pub response: String,
    pub confidence: f64,
    pub sources: Vec<RagContextSource>,
    pub escalated: bool,
    pub escalation_reason: Option<String>,
    pub language_detected: String,
    pub tokens_used: i32,
}

/// Escalation threshold configuration.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct AiEscalationConfig {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub confidence_threshold: f64,
    pub escalation_email: Option<String>,
    pub escalation_webhook_url: Option<String>,
    pub auto_escalate_topics: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request to update escalation config.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UpdateEscalationConfig {
    pub confidence_threshold: Option<f64>,
    pub escalation_email: Option<String>,
    pub escalation_webhook_url: Option<String>,
    pub auto_escalate_topics: Option<Vec<String>>,
}

// =============================================================================
// Story 64.4: AI Photo Enhancement for Listings
// =============================================================================

/// Photo enhancement record.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct PhotoEnhancement {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub listing_id: Option<Uuid>,
    pub user_id: Uuid,
    pub original_photo_url: String,
    pub enhanced_photo_url: Option<String>,
    pub thumbnail_url: Option<String>,
    pub enhancement_type: String,
    pub status: String,
    pub error_message: Option<String>,
    pub metadata: serde_json::Value,
    pub processing_time_ms: Option<i32>,
    pub cost_cents: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Photo enhancement types.
pub mod enhancement_type {
    pub const AUTO_ENHANCE: &str = "auto_enhance";
    pub const SKY_REPLACEMENT: &str = "sky_replacement";
    pub const VIRTUAL_STAGING: &str = "virtual_staging";
    pub const OBJECT_REMOVAL: &str = "object_removal";
    pub const HDR: &str = "hdr";
    pub const PERSPECTIVE_CORRECTION: &str = "perspective_correction";
}

/// Request to enhance a photo.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct EnhancePhotoRequest {
    pub photo_url: String,
    pub listing_id: Option<Uuid>,
    #[serde(default = "default_enhancement_type")]
    pub enhancement_type: String,
    pub options: Option<EnhancementOptions>,
}

fn default_enhancement_type() -> String {
    enhancement_type::AUTO_ENHANCE.to_string()
}

/// Enhancement options.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EnhancementOptions {
    pub brightness_adjust: Option<f32>,
    pub contrast_adjust: Option<f32>,
    pub saturation_adjust: Option<f32>,
    pub sky_type: Option<String>,      // "blue_sky", "sunset", "cloudy"
    pub staging_style: Option<String>, // "modern", "traditional", "minimalist"
    pub objects_to_remove: Option<Vec<String>>,
}

/// Response from photo enhancement.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PhotoEnhancementResponse {
    pub id: Uuid,
    pub status: String,
    pub enhanced_photo_url: Option<String>,
    pub thumbnail_url: Option<String>,
    pub processing_time_ms: Option<i32>,
    pub is_ai_enhanced: bool,
}

/// Batch photo enhancement request.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct BatchEnhancePhotosRequest {
    pub photo_urls: Vec<String>,
    pub listing_id: Option<Uuid>,
    #[serde(default = "default_enhancement_type")]
    pub enhancement_type: String,
}

/// Batch enhancement response.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BatchEnhancePhotosResponse {
    pub batch_id: Uuid,
    pub total_photos: i32,
    pub enhancements: Vec<PhotoEnhancementResponse>,
}

// =============================================================================
// Story 64.5: Voice Assistant Integration
// =============================================================================

/// Voice assistant device link.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct VoiceAssistantDevice {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub user_id: Uuid,
    pub unit_id: Option<Uuid>,
    pub platform: String,
    pub device_id: String,
    pub device_name: Option<String>,
    pub linked_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub access_token_encrypted: Option<String>,
    pub refresh_token_encrypted: Option<String>,
    pub token_expires_at: Option<DateTime<Utc>>,
    pub is_active: bool,
    pub capabilities: serde_json::Value,
    /// Keyed HMAC-SHA256 of the access token — an indexed, deterministic lookup
    /// key so voice-webhook auth selects the candidate device in SQL instead of
    /// decrypt-and-scanning every active device for the platform (#2662). NULL
    /// for rows linked before the column existed (fall back to the linear scan).
    #[serde(default)]
    pub access_token_hash: Option<Vec<u8>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Voice command history.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct VoiceCommandHistory {
    pub id: Uuid,
    pub device_id: Uuid,
    pub user_id: Uuid,
    pub command_text: String,
    pub intent_detected: Option<String>,
    pub response_text: String,
    pub action_taken: Option<String>,
    pub success: bool,
    pub error_message: Option<String>,
    pub processing_time_ms: i32,
    pub created_at: DateTime<Utc>,
}

/// Request to link a voice assistant device.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct LinkVoiceDeviceRequest {
    pub platform: String,
    pub auth_code: String,
    pub device_name: Option<String>,
    pub unit_id: Option<Uuid>,
}

/// Response from device linking.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LinkVoiceDeviceResponse {
    pub device_id: Uuid,
    pub platform: String,
    pub device_name: Option<String>,
    pub capabilities: Vec<String>,
    pub linked_at: DateTime<Utc>,
}

/// Voice command request (from webhook).
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct VoiceCommandRequest {
    pub device_id: Uuid,
    pub command_text: String,
    pub intent: Option<String>,
    pub slots: Option<serde_json::Value>,
}

/// Voice command response.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VoiceCommandResponse {
    pub response_text: String,
    pub ssml: Option<String>,
    pub card_title: Option<String>,
    pub card_content: Option<String>,
    pub should_end_session: bool,
}

/// Supported voice intents.
pub mod voice_intent {
    pub const CHECK_BALANCE: &str = "check_balance";
    pub const REPORT_FAULT: &str = "report_fault";
    pub const CHECK_ANNOUNCEMENTS: &str = "check_announcements";
    pub const BOOK_FACILITY: &str = "book_facility";
    pub const CHECK_METER: &str = "check_meter";
    pub const CONTACT_MANAGER: &str = "contact_manager";
    pub const GET_HELP: &str = "get_help";
}

// =============================================================================
// Statistics and Analytics
// =============================================================================

/// AI usage statistics.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AiUsageStatistics {
    pub total_generations: i64,
    pub successful_generations: i64,
    pub failed_generations: i64,
    pub total_tokens_used: i64,
    pub total_cost_cents: i64,
    pub average_latency_ms: f64,
    pub by_request_type: Vec<RequestTypeStats>,
    pub by_provider: Vec<ProviderStats>,
}

/// Stats by request type.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RequestTypeStats {
    pub request_type: String,
    pub count: i64,
    pub tokens_used: i64,
    pub cost_cents: i64,
}

/// Stats by provider.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProviderStats {
    pub provider: String,
    pub count: i64,
    pub tokens_used: i64,
    pub cost_cents: i64,
    pub average_latency_ms: f64,
}

/// Query parameters for AI usage.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct AiUsageQuery {
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub request_type: Option<String>,
    pub provider: Option<String>,
}

// =============================================================================
// Epic 93: Voice Assistant OAuth Completion
// =============================================================================

/// Story 93.1: OAuth token exchange request from voice assistant platform.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct VoiceOAuthExchangeRequest {
    /// OAuth authorization code from voice platform.
    pub auth_code: String,
    /// Voice platform (alexa or google_assistant).
    pub platform: String,
    /// Redirect URI used in OAuth flow.
    pub redirect_uri: String,
    /// Optional state parameter for CSRF protection.
    pub state: Option<String>,
}

/// Story 93.1: OAuth token exchange response.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VoiceOAuthExchangeResponse {
    /// Device ID after successful linking.
    pub device_id: Uuid,
    /// Whether the exchange was successful.
    pub success: bool,
    /// User-friendly message.
    pub message: String,
    /// Linked capabilities.
    pub capabilities: Vec<String>,
}

/// Story 93.1: Token refresh request.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct VoiceTokenRefreshRequest {
    /// Device ID to refresh token for.
    pub device_id: Uuid,
}

/// Story 93.1: Token refresh result.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VoiceTokenRefreshResult {
    /// Whether refresh was successful.
    pub success: bool,
    /// New expiration time if successful.
    pub expires_at: Option<DateTime<Utc>>,
    /// Error message if refresh failed.
    pub error: Option<String>,
}

/// Story 93.2: Parsed voice command with intent and slots.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ParsedVoiceCommand {
    /// Original command text.
    pub raw_text: String,
    /// Detected intent (check_balance, report_fault, etc.).
    pub intent: String,
    /// Confidence score (0.0-1.0).
    pub confidence: f64,
    /// Extracted slots/entities.
    pub slots: serde_json::Value,
    /// Language detected.
    pub language: String,
}

/// Story 93.2: Voice command action result.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VoiceActionResult {
    /// Whether the action was successful.
    pub success: bool,
    /// Action type that was taken.
    pub action_type: String,
    /// Human-readable response.
    pub response_text: String,
    /// SSML for speech synthesis.
    pub ssml: Option<String>,
    /// Optional card for display devices.
    pub card: Option<VoiceCard>,
    /// Whether to end the conversation.
    pub should_end_session: bool,
    /// Data returned by the action (e.g., fault ID, balance amount).
    pub data: Option<serde_json::Value>,
}

/// Story 93.2: Voice card for visual display.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VoiceCard {
    /// Card title.
    pub title: String,
    /// Card content/body.
    pub content: String,
    /// Optional image URL.
    pub image_url: Option<String>,
}

/// Story 93.3: Alexa Skills Kit request.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct AlexaSkillRequest {
    /// Request version.
    pub version: String,
    /// Session information.
    pub session: AlexaSession,
    /// Context information.
    pub context: serde_json::Value,
    /// The actual request.
    pub request: AlexaRequestBody,
}

/// Story 93.3: Alexa session information.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct AlexaSession {
    /// Whether this is a new session.
    pub new: bool,
    /// Session ID.
    #[serde(rename = "sessionId")]
    pub session_id: String,
    /// Application information.
    pub application: AlexaApplication,
    /// User information.
    pub user: AlexaUser,
}

/// Story 93.3: Alexa application info.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct AlexaApplication {
    /// Skill application ID.
    #[serde(rename = "applicationId")]
    pub application_id: String,
}

/// Story 93.3: Alexa user info.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct AlexaUser {
    /// User ID.
    #[serde(rename = "userId")]
    pub user_id: String,
    /// Access token from account linking.
    #[serde(rename = "accessToken")]
    pub access_token: Option<String>,
}

/// Story 93.3: Alexa request body.
#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(tag = "type")]
pub enum AlexaRequestBody {
    /// Launch request (skill opened).
    LaunchRequest {
        #[serde(rename = "requestId")]
        request_id: String,
        timestamp: String,
        locale: String,
    },
    /// Intent request (user spoke a command).
    IntentRequest {
        #[serde(rename = "requestId")]
        request_id: String,
        timestamp: String,
        locale: String,
        intent: AlexaIntent,
    },
    /// Session ended.
    SessionEndedRequest {
        #[serde(rename = "requestId")]
        request_id: String,
        timestamp: String,
        locale: String,
        reason: String,
    },
}

/// Story 93.3: Alexa intent.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct AlexaIntent {
    /// Intent name.
    pub name: String,
    /// Intent slots.
    pub slots: Option<serde_json::Value>,
}

/// Story 93.3: Alexa skill response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AlexaSkillResponse {
    /// Response version.
    pub version: String,
    /// Response body.
    pub response: AlexaResponseBody,
}

/// Story 93.3: Alexa response body.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AlexaResponseBody {
    /// Speech output.
    #[serde(rename = "outputSpeech")]
    pub output_speech: AlexaOutputSpeech,
    /// Optional card.
    pub card: Option<AlexaCard>,
    /// Whether to end the session.
    #[serde(rename = "shouldEndSession")]
    pub should_end_session: bool,
}

/// Story 93.3: Alexa output speech.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AlexaOutputSpeech {
    /// Speech type (PlainText or SSML).
    #[serde(rename = "type")]
    pub speech_type: String,
    /// Text content (for PlainText).
    pub text: Option<String>,
    /// SSML content (for SSML type).
    pub ssml: Option<String>,
}

/// Story 93.3: Alexa card.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AlexaCard {
    /// Card type (Simple, Standard).
    #[serde(rename = "type")]
    pub card_type: String,
    /// Card title.
    pub title: String,
    /// Card content.
    pub content: Option<String>,
    /// Card text (for Standard cards).
    pub text: Option<String>,
}

/// Story 93.3: Google Actions request.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct GoogleActionsRequest {
    /// Handler information.
    pub handler: GoogleHandler,
    /// Intent information.
    pub intent: GoogleIntent,
    /// Scene information.
    pub scene: Option<GoogleScene>,
    /// Session information.
    pub session: GoogleSession,
    /// User information.
    pub user: GoogleUser,
    /// Device information.
    pub device: Option<serde_json::Value>,
}

/// Story 93.3: Google handler.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct GoogleHandler {
    /// Handler name.
    pub name: String,
}

/// Story 93.3: Google intent.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct GoogleIntent {
    /// Intent name.
    pub name: String,
    /// Intent parameters.
    pub params: Option<serde_json::Value>,
    /// Query text.
    pub query: Option<String>,
}

/// Story 93.3: Google scene.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct GoogleScene {
    /// Scene name.
    pub name: String,
    /// Slot filling status.
    #[serde(rename = "slotFillingStatus")]
    pub slot_filling_status: Option<String>,
    /// Slots.
    pub slots: Option<serde_json::Value>,
}

/// Story 93.3: Google session.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct GoogleSession {
    /// Session ID.
    pub id: String,
    /// Session parameters.
    pub params: Option<serde_json::Value>,
    /// Language code.
    #[serde(rename = "languageCode")]
    pub language_code: Option<String>,
}

/// Story 93.3: Google user.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct GoogleUser {
    /// User verification status.
    #[serde(rename = "verificationStatus")]
    pub verification_status: String,
    /// Account linking access token.
    #[serde(rename = "accountLinkingStatus")]
    pub account_linking_status: Option<String>,
    /// Params including access token.
    pub params: Option<serde_json::Value>,
}

/// Story 93.3: Google Actions response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct GoogleActionsResponse {
    /// Session params to persist.
    pub session: GoogleSessionResponse,
    /// Prompt to speak.
    pub prompt: GooglePrompt,
    /// Scene transition.
    pub scene: Option<GoogleSceneResponse>,
}

/// Story 93.3: Google session response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct GoogleSessionResponse {
    /// Session ID.
    pub id: String,
    /// Parameters to persist.
    pub params: Option<serde_json::Value>,
}

/// Story 93.3: Google prompt.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct GooglePrompt {
    /// Override mode.
    #[serde(rename = "override")]
    pub override_mode: bool,
    /// First simple response.
    #[serde(rename = "firstSimple")]
    pub first_simple: GoogleSimpleResponse,
    /// Optional content.
    pub content: Option<GoogleContent>,
}

/// Story 93.3: Google simple response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct GoogleSimpleResponse {
    /// Speech text.
    pub speech: String,
    /// Display text.
    pub text: Option<String>,
}

/// Story 93.3: Google content for rich responses.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct GoogleContent {
    /// Card content.
    pub card: Option<GoogleCard>,
}

/// Story 93.3: Google card.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct GoogleCard {
    /// Card title.
    pub title: String,
    /// Card subtitle.
    pub subtitle: Option<String>,
    /// Card text.
    pub text: String,
    /// Card image.
    pub image: Option<GoogleImage>,
}

/// Story 93.3: Google image.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct GoogleImage {
    /// Image URL.
    pub url: String,
    /// Alt text.
    pub alt: String,
}

/// Story 93.3: Google scene response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct GoogleSceneResponse {
    /// Scene name to transition to.
    pub name: String,
}

/// Story 93.3: Webhook signature verification result.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WebhookVerificationResult {
    /// Whether the signature is valid.
    pub valid: bool,
    /// Platform that was verified.
    pub platform: String,
    /// Error message if invalid.
    pub error: Option<String>,
}

// =============================================================================
// Story 103.5: pgvector RAG Statistics
// =============================================================================

/// RAG system statistics for an organization.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RagStatistics {
    /// Number of documents indexed for RAG.
    pub indexed_documents: i64,
    /// Total number of text chunks.
    pub total_chunks: i64,
    /// Chunks with vector embeddings (pgvector).
    pub chunks_with_vector: i64,
    /// Chunks pending migration from JSONB to vector.
    pub chunks_pending_migration: i64,
    /// Average chunk length in characters.
    pub avg_chunk_length: i32,
    /// Last time embeddings were updated.
    pub last_updated: Option<DateTime<Utc>>,
}
