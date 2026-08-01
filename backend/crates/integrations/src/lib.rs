//! External integrations: Airbnb, Booking, real estate portals, calendar sync, accounting exports.
//!
//! Epic 150: API Ecosystem Expansion - Connector framework and pre-built integrations.

pub mod airbnb;
pub mod booking;
pub mod portals;

// Epic 150: API Ecosystem Expansion
pub mod connector;
pub mod prebuilt;

// Epic 61: External Integrations Suite
pub mod accounting;
pub mod calendar;
pub mod crypto;

// Epic 96: OAuth Phase 2 Integrations
pub mod oauth;

// Epic 98: Voice Assistant OAuth
pub mod voice_oauth;

// Epic 84: S3 Storage Integration
pub mod storage;

// Epic 84.2: E-Signature Email Integration
pub mod esignature;

// Epic 103: Redis Integration (Cache, Sessions, Pub/Sub)
pub mod redis;

// Epic 64: Advanced AI & LLM Capabilities
pub mod llm;

// Story 84.5: Embedding provider abstraction for RAG (OpenAI + deterministic stub)
pub mod embedding;

// Story 3.1 AC3 (BIT-200): Building geocoding
pub mod geocoding;

// Epic 97: Workflow Execution Engine
pub mod workflow_executor;

// Story 2B.1 (#969 gap 2, BIT-178): event bus retry + exponential backoff
pub mod event_bus;

// Re-exports

// Story 83.1: Airbnb Integration
pub use airbnb::{
    map_reservation_status as map_airbnb_status, map_to_internal_reservation, AirbnbClient,
    AirbnbError, AirbnbGuest, AirbnbListing, AirbnbOAuthConfig, AirbnbOAuthTokens, AirbnbPhoto,
    AirbnbReservation, AirbnbReservationStatus, AirbnbWebhookEvent, AirbnbWebhookEventType,
    ListingSyncResult, Reservation, ReservationSyncResult, AIRBNB_API_BASE,
};

// Story 83.2: Booking.com Integration
pub use booking::ota_xml::{
    parse_avail_notif_rq, parse_rate_amount_notif_rq, ParsedAvailNotif, ParsedRateAmount,
    ParsedRateNotif,
};
pub use booking::{
    map_reservation_status as map_booking_status, AvailStatusMessage, AvailabilityUpdate,
    BookingAddress, BookingClient, BookingContact, BookingCredentials, BookingError, BookingGuest,
    BookingOAuthClient, BookingOAuthConfig, BookingOAuthTokens, BookingProperty,
    BookingReservation, BookingReservationStatus, BookingRoomType, LosRestrictions,
    OtaHotelAvailNotifRQ, OtaHotelAvailNotifRS, OtaHotelRateAmountNotifRQ,
    OtaHotelRateAmountNotifRS, OtaHotelResNotifRQ, OtaHotelResNotifRS, OtaReadRQ, OtaReadRS,
    OtaReservationNotification, PropertyMapping, PushOutcome, RateUpdate, RoomTypeMapping,
    BOOKING_OAUTH_AUTH_URL, BOOKING_OAUTH_TOKEN_URL,
};

// Story 83.3: Portal Webhooks
pub use portals::{
    compute_hmac_sha256, get_parser, parse_webhook, verify_timestamped_signature,
    verify_webhook_signature, BezrealitkyInquiryData, BezrealitkyParser, BezrealitkyWebhook,
    GenericParser, GenericWebhook, ImmoweltContact, ImmoweltParser, ImmoweltWebhook, InquiryStatus,
    ParseError, ParsedInquiry, PortalClient, PortalConnection, PortalError, PortalInquiry,
    PortalParser, PortalType, SrealityContact, SrealityParser, SrealityWebhook,
    TimestampedSignatureError,
};

// Story 61.2: Accounting System Export
pub use accounting::{
    AccountingError, ExportInvoice, ExportPayment, InvoiceItem, MoneyS3Exporter, Partner,
    PaymentType, PohodaExporter, ValidationResult, VatRate,
};

// Story 61.1: Calendar Integration
pub use calendar::{
    AttendeeStatus, CalendarError, CalendarListItem, EventAttendee, ExternalCalendarEvent,
    GoogleCalendarClient, MicrosoftCalendarClient, OAuthConfig, OAuthTokens, SyncResult,
};

// Encryption utilities for sensitive integration data
pub use crypto::{
    decrypt_if_available, encrypt_if_available, encrypt_optional_required, encrypt_required,
    CryptoError, IntegrationCrypto, ENCRYPTION_KEY_ENV,
};

// Story 96.1: OAuth Token Management
pub use oauth::{
    create_revocation_result, ConnectionsNeedingRefresh, DecryptedTokens, OAuthError,
    OAuthProvider, OAuthTokenManager, ProviderConfigs, RefreshResult, RevocationResult,
    StoredToken, TokenRefreshConfig, TokenRefreshScheduler, DEFAULT_REFRESH_BUFFER_SECS,
    MAX_REFRESH_BUFFER_SECS, MIN_REFRESH_BUFFER_SECS,
};

