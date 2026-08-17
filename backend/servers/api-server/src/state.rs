//! Application state.

use std::time::Instant;

use crate::services::id_ocr::{default_id_document_ocr, SharedIdDocumentOcr};
use crate::services::{
    AccountingService, AuthService, EmailService, JwtService, NotificationPipeline, OAuthService,
    PipelineConfig, TotpService,
};
use api_core::TenantMembershipProvider;
use db::{
    repositories::{
        AccountingProviderRepository, AccountingRepository, AgencyRepository, AiChatRepository,
        AnnouncementRepository, ApiEcosystemRepository, AuditLogRepository, AutomationRepository,
        BackgroundJobRepository, BoardMeetingRepository, BudgetRepository,
        BuildingCertificationRepository, BuildingRepository, CommunityRepository,
        ComplianceRepository, CriticalNotificationRepository, DataExportRepository,
        DataResidencyRepository, DelegationRepository, DevicePushTokenRepository,
        DisputeRepository, DocumentRepository, DocumentTemplateRepository,
        ESignatureNonceRepository, EddRepository, EmergencyRepository, EnergyRepository,
        EnhancedTenantScreeningRepository, EquipmentRepository, EsgReportingRepository,
        FacilityRepository, FaultRepository, FeatureAnalyticsRepository, FeatureFlagRepository,
        FeaturePackageRepository, FinancialRepository, FormRepository, GovernmentPortalRepository,
        GranularNotificationRepository, HealthMonitoringRepository, HelpRepository,
        InfrastructureRepository, InsuranceRepository, IntegrationRepository,
        InvestorPortalRepository, LeaseAbstractionRepository, LeaseRepository, LegalRepository,
        ListingRepository, LlmDocumentRepository, MarketPricingRepository, MarketplaceRepository,
        MeterRepository, MigrationRepository, MultiCurrencyRepository,
        NotificationPreferenceRepository, OAuthRepository, OAuthTokenEventRepository,
        OnboardingRepository, OperationsRepository, OrganizationMemberRepository,
        OrganizationRepository, OutageRepository, OwnerAnalyticsRepository,
        PackageVisitorRepository, PasswordResetRepository, PersonMonthRepository,
        PlatformAdminRepository, PortfolioAnalyticsRepository, PortfolioPerformanceRepository,
        PredictiveMaintenanceRepository, PropertyValuationRepository, RegionalComplianceRepository,
        RegistryRepository, RentalRepository, ReportScheduleRepository, ReserveFundRepository,
        RoleRepository, SensorRepository, SentimentRepository, SessionRepository,
        SignatureRequestRepository, SubscriptionRepository, SystemAnnouncementRepository,
        TwoFactorAuthRepository, UnitRepository, UnitResidentRepository, UserRepository,
        VendorRepository, ViolationRepository, VoteRepository, WorkOrderRepository,
        WorkflowRepository,
    },
    DbPool,
};
use integrations::{
    GeocodingService, LlmClient, PubSubService, RedisClient, SessionStore, StorageService,
};

/// Airbnb integration configuration loaded once at startup (issue #711).
///
/// Previously each handler called `std::env::var("AIRBNB_…")` per request:
///   * a missing env was only discovered when a user hit the endpoint,
///   * env reads on the hot path are a minor perf concern,
///   * it diverged from the project pattern of wiring credentials into
///     `AppState` at startup.
///
/// We now load these at server boot and stash them on `AppState`. Handlers
/// receive them via `State(state)` and check `airbnb_config.is_some()` /
/// emptiness exactly as before, just without touching the environment per
/// request.
#[derive(Debug, Clone, Default)]
pub struct AirbnbAppConfig {
    /// `AIRBNB_CLIENT_ID` — required for any Airbnb OAuth flow.
    pub client_id: String,
    /// `AIRBNB_CLIENT_SECRET` — required for token exchange.
    pub client_secret: String,
    /// `AIRBNB_REDIRECT_URI` — optional; defaulted by individual handlers
    /// when absent (kept here for completeness so handlers never re-read
    /// the env).
    pub redirect_uri: String,
    /// `AIRBNB_WEBHOOK_SECRET` — required for inbound webhook signature
    /// verification.
    pub webhook_secret: String,
    /// `AIRBNB_API_BASE` — base URL of the Airbnb Partner API used for
    /// listing/reservation calls. Defaults to the production endpoint
    /// ([`integrations::AIRBNB_API_BASE`]) when unset; overridable so
    /// integration tests can point the direct-connect success path at a stub
    /// server (issue #2240) and so a non-production Airbnb sandbox can be
    /// targeted without a code change. Unlike the credential fields, this is
    /// never empty — an unset env resolves to the production default.
    pub api_base: String,
}

impl AirbnbAppConfig {
    /// Load from the standard env vars. Empty strings are preserved for the
    /// credential fields so the handlers can still emit `NOT_CONFIGURED` for
    /// missing values — this matches the previous per-request behaviour
    /// exactly. `api_base` is the exception: an unset/empty `AIRBNB_API_BASE`
    /// resolves to the production default so the client URL is always valid.
    pub fn from_env() -> Self {
        let api_base = std::env::var("AIRBNB_API_BASE")
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| integrations::AIRBNB_API_BASE.to_string());
        Self {
            client_id: std::env::var("AIRBNB_CLIENT_ID").unwrap_or_default(),
            client_secret: std::env::var("AIRBNB_CLIENT_SECRET").unwrap_or_default(),
            redirect_uri: std::env::var("AIRBNB_REDIRECT_URI").unwrap_or_default(),
            webhook_secret: std::env::var("AIRBNB_WEBHOOK_SECRET").unwrap_or_default(),
            api_base,
        }
    }
}

