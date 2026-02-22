//! Application state.

use std::time::Instant;

use crate::services::{AuthService, EmailService, JwtService, OAuthService, TotpService};
use api_core::TenantMembershipProvider;
use db::{
    repositories::{
        AgencyRepository, AiChatRepository, AnnouncementRepository, ApiEcosystemRepository,
        AuditLogRepository, AutomationRepository, BackgroundJobRepository, BoardMeetingRepository,
        BudgetRepository, BuildingCertificationRepository, BuildingRepository, CommunityRepository,
        CriticalNotificationRepository, DataExportRepository, DelegationRepository,
        DisputeRepository, DocumentRepository, DocumentTemplateRepository, EmergencyRepository,
        EnergyRepository, EnhancedTenantScreeningRepository, EquipmentRepository,
        EsgReportingRepository, FacilityRepository, FaultRepository, FeatureAnalyticsRepository,
        FeatureFlagRepository, FeaturePackageRepository, FinancialRepository, FormRepository,
        GovernmentPortalRepository, GranularNotificationRepository, HealthMonitoringRepository,
        HelpRepository, InfrastructureRepository, InsuranceRepository, IntegrationRepository,
        InvestorPortalRepository, LeaseAbstractionRepository, LeaseRepository, LegalRepository,
        ListingRepository, LlmDocumentRepository, MarketPricingRepository, MeterRepository,
        MultiCurrencyRepository, NotificationPreferenceRepository, OAuthRepository,
        OnboardingRepository, OperationsRepository, OrganizationMemberRepository,
        OrganizationRepository, OutageRepository, OwnerAnalyticsRepository,
        PackageVisitorRepository, PasswordResetRepository, PersonMonthRepository,
        PlatformAdminRepository, PortfolioAnalyticsRepository, PortfolioPerformanceRepository,
        PredictiveMaintenanceRepository, PropertyValuationRepository, RegistryRepository,
        RentalRepository, ReserveFundRepository, RoleRepository, SensorRepository,
        SentimentRepository, SessionRepository, SignatureRequestRepository, SubscriptionRepository,
        SystemAnnouncementRepository, TwoFactorAuthRepository, UnitRepository,
        UnitResidentRepository, UserRepository, VendorRepository, ViolationRepository,
        VoteRepository, WorkOrderRepository, WorkflowRepository,
    },
    DbPool,
};
use integrations::{LlmClient, PubSubService, RedisClient, SessionStore, StorageService};

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
    pub critical_notification_repo: CriticalNotificationRepository,
    pub two_factor_repo: TwoFactorAuthRepository,
    pub audit_log_repo: AuditLogRepository,
    pub data_export_repo: DataExportRepository,
    pub oauth_repo: OAuthRepository,
    pub platform_admin_repo: PlatformAdminRepository,
    pub feature_flag_repo: FeatureFlagRepository,
    pub granular_notification_repo: GranularNotificationRepository,
    pub health_monitoring_repo: HealthMonitoringRepository,
    pub system_announcement_repo: SystemAnnouncementRepository,
    pub onboarding_repo: OnboardingRepository,
    pub help_repo: HelpRepository,
    pub signature_request_repo: SignatureRequestRepository,
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
    // Epic 91: AI Chat LLM Integration
    pub llm_client: LlmClient,
    pub auth_service: AuthService,
    pub email_service: EmailService,
    pub jwt_service: JwtService,
    pub totp_service: TotpService,
    pub oauth_service: OAuthService,
    // Epic 103: Redis Integration
    pub redis_client: Option<RedisClient>,
    pub session_store: Option<SessionStore>,
    pub pubsub_service: Option<PubSubService>,
    // Epic 103: S3 Storage Service
    pub storage_service: Option<StorageService>,
}

impl AppState {
    /// Create a new AppState.
    pub fn new(db: DbPool, email_service: EmailService, jwt_service: JwtService) -> Self {
        let user_repo = UserRepository::new(db.clone());
        let session_repo = SessionRepository::new(db.clone());
        let password_reset_repo = PasswordResetRepository::new(db.clone());
        let org_repo = OrganizationRepository::new(db.clone());
        let org_member_repo = OrganizationMemberRepository::new(db.clone());
        let role_repo = RoleRepository::new(db.clone());
        let building_repo = BuildingRepository::new(db.clone());
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
        let critical_notification_repo = CriticalNotificationRepository::new(db.clone());
        let two_factor_repo = TwoFactorAuthRepository::new(db.clone());
        let audit_log_repo = AuditLogRepository::new(db.clone());
        let data_export_repo = DataExportRepository::new(db.clone());
        let oauth_repo = OAuthRepository::new(db.clone());
        let platform_admin_repo = PlatformAdminRepository::new(db.clone());
        let feature_flag_repo = FeatureFlagRepository::new(db.clone());
        let granular_notification_repo = GranularNotificationRepository::new(db.clone());
        let health_monitoring_repo = HealthMonitoringRepository::new(db.clone());
        let system_announcement_repo = SystemAnnouncementRepository::new(db.clone());
        let onboarding_repo = OnboardingRepository::new(db.clone());
        let help_repo = HelpRepository::new(db.clone());
        let signature_request_repo = SignatureRequestRepository::new(db.clone());
        let financial_repo = FinancialRepository::new(db.clone());
        let meter_repo = MeterRepository::new(db.clone());
        // Epic 13: AI Assistant & Automation
        let ai_chat_repo = AiChatRepository::new(db.clone());
        let sentiment_repo = SentimentRepository::new(db.clone());
        let equipment_repo = EquipmentRepository::new(db.clone());
        let workflow_repo = WorkflowRepository::new(db.clone());
        // Epic 14: IoT & Smart Building
        let sensor_repo = SensorRepository::new(db.clone());
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
        let reserve_fund_repo = ReserveFundRepository::new(db.clone());
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
        // Epic 91: AI Chat LLM Integration
        let llm_client = LlmClient::new();
        let auth_service = AuthService::new();
        let totp_service = TotpService::new("Property Management".to_string());
        let oauth_service = OAuthService::new(oauth_repo.clone(), auth_service.clone());

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
            critical_notification_repo,
            two_factor_repo,
            audit_log_repo,
            data_export_repo,
            oauth_repo,
            platform_admin_repo,
            feature_flag_repo,
            granular_notification_repo,
            health_monitoring_repo,
            system_announcement_repo,
            onboarding_repo,
            help_repo,
            signature_request_repo,
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
            llm_client,
            auth_service,
            email_service,
            jwt_service,
            totp_service,
            oauth_service,
            // Epic 103: Redis services (initialized separately if available)
            redis_client: None,
            session_store: None,
            pubsub_service: None,
            // Epic 103: S3 Storage Service
            storage_service: None,
        }
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

    /// Set S3 storage service (Epic 103).
    ///
    /// Call this after creating the AppState if S3 is configured.
    pub fn with_storage(mut self, storage_service: StorageService) -> Self {
        self.storage_service = Some(storage_service);
        self
    }
}

// SECURITY: Implement TenantMembershipProvider to enable ValidatedTenantExtractor
impl TenantMembershipProvider for AppState {
    fn db_pool(&self) -> &DbPool {
        &self.db
    }
}