// Story 98.1: Voice Assistant OAuth
pub use voice_oauth::{
    VoiceOAuthClient, VoiceOAuthConfig, VoiceOAuthError, VoiceOAuthManager, VoiceOAuthTokens,
    VoicePlatform,
};

// Story 3.1 AC3 (BIT-200): Building geocoding
pub use geocoding::{Coordinates, GeocodingError, GeocodingProvider, GeocodingService};

// Story 64.1-64.4: LLM Integration
// Story 97.1-97.4: Enhanced LLM Capabilities
pub use llm::{
    token_limits, BatchSentimentResult, ChatCompletionRequest, ChatCompletionResponse, ChatMessage,
    ContextChunk, EmbeddingResult, EnhancedChatResult, LeaseGenerationInput, LeaseGenerationResult,
    ListingDescriptionInput, ListingDescriptionResult, LlmClient, LlmConfig, LlmError,
    SentimentResult, TenantAiConfig, TokenUsage,
};

// Story 84.5: Embedding provider abstraction for RAG
pub use embedding::{EmbeddingProvider, StubEmbeddingProvider, DEFAULT_EMBEDDING_DIM};

// Story 84.2: E-Signature Email Integration
pub use esignature::{
    DocusignClient, DocusignConfig, ESignatureError, ESignatureProvider, LightweightConfig,
    LightweightProvider, SigningToken, MIN_TOKEN_SECRET_LEN,
};

// Story 84.1: S3 Presigned URLs
pub use storage::{
    generate_callback_token, generate_storage_key, get_content_type, is_allowed_content_type,
    supports_inline_preview, DownloadUrlResponse, PresignedUrl, StorageConfig, StorageError,
    StorageService, UploadUrlResponse, ALLOWED_MIME_TYPES, DEFAULT_DOWNLOAD_EXPIRATION_SECS,
    DEFAULT_UPLOAD_EXPIRATION_SECS, MAX_UPLOAD_SIZE_BYTES,
};

// Story 97.3: Workflow Execution Engine
pub use workflow_executor::{
    action_type as workflow_action_type, ActionResult, AssignTaskConfig, CreateFaultConfig,
    DelayConfig, LlmResponseConfig, SendEmailConfig, SendNotificationConfig, WebhookConfig,
    WorkflowExecutionError, WorkflowExecutor,
};

// Story 103.2-103.4: Redis Integration
pub use redis::{
    channels as redis_channels, event_types as redis_events, CacheError, InMemoryBroker,
    PubSubMessage, PubSubService, RedisClient, RedisConfig, SessionData, SessionStore,
    CACHE_KEY_PREFIX, DEFAULT_CACHE_TTL_SECS, DEFAULT_SESSION_TTL_SECS, PUBSUB_CHANNEL_PREFIX,
    REDIS_URL_ENV, SESSION_KEY_PREFIX,
};

// Story 2B.1 (#969 gap 2, BIT-178): event bus retry + exponential backoff
pub use event_bus::{
    DeadLetter, EventBus, EventHandler, EventHandlerError, ExponentialBackoff, DEFAULT_BASE_DELAY,
    DEFAULT_MAX_DELAY, DEFAULT_MAX_RETRIES,
};

// Epic 150: API Ecosystem Expansion
// Story 150.2: Connector Framework
pub use connector::{
    AuthConfig, ConnectorAction, ConnectorConfig, ConnectorError, DataTransformer,
    ExecutionLogEntry, ExecutionResult, HttpConnector, RateLimitConfig, RateLimiterState,
    RetryConfig,
};

// Story 150.4: Pre-Built Integrations
pub use prebuilt::{
    HubSpotClient, HubSpotContact, HubSpotContactProperties, HubSpotDeal, HubSpotDealProperties,
    IntegrationSyncResult, QuickBooksAddress, QuickBooksClient, QuickBooksCustomer,
    QuickBooksEmailAddr, QuickBooksInvoice, QuickBooksInvoiceLine, QuickBooksLinkedTxn,
    QuickBooksPayment, QuickBooksPaymentLine, QuickBooksPhone, QuickBooksRef,
    QuickBooksSalesItemLineDetail, SalesforceClient, SalesforceContact, SalesforceLead,
    SalesforceOpportunity, SlackAttachment, SlackAttachmentField, SlackBlock, SlackChannel,
    SlackClient, SlackMessage, SlackTextObject, SyncError, TeamsAdaptiveCard, TeamsAttachment,
    TeamsClient, TeamsMessage, XeroAccountRef, XeroClient, XeroContact, XeroInvoice,
    XeroInvoiceRef, XeroLineItem, XeroPayment,
};