/// Booking.com OAuth integration configuration loaded once at startup
/// (Coverage 83-2). Mirrors [`AirbnbAppConfig`]; secrets are never accepted
/// from a request body and a missing secret fails closed (503).
#[derive(Debug, Clone, Default)]
pub struct BookingOAuthAppConfig {
    /// `BOOKING_CLIENT_ID` — required for any Booking.com OAuth flow.
    pub client_id: String,
    /// `BOOKING_CLIENT_SECRET` — required for token exchange.
    pub client_secret: String,
    /// `BOOKING_REDIRECT_URI` — the registered OAuth callback URI.
    pub redirect_uri: String,
    /// `BOOKING_WEBHOOK_SECRET` — HMAC-SHA256 signing secret for the inbound
    /// push receiver (`X-Booking-Signature` over `"{X-Booking-Timestamp}.{body}"`).
    /// Required to accept a Booking.com push (audit F1/R1); empty ⇒ the receiver
    /// fails closed with `503 NOT_CONFIGURED` so an unverified, attacker-forged
    /// OTA payload is never processed.
    pub webhook_secret: String,
}

impl BookingOAuthAppConfig {
    /// Load from the standard env vars. Empty strings are preserved so the
    /// handler can emit `NOT_CONFIGURED` for missing values.
    pub fn from_env() -> Self {
        Self {
            client_id: std::env::var("BOOKING_CLIENT_ID").unwrap_or_default(),
            client_secret: std::env::var("BOOKING_CLIENT_SECRET").unwrap_or_default(),
            redirect_uri: std::env::var("BOOKING_REDIRECT_URI").unwrap_or_default(),
            webhook_secret: std::env::var("BOOKING_WEBHOOK_SECRET").unwrap_or_default(),
        }
    }
}

/// Stripe Checkout integration configuration loaded once at startup
/// (Story 11.5, [BIT-181]). Mirrors [`AirbnbAppConfig`]: secrets are read from
/// the environment at boot, never accepted from a request body, and an empty
/// `secret_key`/`webhook_secret` fails closed at the handler.
///
/// We use Stripe **hosted Checkout** (SAQ-A): raw card data never touches our
/// servers — the server only creates a Checkout Session and confirms the
/// webhook. So we hold the API secret key (server→Stripe) and the webhook
/// signing secret (Stripe→server), plus the redirect URLs the payer returns to.
#[derive(Debug, Clone, Default)]
pub struct StripeAppConfig {
    /// `STRIPE_SECRET_KEY` — server-side API key used to create Checkout
    /// Sessions. Required to initiate a checkout; empty ⇒ `503 NOT_CONFIGURED`.
    pub secret_key: String,
    /// `STRIPE_WEBHOOK_SECRET` — signing secret for inbound webhook signature
    /// verification (`Stripe-Signature` header). Required to accept a webhook;
    /// empty ⇒ the receiver fails closed with `503 NOT_CONFIGURED`.
    pub webhook_secret: String,
    /// `STRIPE_SUCCESS_URL` — where Stripe redirects the payer on success.
    /// `{CHECKOUT_SESSION_ID}` is substituted by Stripe.
    pub success_url: String,
    /// `STRIPE_CANCEL_URL` — where Stripe redirects the payer on cancel.
    pub cancel_url: String,
}

impl StripeAppConfig {
    /// Load from the standard env vars. Empty strings are preserved so the
    /// handlers can emit `NOT_CONFIGURED` for missing values — matching the
    /// fail-closed contract of the other integration configs.
    pub fn from_env() -> Self {
        Self {
            secret_key: std::env::var("STRIPE_SECRET_KEY").unwrap_or_default(),
            webhook_secret: std::env::var("STRIPE_WEBHOOK_SECRET").unwrap_or_default(),
            success_url: std::env::var("STRIPE_SUCCESS_URL").unwrap_or_default(),
            cancel_url: std::env::var("STRIPE_CANCEL_URL").unwrap_or_default(),
        }
    }
}

/// Portal (real-estate listing-site) inbound-webhook configuration loaded once
/// at startup. Mirrors [`AirbnbAppConfig`]/[`StripeAppConfig`]: the signing
/// secret is read from the environment at boot, never accepted from a request
/// body, and an empty `webhook_secret` fails closed at the handler so an
/// unverified, attacker-forged payload is never processed.
///
/// Backs the connection-scoped receiver
/// `POST /api/v1/integrations/webhooks/portal/{connection_id}` in
/// `routes/integrations/webhook.rs`. (The per-portal
/// `<PORTAL>_WEBHOOK_SECRET` receivers in `routes/portal_webhooks.rs` are a
/// separate surface keyed by portal name.)
#[derive(Debug, Clone, Default)]
pub struct PortalAppConfig {
    /// `PORTAL_WEBHOOK_SECRET` — HMAC-SHA256 signing secret for inbound portal
    /// webhook signature verification (`X-Webhook-Signature` header). Required
    /// to accept a portal webhook; empty ⇒ the receiver fails closed.
    pub webhook_secret: String,
}

impl PortalAppConfig {
    /// Load from the standard env var. An empty string is preserved so the
    /// handler can fail closed on a missing secret.
    pub fn from_env() -> Self {
        Self {
            webhook_secret: std::env::var("PORTAL_WEBHOOK_SECRET").unwrap_or_default(),
        }
    }
}

/// A single realtime preference-sync event captured by a
/// [`PreferenceEventRecorder`] (issue #1376).
///
/// Mirrors the `(channel, PubSubMessage)` pair the notification-preference
/// handler hands to `PubSubService::publish` — minus the Redis round-trip — so
/// the publish *contract* (target channel, event type, `{channel, enabled}`
/// payload) can be asserted in CI without a live Redis daemon.
#[derive(Clone, Debug)]
pub struct RecordedPreferenceEvent {
    /// The pub/sub channel the event would be published on, e.g.
    /// `notifications:{user_id}`.
    pub channel: String,
    /// The event type, e.g. `preference.updated`.
    pub event_type: String,
    /// The event payload, e.g. `{ "channel": "email", "enabled": false }`.
    pub payload: serde_json::Value,
}

/// Test-only sink that captures realtime preference-sync events the
/// notification-preference handler would publish (issue #1376).
///
/// The production realtime leg (`PubSubService::publish`) requires a live Redis
/// daemon, which CI does not provide — so the only previously-existing
/// publish-asserting test (S4) was `#[ignore]`d and never ran in CI, leaving
/// the actual point of the 8a-3 cluster (the publish/delivery contract) with
/// zero CI coverage.
///
/// This recorder is always compiled (a tiny `Arc<Mutex<Vec<_>>>`), defaults to
/// absent on the production `AppState`, and is installed only by tests via
/// [`AppState::with_pref_event_recorder`]. When present, the handler records
/// every event it would publish — *independently* of whether `pubsub_service`
/// is configured — giving CI a deterministic, non-flaky proof of the publish
/// contract while the real-Redis S4 test stays as the integration backstop.
#[derive(Clone, Default)]
pub struct PreferenceEventRecorder {
    events: std::sync::Arc<std::sync::Mutex<Vec<RecordedPreferenceEvent>>>,
}

impl PreferenceEventRecorder {
    /// Create an empty recorder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record one event the handler would have published.
    pub fn record(&self, channel: &str, event_type: &str, payload: serde_json::Value) {
        if let Ok(mut events) = self.events.lock() {
            events.push(RecordedPreferenceEvent {
                channel: channel.to_string(),
                event_type: event_type.to_string(),
                payload,
            });
        }
    }

    /// Drain and return all captured events, leaving the recorder empty.
    pub fn drain(&self) -> Vec<RecordedPreferenceEvent> {
        match self.events.lock() {
            Ok(mut events) => std::mem::take(&mut *events),
            Err(_) => Vec::new(),
        }
    }
}

/// Application state shared across all handlers.
#[derive(Clone)]
pub struct AppState {
    /// Boot time for uptime tracking (Story 88.1)
    pub boot_time: Instant,
    pub db: DbPool,
    pub user_repo: UserRepository,
    pub session_repo: SessionRepository,
    pub password_reset_repo: PasswordResetRepository,
    pub org_repo: OrganizationRepository,
    pub org_member_repo: OrganizationMemberRepository,
    pub role_repo: RoleRepository,
    pub building_repo: BuildingRepository,
    /// Story 3.1 AC3 (BIT-200): geocodes building addresses on write. Disabled
    /// (no-op) unless `GEOCODING_PROVIDER` + credentials are configured.
    pub geocoding: GeocodingService,
    pub unit_repo: UnitRepository,
    pub unit_resident_repo: UnitResidentRepository,
    pub delegation_repo: DelegationRepository,
    pub person_month_repo: PersonMonthRepository,
    pub facility_repo: FacilityRepository,
    pub fault_repo: FaultRepository,
    pub vote_repo: VoteRepository,
    pub announcement_repo: AnnouncementRepository,
    pub document_repo: DocumentRepository,
    pub document_template_repo: DocumentTemplateRepository,
    pub notification_pref_repo: NotificationPreferenceRepository,
    pub device_push_token_repo: DevicePushTokenRepository,
    pub critical_notification_repo: CriticalNotificationRepository,
    pub two_factor_repo: TwoFactorAuthRepository,
    pub audit_log_repo: AuditLogRepository,
    pub data_export_repo: DataExportRepository,
    pub data_residency_repo: DataResidencyRepository,
    pub oauth_repo: OAuthRepository,
    /// Epic 10A (data audit, #2628): OAuth token-usage analytics. Written
    /// best-effort by the token lifecycle path (`oauth_service`), read by the
    /// platform-admin token-usage endpoint, pruned daily by the scheduler.
    pub oauth_token_event_repo: OAuthTokenEventRepository,
    pub platform_admin_repo: PlatformAdminRepository,
    pub feature_flag_repo: FeatureFlagRepository,
    pub granular_notification_repo: GranularNotificationRepository,
    pub health_monitoring_repo: HealthMonitoringRepository,
    pub system_announcement_repo: SystemAnnouncementRepository,
    pub onboarding_repo: OnboardingRepository,
    pub help_repo: HelpRepository,
    pub signature_request_repo: SignatureRequestRepository,
    pub e_signature_nonce_repo: ESignatureNonceRepository,
    pub financial_repo: FinancialRepository,
    pub meter_repo: MeterRepository,
    // Epic 13: AI Assistant & Automation
    pub ai_chat_repo: AiChatRepository,
    pub sentiment_repo: SentimentRepository,
    pub equipment_repo: EquipmentRepository,
    pub workflow_repo: WorkflowRepository,
    // Epic 14: IoT & Smart Building
    pub sensor_repo: SensorRepository,
    // Epic 15: Property Listings & Multi-Portal Sync
    pub listing_repo: ListingRepository,
    // Epic 17: Agency & Realtor Management
    pub agency_repo: AgencyRepository,
    // Epic 18: Short-Term Rental Integration
    pub rental_repo: RentalRepository,
    // Epic 19: Lease Management & Tenant Screening
    pub lease_repo: LeaseRepository,
    // Epic 20: Maintenance Scheduling & Work Orders
    pub work_order_repo: WorkOrderRepository,
    // Epic 21: Supplier & Vendor Management
    pub vendor_repo: VendorRepository,
    // Epic 22: Insurance Management
    pub insurance_repo: InsuranceRepository,
    // Epic 23: Emergency Management
    pub emergency_repo: EmergencyRepository,
    // Epic 24: Budget & Financial Planning
    pub budget_repo: BudgetRepository,
    // Epic 25: Legal Document & Compliance
    pub legal_repo: LegalRepository,
    // Epic 26: Platform Subscription & Billing
    pub subscription_repo: SubscriptionRepository,
    // Epic 30: Government Portal Integration
    pub government_portal_repo: GovernmentPortalRepository,
    // Epic 37: Community & Social Features
    pub community_repo: CommunityRepository,
    // Epic 38: Workflow Automation
    pub automation_repo: AutomationRepository,
    // Epic 54: Forms Management
    pub form_repo: FormRepository,
    // Epic 58: Package & Visitor Management
    pub package_visitor_repo: PackageVisitorRepository,
    // Epic 61: External Integrations Suite
    pub integration_repo: IntegrationRepository,
    // Epic 66: Platform Migration & Data Import
    pub migration_repo: MigrationRepository,
    // Epic 65: Energy & Sustainability Tracking
    pub energy_repo: EnergyRepository,
    // Epic 64: Advanced AI & LLM Capabilities
    pub llm_document_repo: LlmDocumentRepository,
    // Epic 57: Pet & Vehicle Registry
    pub registry_repo: RegistryRepository,
    // Epic 73: Infrastructure & Operations
    pub operations_repo: OperationsRepository,
    // Epic 74: Owner Investment Analytics
    pub owner_analytics_repo: OwnerAnalyticsRepository,
    // Epic 77: Dispute Resolution
    pub dispute_repo: DisputeRepository,
    // Epic 71: Background Jobs Infrastructure (Phase 1.3)
    pub background_job_repo: BackgroundJobRepository,
    // Epic 89: Feature Flags & Health Monitoring
    pub infrastructure_repo: InfrastructureRepository,
    // Epic 108: Feature Packages & Bundles
    pub feature_package_repo: FeaturePackageRepository,
    // Epic 109: User Type Feature Experience
    pub feature_analytics_repo: FeatureAnalyticsRepository,
    // UC-12: Utility Outages
    pub outage_repo: OutageRepository,
    // Epic 132: Dynamic Rent Pricing & Market Analytics
    pub market_pricing_repo: MarketPricingRepository,
    pub marketplace_repo: MarketplaceRepository,
    // Epic 133: AI Lease Abstraction & Document Intelligence
    pub lease_abstraction_repo: LeaseAbstractionRepository,
    // Epic 134: Predictive Maintenance & Equipment Intelligence
    pub predictive_maintenance_repo: PredictiveMaintenanceRepository,
    // Epic 135: Enhanced Tenant Screening with AI Risk Scoring
    pub enhanced_tenant_screening_repo: EnhancedTenantScreeningRepository,
    // Epic 136: ESG Reporting Dashboard
    pub esg_reporting_repo: EsgReportingRepository,
    // Epic 137: Smart Building Certification
    pub building_certification_repo: BuildingCertificationRepository,
    // Epic 138: Automated Property Valuation Model (AVM)
    pub property_valuation_repo: PropertyValuationRepository,
    // Epic 139: Investor Portal & ROI Reporting
    pub investor_portal_repo: InvestorPortalRepository,
    // Epic 140: Multi-Property Portfolio Analytics
    pub portfolio_analytics_repo: PortfolioAnalyticsRepository,
    // Epic 141: Reserve Fund Management
    pub reserve_fund_repo: ReserveFundRepository,
    // Epic 142: Violation Tracking & Enforcement
    pub violation_repo: ViolationRepository,
    // Epic 143: Board Meeting Management
    pub board_meeting_repo: BoardMeetingRepository,
    // Epic 144: Portfolio Performance Analytics
    pub portfolio_performance_repo: PortfolioPerformanceRepository,
    // Epic 145: Multi-Currency & Cross-Border Support
    pub multi_currency_repo: MultiCurrencyRepository,
    // Epic 150: API Ecosystem Expansion
    pub api_ecosystem_repo: ApiEcosystemRepository,
    // Epic 67/100: AML/DSA Compliance & Enhanced Due Diligence
    pub edd_repo: EddRepository,
    pub compliance_repo: ComplianceRepository,
    // Epic 72: Regional Legal Compliance (SK/CZ)
    pub regional_compliance_repo: RegionalComplianceRepository,
    // Epic 81: Report Schedule Management & Execution History
    pub report_schedule_repo: ReportScheduleRepository,
    pub accounting_service: AccountingService,
    pub accounting_repo: AccountingRepository,
    pub accounting_provider_repo: AccountingProviderRepository,
    // Epic 91: AI Chat LLM Integration
    pub llm_client: LlmClient,
    pub auth_service: AuthService,
    pub email_service: EmailService,
    /// Epic 2B: notification delivery pipeline (preference routing + transport
    /// adapters + delivery tracking). Wired here so publish paths
    /// (announcements, critical notifications) can fan out instead of logging.
    pub notification_pipeline: NotificationPipeline,
    pub jwt_service: JwtService,
    pub totp_service: TotpService,
    pub oauth_service: OAuthService,
    // Epic 103: Redis Integration
    pub redis_client: Option<RedisClient>,
    pub session_store: Option<SessionStore>,
    pub pubsub_service: Option<PubSubService>,
    /// Test-only sink for realtime preference-sync events (issue #1376).
    ///
    /// `None` in production. Installed by tests via
    /// [`AppState::with_pref_event_recorder`] so the `preference.updated`
    /// publish contract can be asserted in CI without a live Redis daemon.
    pub pref_event_recorder: Option<PreferenceEventRecorder>,
    /// Test-only in-memory OAuth `state` store (issue #2203).
    ///
    /// `None` in production, where single-use CSRF-state enforcement runs
    /// against Redis (`redis_client`). Installed by tests via
    /// [`AppState::with_oauth_state_store`] so the Airbnb-callback consume path
    /// (`ConsumeOutcome::Consumed` / `Rejected → 400 INVALID_STATE`) can be
    /// exercised at the handler level without a live Redis daemon.
    pub oauth_state_store: Option<crate::routes::integrations::oauth_state::OAuthStateStore>,
    // Epic 103: S3 Storage Service
    pub storage_service: Option<StorageService>,
    /// Epic 18 / Story 18.2 (#1687): provider-agnostic guest ID-document OCR
    /// seam. Defaults to the not-configured stub (`501 OCR_NOT_CONFIGURED`);
    /// a real Vision-capable provider is wired here in Stage B without any
    /// route changes.
    pub id_document_ocr: SharedIdDocumentOcr,
    /// Phase 1: Host-resolution cache shared with `host_tenant_middleware`.
    /// Holds the SAME `Arc` the middleware uses, so domain-management handlers
    /// can invalidate entries (e.g. after a domain is verified).
    pub tenant_resolution_cache: std::sync::Arc<api_core::middleware::TenantResolutionCache>,
    /// Phase 5.5: per-tenant rate limiter set shared with `host_tenant_middleware`.
    /// Holds the SAME `Arc` the middleware uses; admin handlers can install
    /// per-tenant overrides via `tenant_rate_limiters.set_override(org, rpm)`
    /// (e.g. after `tenant_settings.rate_limit_rpm` is updated).
    pub tenant_rate_limiters: std::sync::Arc<api_core::middleware::TenantRateLimiterSet>,
    /// Airbnb integration configuration loaded once at startup (issue #711).
    /// Eliminates per-request `std::env::var` reads in Airbnb handlers and
    /// surfaces misconfiguration at boot rather than at runtime.
    pub airbnb_config: AirbnbAppConfig,
    /// Booking.com OAuth integration configuration loaded once at startup
    /// (Coverage 83-2). Handlers fail closed when `client_id.is_empty()`.
    pub booking_config: BookingOAuthAppConfig,
    /// Stripe Checkout configuration loaded once at startup (Story 11.5,
    /// [BIT-181]). Handlers fail closed when `secret_key`/`webhook_secret`
    /// are empty.
    pub stripe_config: StripeAppConfig,
    /// Portal inbound-webhook configuration loaded once at startup. The
    /// connection-scoped portal webhook receiver fails closed when
    /// `webhook_secret` is empty and verifies the `X-Webhook-Signature` HMAC
    /// over the raw body before acting.
    pub portal_config: PortalAppConfig,
    /// Trusted reverse-proxy allowlist loaded once at startup from
    /// `TRUSTED_PROXY_CIDRS` (issue #2789). Gates whether `X-Forwarded-For` /
    /// `CF-Connecting-IP` are believed when resolving a request's client IP for
    /// the share-access audit log and the `token:ip` brute-force throttle:
    /// forwarding headers are only trusted when the socket peer is in this set,
    /// so a directly-reachable client can no longer spoof its source IP.
    pub trusted_proxies: crate::client_ip::TrustedProxies,
}

impl AppState {
    /// Create a new AppState.
    pub fn new(
        db: DbPool,
        email_service: EmailService,
        jwt_service: JwtService,
        tenant_resolution_cache: std::sync::Arc<api_core::middleware::TenantResolutionCache>,
        tenant_rate_limiters: std::sync::Arc<api_core::middleware::TenantRateLimiterSet>,
    ) -> Self {
        let user_repo = UserRepository::new(db.clone());
        let session_repo = SessionRepository::new(db.clone());
        let password_reset_repo = PasswordResetRepository::new(db.clone());
        let org_repo = OrganizationRepository::new(db.clone());
        let org_member_repo = OrganizationMemberRepository::new(db.clone());
        let role_repo = RoleRepository::new(db.clone());
        let building_repo = BuildingRepository::new(db.clone());
        let geocoding = GeocodingService::from_env();
        let unit_repo = UnitRepository::new(db.clone());
        let unit_resident_repo = UnitResidentRepository::new(db.clone());
        let delegation_repo = DelegationRepository::new(db.clone());
        let person_month_repo = PersonMonthRepository::new(db.clone());
        let facility_repo = FacilityRepository::new(db.clone());
        let fault_repo = FaultRepository::new(db.clone());
        let vote_repo = VoteRepository::new(db.clone());
        let announcement_repo = AnnouncementRepository::new(db.clone());
        let document_repo = DocumentRepository::new(db.clone());
        let document_template_repo = DocumentTemplateRepository::new(db.clone());
        let notification_pref_repo = NotificationPreferenceRepository::new(db.clone());
        let device_push_token_repo = DevicePushTokenRepository::new(db.clone());
        let critical_notification_repo = CriticalNotificationRepository::new(db.clone());
        let two_factor_repo = TwoFactorAuthRepository::new(db.clone());
        let audit_log_repo = AuditLogRepository::new(db.clone());
        let data_export_repo = DataExportRepository::new(db.clone());
        let data_residency_repo = DataResidencyRepository::new(db.clone());
        let oauth_repo = OAuthRepository::new(db.clone());
        let oauth_token_event_repo = OAuthTokenEventRepository::new(db.clone());
        let platform_admin_repo = PlatformAdminRepository::new(db.clone());
        let feature_flag_repo = FeatureFlagRepository::new(db.clone());
        let granular_notification_repo = GranularNotificationRepository::new(db.clone());
        let health_monitoring_repo = HealthMonitoringRepository::new(db.clone());
        let system_announcement_repo = SystemAnnouncementRepository::new(db.clone());
        let onboarding_repo = OnboardingRepository::new(db.clone());
        let help_repo = HelpRepository::new(db.clone());
        let signature_request_repo = SignatureRequestRepository::new(db.clone());
        let e_signature_nonce_repo = ESignatureNonceRepository::new(db.clone());
        let financial_repo = FinancialRepository::new(db.clone());
        let meter_repo = MeterRepository::new(db.clone());
        // Epic 13: AI Assistant & Automation
        let ai_chat_repo = AiChatRepository::new(db.clone());
        let sentiment_repo = SentimentRepository::new(db.clone());
        let equipment_repo = EquipmentRepository::new(db.clone());
        let workflow_repo = WorkflowRepository::new(db.clone());
        // Epic 14: IoT & Smart Building
        let sensor_repo = SensorRepository::new();
        // Epic 15: Property Listings & Multi-Portal Sync
        let listing_repo = ListingRepository::new(db.clone());
        // Epic 17: Agency & Realtor Management
        let agency_repo = AgencyRepository::new(db.clone());
        // Epic 18: Short-Term Rental Integration
        let rental_repo = RentalRepository::new(db.clone());
        // Epic 19: Lease Management & Tenant Screening
        let lease_repo = LeaseRepository::new(db.clone());
        // Epic 20: Maintenance Scheduling & Work Orders
        let work_order_repo = WorkOrderRepository::new(db.clone());
        // Epic 21: Supplier & Vendor Management
        let vendor_repo = VendorRepository::new(db.clone());
        // Epic 22: Insurance Management
        let insurance_repo = InsuranceRepository::new(db.clone());
        // Epic 23: Emergency Management
        let emergency_repo = EmergencyRepository::new(db.clone());
        // Epic 24: Budget & Financial Planning
        let budget_repo = BudgetRepository::new(db.clone());
        // Epic 25: Legal Document & Compliance
        let legal_repo = LegalRepository::new(db.clone());
        // Epic 26: Platform Subscription & Billing
        let subscription_repo = SubscriptionRepository::new(db.clone());
        // Epic 30: Government Portal Integration
        let government_portal_repo = GovernmentPortalRepository::new(db.clone());
        // Epic 37: Community & Social Features
        let community_repo = CommunityRepository::new(db.clone());
        // Epic 38: Workflow Automation
        let automation_repo = AutomationRepository::new(db.clone());
        // Epic 54: Forms Management
        let form_repo = FormRepository::new(db.clone());
        // Epic 58: Package & Visitor Management
        let package_visitor_repo = PackageVisitorRepository::new(db.clone());
        // Epic 61: External Integrations Suite
        let integration_repo = IntegrationRepository::new(db.clone());
        // Epic 66: Platform Migration & Data Import
        let migration_repo = MigrationRepository::new(db.clone());
        // Epic 65: Energy & Sustainability Tracking
        let energy_repo = EnergyRepository::new(db.clone());
        // Epic 64: Advanced AI & LLM Capabilities
        let llm_document_repo = LlmDocumentRepository::new(db.clone());
        // Epic 57: Pet & Vehicle Registry
        let registry_repo = RegistryRepository::new(db.clone());
        // Epic 73: Infrastructure & Operations
        let operations_repo = OperationsRepository::new(db.clone());
        // Epic 74: Owner Investment Analytics
        let owner_analytics_repo = OwnerAnalyticsRepository::new(db.clone());
        // Epic 77: Dispute Resolution
        let dispute_repo = DisputeRepository::new(db.clone());
        // Epic 71: Background Jobs Infrastructure (Phase 1.3)
        let background_job_repo = BackgroundJobRepository::new(db.clone());
        // Epic 89: Feature Flags & Health Monitoring
        let infrastructure_repo = InfrastructureRepository::new(db.clone());
        // Epic 108: Feature Packages & Bundles
        let feature_package_repo = FeaturePackageRepository::new(db.clone());
        // Epic 109: User Type Feature Experience
        let feature_analytics_repo = FeatureAnalyticsRepository::new(db.clone());
        // UC-12: Utility Outages
        let outage_repo = OutageRepository::new(db.clone());
        // Epic 132: Dynamic Rent Pricing & Market Analytics
        let market_pricing_repo = MarketPricingRepository::new(db.clone());
        let marketplace_repo = MarketplaceRepository::new(db.clone());
        // Epic 133: AI Lease Abstraction & Document Intelligence
        let lease_abstraction_repo = LeaseAbstractionRepository::new(db.clone());
        // Epic 134: Predictive Maintenance & Equipment Intelligence
        let predictive_maintenance_repo = PredictiveMaintenanceRepository::new(db.clone());
        // Epic 135: Enhanced Tenant Screening with AI Risk Scoring
        let enhanced_tenant_screening_repo = EnhancedTenantScreeningRepository::new(db.clone());
        // Epic 136: ESG Reporting Dashboard
        let esg_reporting_repo = EsgReportingRepository::new(db.clone());
        // Epic 137: Smart Building Certification
        let building_certification_repo = BuildingCertificationRepository::new(db.clone());
        // Epic 138: Automated Property Valuation Model (AVM)
        let property_valuation_repo = PropertyValuationRepository::new(db.clone());
        // Epic 139: Investor Portal & ROI Reporting
        let investor_portal_repo = InvestorPortalRepository::new(db.clone());
        // Epic 140: Multi-Property Portfolio Analytics
        let portfolio_analytics_repo = PortfolioAnalyticsRepository::new(db.clone());
        // Epic 141: Reserve Fund Management
        let reserve_fund_repo = ReserveFundRepository::new();
        // Epic 142: Violation Tracking & Enforcement
        let violation_repo = ViolationRepository::new(db.clone());
        // Epic 143: Board Meeting Management
        let board_meeting_repo = BoardMeetingRepository::new(db.clone());
        // Epic 144: Portfolio Performance Analytics
        let portfolio_performance_repo = PortfolioPerformanceRepository::new(db.clone());
        // Epic 145: Multi-Currency & Cross-Border Support
        let multi_currency_repo = MultiCurrencyRepository::new(db.clone());
        // Epic 150: API Ecosystem Expansion
        let api_ecosystem_repo = ApiEcosystemRepository::new(db.clone());
        // Epic 67/100: AML/DSA Compliance & Enhanced Due Diligence
        let edd_repo = EddRepository::new(db.clone());
        let compliance_repo = ComplianceRepository::new(db.clone());
        // Epic 72: Regional Legal Compliance (SK/CZ)
        let regional_compliance_repo = RegionalComplianceRepository::new(db.clone());
        // Epic 81: Report Schedule Management & Execution History
        let report_schedule_repo = ReportScheduleRepository::new(db.clone());
        let accounting_repo = AccountingRepository::new(db.clone());
        let accounting_provider_repo = AccountingProviderRepository::new(db.clone());
        // Epic 91: AI Chat LLM Integration
        let llm_client = LlmClient::new();
        let auth_service = AuthService::new();
        let accounting_service = AccountingService::new(accounting_repo.clone());
        let totp_service = TotpService::new("Property Management".to_string());
        // Wire the token-usage analytics recorder into the OAuth service so the
        // issuance / refresh / revocation paths emit best-effort events
        // (Epic 10A, #2628). Recording never fails the token flow.
        let oauth_service =
            OAuthService::new(oauth_repo.clone(), user_repo.clone(), auth_service.clone())
                .with_token_event_repo(oauth_token_event_repo.clone());

        // Epic 2B: build the notification pipeline from shared resources.
        // `pubsub` is `None` here — Redis is wired post-construction (see
        // `with_redis`), and in-app delivery (the mandatory per-recipient DB
        // record) does not depend on it; only the real-time WebSocket fan-out
        // (Story 8A.3) does. FCM is read from env: when unconfigured the push
        // channel fails closed to `Skipped` rather than a fake `Sent`.
        let notification_pipeline = NotificationPipeline::new(
            db.clone(),
            email_service.clone(),
            None,
            PipelineConfig::default(),
        );

        Self {
            boot_time: Instant::now(),
            db,
            user_repo,
            session_repo,
            password_reset_repo,
            org_repo,
            org_member_repo,
            role_repo,
            building_repo,
            geocoding,
            unit_repo,
            unit_resident_repo,
            delegation_repo,
            person_month_repo,
            facility_repo,
            fault_repo,
            vote_repo,
            announcement_repo,
            document_repo,
            document_template_repo,
            notification_pref_repo,
            device_push_token_repo,
            critical_notification_repo,
            two_factor_repo,
            audit_log_repo,
            data_export_repo,
            data_residency_repo,
            oauth_repo,
            oauth_token_event_repo,
            platform_admin_repo,
            feature_flag_repo,
            granular_notification_repo,
            health_monitoring_repo,
            system_announcement_repo,
            onboarding_repo,
            help_repo,
            signature_request_repo,
            e_signature_nonce_repo,
            financial_repo,
            meter_repo,
            ai_chat_repo,
            sentiment_repo,
            equipment_repo,
            workflow_repo,
            sensor_repo,
            listing_repo,
            agency_repo,
            rental_repo,
            lease_repo,
            work_order_repo,
            vendor_repo,
            insurance_repo,
            emergency_repo,
            budget_repo,
            legal_repo,
            subscription_repo,
            government_portal_repo,
            community_repo,
            automation_repo,
            form_repo,
            package_visitor_repo,
            integration_repo,
            migration_repo,
            energy_repo,
            llm_document_repo,
            registry_repo,
            operations_repo,
            owner_analytics_repo,
            dispute_repo,
            background_job_repo,
            infrastructure_repo,
            feature_package_repo,
            feature_analytics_repo,
            outage_repo,
            market_pricing_repo,
            marketplace_repo,
            lease_abstraction_repo,
            predictive_maintenance_repo,
            enhanced_tenant_screening_repo,
            esg_reporting_repo,
            building_certification_repo,
            property_valuation_repo,
            investor_portal_repo,
            portfolio_analytics_repo,
            reserve_fund_repo,
            violation_repo,
            board_meeting_repo,
            portfolio_performance_repo,
            multi_currency_repo,
            api_ecosystem_repo,
            edd_repo,
            compliance_repo,
            regional_compliance_repo,
            report_schedule_repo,
            accounting_service,
            accounting_repo,
            accounting_provider_repo,
            llm_client,
            auth_service,
            email_service,
            notification_pipeline,
            jwt_service,
            totp_service,
            oauth_service,
            // Epic 103: Redis services (initialized separately if available)
            redis_client: None,
            session_store: None,
            pubsub_service: None,
            // Issue #1376: test-only preference-sync recorder (None in prod).
            pref_event_recorder: None,
            // Issue #2203: test-only OAuth state store (None in prod; Redis is
            // authoritative). Installed by tests via `with_oauth_state_store`.
            oauth_state_store: None,
            // Epic 103: S3 Storage Service
            storage_service: None,
            // Story 18.2 (#1687): default guest ID-document OCR = not-configured
            // stub; replaced with a real provider in Stage B.
            id_document_ocr: default_id_document_ocr(),
            // Phase 1: shared host-resolution cache
            tenant_resolution_cache,
            // Phase 5.5: shared per-tenant rate limiter set (defense leak #15)
            tenant_rate_limiters,
            // Issue #711: Airbnb integration env vars cached at startup so
            // handlers never call `std::env::var` per request.
            airbnb_config: AirbnbAppConfig::from_env(),
            // Coverage 83-2: Booking.com OAuth env vars cached at startup.
            booking_config: BookingOAuthAppConfig::from_env(),
            // Story 11.5 (BIT-181): Stripe Checkout env vars cached at startup.
            stripe_config: StripeAppConfig::from_env(),
            // Portal inbound-webhook signing secret cached at startup so the
            // connection-scoped receiver can fail closed and verify signatures
            // without a per-request env read.
            portal_config: PortalAppConfig::from_env(),
            // Issue #2789: trusted reverse-proxy allowlist cached at startup so
            // client-IP resolution can gate forwarding-header trust on the
            // socket peer without a per-request env read.
            trusted_proxies: crate::client_ip::TrustedProxies::from_env(),
        }
    }

    /// Set a custom pub/sub service (Story 1376).
    pub fn with_pubsub(mut self, pubsub_service: PubSubService) -> Self {
        self.pubsub_service = Some(pubsub_service);
        self
    }

    /// Set Redis client and derived services (Epic 103).
    ///
    /// Call this after creating the AppState if Redis is available.
    pub fn with_redis(mut self, redis_client: RedisClient) -> Self {
        let session_store = SessionStore::new(redis_client.clone());
        let pubsub_service = PubSubService::new(redis_client.clone());

        self.redis_client = Some(redis_client);
        self.session_store = Some(session_store);
        self.pubsub_service = Some(pubsub_service);
        self
    }

    /// Install a [`PreferenceEventRecorder`] and return the recorder handle
    /// (issue #1376).
    ///
    /// Test-only: lets a CI test capture the `preference.updated` events the
    /// notification-preference handler would publish, without a live Redis.
    /// The returned handle shares the same backing store as the one stored on
    /// the state, so the test can `drain()` it after issuing a PATCH.
    pub fn with_pref_event_recorder(mut self) -> (Self, PreferenceEventRecorder) {
        let recorder = PreferenceEventRecorder::new();
        self.pref_event_recorder = Some(recorder.clone());
        (self, recorder)
    }

    /// Install a test-only in-memory OAuth `state` store and return its handle
    /// (issue #2203).
    ///
    /// Test-only: lets a CI integration test seed a freshly-issued single-use
    /// state and then drive the Airbnb-callback consume path
    /// (`ConsumeOutcome::Consumed` on first use, `Rejected → 400 INVALID_STATE`
    /// on replay) without a live Redis. The returned handle shares the same
    /// backing map as the one stored on the state, so the test can `seed(...)`
    /// before issuing the callback request. `None` in production.
    pub fn with_oauth_state_store(
        mut self,
    ) -> (
        Self,
        crate::routes::integrations::oauth_state::OAuthStateStore,
    ) {
        let store = crate::routes::integrations::oauth_state::OAuthStateStore::new();
        self.oauth_state_store = Some(store.clone());
        (self, store)
    }

    /// Set S3 storage service (Epic 103).
    ///
    /// Call this after creating the AppState if S3 is configured.
    pub fn with_storage(mut self, storage_service: StorageService) -> Self {
        self.storage_service = Some(storage_service);
        self
    }

    /// Install a custom guest ID-document OCR provider (Story 18.2, #1687).
    ///
    /// Production wires the not-configured stub by default; Stage B swaps in a
    /// real Vision-capable provider here, and tests can inject a fake.
    pub fn with_id_document_ocr(mut self, ocr: SharedIdDocumentOcr) -> Self {
        self.id_document_ocr = ocr;
        self
    }

    /// Override the Airbnb integration configuration (test seam, issue #2240).
    ///
    /// Production loads `airbnb_config` from the environment in
    /// [`AppState::new`]. Integration tests use this to inject non-empty
    /// credentials plus an `api_base` pointing at a `wiremock` stub, so the
    /// `direct_connect_airbnb` success/write path can be exercised network-free
    /// without setting process-global `AIRBNB_*` env vars (which would race the
    /// other `#[sqlx::test]` cases sharing the same test binary).
    pub fn with_airbnb_config(mut self, config: AirbnbAppConfig) -> Self {
        self.airbnb_config = config;
        self
    }
}

// SECURITY: Implement TenantMembershipProvider to enable ValidatedTenantExtractor
impl TenantMembershipProvider for AppState {
    fn db_pool(&self) -> &DbPool {
        &self.db
    }
}
