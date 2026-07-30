//! Database models.

pub mod accounting;
// ACC Invoicing/Accounting MVP — Foundation-owned model files (EPIC-ACC-02/03/04/05/16).
pub mod acc_catalog;
pub mod acc_config;
pub mod acc_contacts_ext;
pub mod acc_invoicing_ext;
pub mod acc_platform;
pub mod accounting_provider;
pub mod agency_branding;
pub mod agency_domain;
pub mod announcement;
pub mod audit_log;
pub mod building;
pub mod critical_notification;
pub mod data_export;
pub mod delegation;
pub mod document;
pub mod document_template;
pub mod facility;
pub mod fault;
pub mod financial;
pub mod layout;

pub mod device_push_token;
pub mod granular_notification;
pub mod membership;
pub mod messaging;
pub mod meter;
pub mod notification_preference;
pub mod oauth;
pub mod oauth_token_event;
pub mod organization;
pub mod organization_member;
pub mod password_reset;
pub mod person_month;
pub mod platform_admin;
pub mod portal_password_reset;
pub mod refresh_token;
pub mod role;
pub mod signature_request;
pub mod two_factor_auth;
pub mod unit;
pub mod unit_resident;
pub mod user;
pub mod vote;

// Epic 13: AI Assistant & Automation
pub mod ai_chat;
pub mod equipment;
pub mod sentiment;
pub mod workflow;
pub mod workflow_templates;

// Epic 14: IoT & Smart Building
pub mod sensor;

// Epic 15: Property Listings & Multi-Portal Sync
pub mod listing;

// Epic 16: Portal Search & Discovery
pub mod portal;

// Epic 17: Agency & Realtor Management
pub mod agency;

// Epic 18: Short-Term Rental Integration
pub mod rental;

pub use ai_chat::{
    message_role, AiChatMessage, AiChatSession, AiResponse, AiSource, AiTrainingFeedback,
    ChatSessionSummary, CreateChatSession, ProvideFeedback, SendChatMessage,
};
pub use equipment::{
    equipment_status, maintenance_status, maintenance_type, CreateEquipment, CreateMaintenance,
    Equipment, EquipmentMaintenance, EquipmentQuery, EquipmentWithSummary, MaintenancePrediction,
    UpdateEquipment, UpdateMaintenance,
};
pub use sensor::{
    sensor_status, sensor_type, AggregatedReading, AlertQuery, BatchSensorReadings, CreateSensor,
    CreateSensorAlert, CreateSensorFaultCorrelation, CreateSensorReading, CreateSensorThreshold,
    ReadingQuery, Sensor, SensorAlert, SensorDashboard, SensorFaultCorrelation, SensorQuery,
    SensorReading, SensorSummary, SensorThreshold, SensorThresholdTemplate, SensorTypeCount,
    SingleReading, UpdateSensor, UpdateSensorThreshold,
};
pub use sentiment::{
    alert_type, BuildingSentiment, CreateSentimentAlert, SentimentAlert, SentimentDashboard,
    SentimentThresholds, SentimentTrend, SentimentTrendQuery, UpdateSentimentThresholds,
    UpsertSentimentTrend,
};
pub use workflow::{
    action_type, execution_status, on_failure, step_status, trigger_type, CreateWorkflow,
    CreateWorkflowAction, ExecutionQuery, TriggerWorkflow, UpdateWorkflow, Workflow,
    WorkflowAction, WorkflowExecution, WorkflowExecutionStep, WorkflowQuery, WorkflowSchedule,
    WorkflowSummary, WorkflowWithDetails,
};
pub use workflow_templates::{
    get_builtin_templates, template_category, template_scope, CreateTemplateAction,
    CreateTemplateVariable, CreateWorkflowTemplate, ImportTemplateRequest, RateTemplateRequest,
    TemplateSearchQuery, UpdateWorkflowTemplate, WorkflowTemplate, WorkflowTemplateAction,
    WorkflowTemplateRating, WorkflowTemplateSummary, WorkflowTemplateVariable,
    WorkflowTemplateWithDetails,
};

// Note: `AgencyBranding` from `agency_branding` is intentionally NOT re-exported here
// because `agency::AgencyBranding` already occupies that name at this scope. The new
// per-tenant branding type is consumed via the module path: `crate::models::agency_branding::AgencyBranding`.
pub use agency_branding::UpdateOrganizationBranding;
pub use agency_domain::{
    AgencyDomain, AgencyDomainKind, AgencyDomainVerificationState, CreateAgencyDomain,
};

pub use accounting::{
    BankStatement, BankStatementLine, Contact, CreateInvoice as CreateAccountingInvoice,
    CreateInvoiceItem as CreateAccountingInvoiceItem, Invoice as AccountingInvoice,
    InvoiceItem as AccountingInvoiceItem, InvoiceStatus as AccountingInvoiceStatus, PaymentMatch,
    PaymentMatchState, UpdateInvoice, UpdateInvoiceItem, VatRate,
};
pub use accounting_provider::{
    AccountingProviderAuthFlow, AccountingProviderConnection, AccountingProviderContact,
    AccountingProviderIssuedInvoice, AccountingProviderPaymentMatchSnapshot,
    AccountingProviderSyncCursor,
};

// ACC Invoicing/Accounting MVP — re-exports (EPIC-ACC-02/03/04/05/16).
pub use acc_catalog::{AccCatalogItem, AccItemCategory, AccPriceLevel};
pub use acc_config::{
    AccBankAccount, AccCompanySettings, AccDocumentTemplate, AccEmailTemplate, AccNumberingSeries,
    AccUnit, AccVatRate,
};
pub use acc_contacts_ext::{AccContactAddress, AccContactExt, AccContactTag};
pub use acc_invoicing_ext::{AccAttachment, AccDocType, AccDocumentLink, AccInvoiceExt};
pub use acc_platform::{AccAuditLog, AccShareLink, AccTag, AccTwoFactor};
pub use announcement::{
    announcement_status, target_type, AcknowledgeAnnouncement, AcknowledgmentStats, Announcement,
    AnnouncementAttachment, AnnouncementComment, AnnouncementListQuery, AnnouncementRead,
    AnnouncementStatistics, AnnouncementSummary, AnnouncementWithDetails, CommentWithAuthor,
    CommentWithAuthorRow, CreateAnnouncement, CreateAnnouncementAttachment, CreateComment,
    DeleteComment, MarkAnnouncementRead, PinAnnouncement, PublishAnnouncement, UpdateAnnouncement,
    UserAcknowledgmentStatus,
};
pub use audit_log::{
    ActionCount, AuditAction, AuditLog, AuditLogQuery, AuditLogSummary, CreateAuditLog,
};
pub use building::{
    building_status, Building, BuildingContact, BuildingStatistics, BuildingSummary,
    CreateBuilding, UpdateBuilding,
};
pub use critical_notification::{
    AcknowledgeCriticalNotificationResponse, CreateCriticalNotificationRequest,
    CreateCriticalNotificationResponse, CriticalNotification, CriticalNotificationAcknowledgment,
    CriticalNotificationResponse, CriticalNotificationStats, UnacknowledgedNotificationsResponse,
};
pub use data_export::{
    CreateDataExportRequest, DataExportRequest, DataExportRequestResponse, DataExportStatus,
    DataExportStatusResponse, ExportCategories, ExportCategory, ExportFormat, UserDataExport,
};
pub use delegation::{
    delegation_scope, delegation_status, AcceptDelegation, CreateDelegation, DeclineDelegation,
    Delegation, DelegationAuditLog, DelegationSummary, DelegationWithUsers, RevokeDelegation,
    UpdateDelegation,
};
pub use device_push_token::{
    DevicePushToken, PushPlatform, PushTokenResponse, RegisterPushTokenRequest,
};
pub use document::{
    access_scope, document_category, ocr_status, share_type, ClassificationFeedback,
    CreateDocument, CreateDocumentVersion, CreateFolder, CreateShare, CreateVersionResponse,
    Document, DocumentClassificationHistory, DocumentFolder, DocumentIntelligenceStats,
    DocumentListQuery, DocumentOcrQueue, DocumentSearchRequest, DocumentSearchResponse,
    DocumentSearchResult, DocumentShare, DocumentSummarizationQueue, DocumentSummary,
    DocumentVersion, DocumentVersionHistory, DocumentWithDetails, DocumentWithIntelligence,
    FolderTreeNode, FolderWithCount, GenerateSummaryRequest, LogShareAccess, MoveDocument,
    RestoreVersionRequest, RestoreVersionResponse, RevokeShare, ShareAccessLog, ShareWithDocument,
    UpdateDocument, UpdateFolder, ALLOWED_MIME_TYPES, MAX_FILE_SIZE,
};
pub use document_template::{
    placeholder_type, template_type, CreateTemplate, DocumentTemplate, GenerateDocumentRequest,
    GenerateDocumentResponse, TemplateListQuery, TemplatePlaceholder, TemplateSummary,
    TemplateWithDetails, UpdateTemplate,
};
pub use facility::{
    booking_status, facility_type, ApproveBooking, AvailableSlot, BookingWithDetails,
    CancelBooking, CreateFacility, CreateFacilityBooking, Facility, FacilityBooking,
    FacilitySummary, RejectBooking, UpdateFacility, UpdateFacilityBooking,
};
pub use fault::{
    fault_category, fault_priority, fault_status, timeline_action, AddFaultComment, AddWorkNote,
    AiSuggestion, AssignFault, CategoryCount, ConfirmFault, CreateFault, CreateFaultAttachment,
    CreateFaultTimelineEntry, Fault, FaultAttachment, FaultListQuery, FaultStatistics,
    FaultSummary, FaultTimelineEntry, FaultTimelineEntryWithUser, FaultWithDetails, PriorityCount,
    ReopenFault, ResolveFault, StatusCount, TriageFault, UpdateFault, UpdateFaultStatus,
};
pub use financial::{
    ARReportEntry, ARReportTotals, AccountTransaction, AccountsReceivableReport, BalanceSheetLine,
    BalanceSheetReport, CashFlowLine, CashFlowReport, CreateFeeSchedule, CreateFinancialAccount,
    CreateInvoice, CreateInvoiceItem, CreateTransaction, FeeFrequency, FeeSchedule,
    FinancialAccount, FinancialAccountResponse, FinancialAccountType, IncomeStatement,
    IncomeStatementLine, InitiatePaymentResponse, Invoice, InvoiceItem, InvoiceResponse,
    InvoiceStatus, LateFeeConfig, ListInvoicesResponse, OnlinePaymentSession, Payment,
    PaymentAllocation, PaymentMethod, PaymentResponse, PaymentStatus, RecordPayment,
    ReminderSchedule, TransactionCategory, TransactionType, UnitCreditBalance, UnitFee,
};
pub use granular_notification::{
    AddToGroupRequest, CategorySummary, CreateHeldNotification, DigestNotification,
    EventNotificationPreference, EventPreferenceWithDetails, EventPreferencesResponse,
    GenerateDigestRequest, GroupedNotification, GroupedNotificationsResponse, HeldNotification,
    NotificationDigest, NotificationEventCategory, NotificationEventType, NotificationGroup,
    NotificationGroupWithNotifications, NotificationSchedule, NotificationScheduleResponse,
    RoleDefaultsListResponse, RoleNotificationDefaults, UpdateEventPreferenceRequest,
    UpdateNotificationScheduleRequest, UpdateRoleDefaultsRequest,
};
pub use messaging::{
    BlockWithUserInfo, BlockWithUserInfoRow, CreateBlock, CreateMessage, CreateMessageAttachment,
    CreateThread, Message, MessageAttachment, MessagePreview, MessageThread, MessageWithSender,
    MessageWithSenderRow, ParticipantInfo, ThreadWithPreview, ThreadWithPreviewRow, UserBlock,
};
pub use meter::{
    ConsumptionAggregate, ConsumptionComparison, ConsumptionDataPoint, ConsumptionHistoryResponse,
    CreateSubmissionWindow, CreateUtilityBill, DistributeUtilityBill, DistributionMethod,
    IngestSmartMeterReading, ListMetersResponse, ListReadingsResponse, Meter, MeterReading,
    MeterResponse, MeterType, MissingReadingAlert, ReadingApprovalEntry, ReadingSource,
    ReadingStatus, ReadingSubmissionWindow, ReadingValidationRule, RegisterMeter, ReplaceMeter,
    SmartMeterProvider, SmartMeterReadingLog, SubmitReading, UnitDistributionOverride, UtilityBill,
    UtilityBillDistribution, UtilityBillResponse, ValidateReading,
};
pub use notification_preference::{
    DisableAllWarningResponse, NotificationChannel, NotificationPreference,
    NotificationPreferenceResponse, NotificationPreferencesResponse,
    UpdateNotificationPreferenceRequest,
};
pub mod notification_event;
pub use notification_event::{ChannelEventCounts, NewNotificationEvent, NotificationEvent};
pub use oauth::{
    AuthorizeRequest, ConsentPageData, CreateAccessToken, CreateAuthorizationCode,
    CreateOAuthClient, CreateRefreshToken as CreateOAuthRefreshToken, CreateUserOAuthGrant,
    IntrospectionResponse, OAuthAccessToken, OAuthAuthorizationCode, OAuthClient,
    OAuthClientSummary, OAuthError, OAuthRefreshToken, OAuthScope, RegisterClientRequest,
    RegisterClientResponse, RevokeTokenRequest, ScopeDisplay, TokenRequest, TokenResponse,
    UpdateOAuthClient, UserGrantWithClient, UserGrantWithClientRow, UserOAuthGrant,
};
pub use oauth_token_event::{
    CreateOAuthTokenEvent, OAuthClientTokenUsage, OAuthTokenEvent, OAuthTokenEventTotals,
    OAuthTokenEventType, OAuthTokenKind,
};
pub use organization::{
    CreateOrganization, Organization, OrganizationStatus, OrganizationSummary, UpdateOrganization,
};
pub use organization_member::{
    CreateOrganizationMember, MembershipStatus, OrganizationMember, OrganizationMemberWithUser,
    UpdateOrganizationMember, UserOrganizationMembership,
};
pub use password_reset::{CreatePasswordResetToken, PasswordResetToken};
pub use person_month::{
    person_month_source, BuildingPersonMonthSummary, BulkPersonMonthEntry, BulkUpsertPersonMonths,
    CreatePersonMonth, MonthlyCount, PersonMonth, PersonMonthWithUnit, UpdatePersonMonth,
    YearlyPersonMonthSummary,
};
pub use platform_admin::{
    AdminOrganizationDetail, AdminOrganizationSummary, AnnouncementSeverity, CatalogPagination,
    CategoryWithCount, CreateFeatureCategoryRequest, CreateFeatureFlagOverrideRequest,
    CreateFeatureFlagRequest, CreateHelpArticleRequest, CreateMaintenanceRequest,
    CreateSystemAnnouncementRequest, FeatureAccessState, FeatureCatalogItem, FeatureCatalogQuery,
    FeatureCatalogResponse, FeatureCategory, FeatureCategorySummary, FeatureDescriptor,
    FeatureDescriptorDisplay, FeatureFlag, FeatureFlagOverride, FeatureFlagScope, FeatureState,
    FeatureUserTypeAccess, HelpArticle, HelpArticleRevision, MetricAlert, MetricThreshold,
    MetricType, OnboardingStep, OnboardingTour, OrganizationDetailMetrics, OrganizationMetrics,
    PlatformMetric, ReactivateOrganizationRequest, ScheduledMaintenance,
    SetFeatureUserTypeAccessRequest, SetUserFeaturePreferenceRequest, StepPlacement,
    SupportAccessLog, SupportAccessRequest, SupportAccessStatus, SuspendOrganizationRequest,
    SystemAnnouncement, SystemAnnouncementAcknowledgment, UpdateFeatureCategoryRequest,
    UpsertFeatureDescriptorRequest, UserFeaturePreference, UserOnboardingProgress,
};
pub use portal_password_reset::{CreatePortalPasswordResetToken, PortalPasswordResetToken};
pub use refresh_token::{CreateRefreshToken, LoginAttempt, RateLimitStatus, RefreshToken};
pub use role::{permissions, system_roles, CreateRole, PermissionDefinition, Role, UpdateRole};
pub use signature_request::{
    CancelSignatureRequestRequest, CancelSignatureRequestResponse, CreateSignatureRequest,
    CreateSignatureRequestResponse, CreateSigner, ListSignatureRequestsResponse,
    SendReminderRequest, SendReminderResponse, SignatureRequest, SignatureRequestResponse,
    SignatureRequestStatus, SignatureRequestWithDocument, SignatureWebhookEvent, Signer,
    SignerCounts, SignerStatus, WebhookResponse,
};
pub use two_factor_auth::{
    CreateTwoFactorAuth, TwoFactorAuth, TwoFactorStatus, UpdateTwoFactorStatus,
};
pub use unit::{
    occupancy_status, unit_status, unit_type, AssignUnitOwner, CreateUnit, Unit, UnitOwner,
    UnitOwnerInfo, UnitSummary, UnitWithOwners, UpdateUnit,
};
pub use unit_resident::{
    resident_type, CreateUnitResident, EndResidency, UnitResident, UnitResidentSummary,
    UnitResidentWithUser, UpdateUnitResident,
};
pub use user::{
    CreateUser, EmailVerificationToken, Locale, NeighborRow, NeighborView, PrincipalKind,
    PrivacySettings, ProfileVisibility, UpdatePrivacySettings, UpdateUser, User, UserStatus,
};

pub use membership::{GrantMembership, UserInvite, UserMembership, UserMergeCollision};
pub use vote::{
    audit_action, question_type, quorum_type, vote_status, CancelVote, CastVote, CreateVote,
    CreateVoteAuditLog, CreateVoteComment, CreateVoteQuestion, EligibleUnit, HideVoteComment,
    OptionResult, ParticipationDetail, PublishVote, QuestionOption, QuestionResult, UpdateVote,
    UpdateVoteQuestion, Vote, VoteAuditLog, VoteComment, VoteCommentWithUser, VoteEligibility,
    VoteListQuery, VoteQuestion, VoteReceipt, VoteReportData, VoteResponse, VoteResults,
    VoteSummary, VoteWithDetails,
};

// Epic 15: Property Listings & Multi-Portal Sync
// Epic 105: Portal Syndication
pub use listing::{
    currency, listing_status, portal as listing_portal, property_type, syndication_job_type,
    syndication_status, transaction_type, webhook_event_type, CreateListing, CreateListingFromUnit,
    CreateListingPhoto, CreatePortalWebhookEvent, CreateSyndication, Listing, ListingListQuery,
    ListingPhoto, ListingStatistics, ListingSummary, ListingSyndication, ListingWithDetails,
    OrganizationSyndicationStats, PortalInquiryWebhook, PortalStats, PortalSyndicationStatus,
    PortalViewWebhook, PortalWebhookEvent, PropertyTypeCount, PublishListingResponse,
    ReorderPhotos, SyndicationDashboardQuery, SyndicationDashboardResponse,
    SyndicationHealthStatus, SyndicationJobPayload, SyndicationResult, SyndicationStatusDashboard,
    UpdateListing, UpdateListingStatus,
};

// Epic 16: Portal Search & Discovery
pub use portal::{
    alert_frequency, AddFavorite, CreatePortalUser, CreateSavedSearch, Favorite,
    FavoriteWithListing, FavoriteWithListingRow, FavoritesResponse, MatchedListing, PortalSession,
    PortalUser, PublicListingDetail, PublicListingQuery, PublicListingSearchResponse,
    PublicListingSummary, SavedSearch, SavedSearchesResponse, SearchAlert, SearchCriteria,
    SearchSuggestions, UpdatePortalUser, UpdateSavedSearch,
};

// Epic 17: Agency & Realtor Management
pub use agency::{
    agency_status, import_source, import_status, inquiry_assignment, listing_visibility,
    member_role, AcceptInvitation, Agency, AgencyBranding, AgencyInvitation, AgencyListing,
    AgencyListingSummary, AgencyListingsResponse, AgencyMember, AgencyMemberWithUser,
    AgencyMemberWithUserRow, AgencyMembersResponse, AgencyProfile, AgencySummary, CreateAgency,
    CreateAgencyListing, CreateImportJob, FieldMapping, ImportConfig, ImportError, ImportPreview,
    ImportResult, InviteMember, ListingCollaborator, ListingEditHistory, ListingImportJob,
    UpdateAgency, UpdateListingVisibility, UpdateMemberRole,
};

// Epic 18: Short-Term Rental Integration
pub use rental::{
    authority_code, block_reason, booking_status as rental_booking_status, guest_status,
    rental_platform, report_status, report_type, BookingListQuery, BookingSummary,
    BookingWithGuests, BookingsResponse, CalendarBlock, CalendarEvent, CheckInReminder,
    ConnectionStatus, CreateBooking, CreateCalendarBlock, CreateGuest, CreateGuestIdDocument,
    CreateICalFeed, CreatePlatformConnection, GenerateReport, GuestSummary, ICalFeed,
    ICalFeedSummary, NationalityStats, OAuthCallback, PlatformConnectionDetail,
    PlatformConnectionSummary, PlatformSyncStatus, RegisterGuest, RentalBooking, RentalGuest,
    RentalGuestIdDocument, RentalGuestReport, RentalPlatformConnection, RentalStatistics,
    ReportPreview, ReportSummary, SubmitReport, UnitAvailability, UpdateBooking,
    UpdateBookingStatus, UpdateGuest, UpdateICalFeed, UpdatePlatformConnection,
};

// Epic 19: Lease Management & Tenant Screening
pub mod lease;

pub use lease::{
    application_status, lease_status, screening_status, screening_type, termination_reason,
    ApplicationListQuery, ApplicationSummary, CreateAmendment, CreateApplication, CreateLease,
    CreateLeaseTemplate, CreateReminder, ExpirationOverview, InitiateScreening, Lease,
    LeaseAmendment, LeaseListQuery, LeasePayment, LeaseReminder, LeaseStatistics, LeaseSummary,
    LeaseTemplate, LeaseWithDetails, PaymentSummary, RecordPayment as RecordLeasePayment,
    RenewLease, ReviewApplication, ScreeningConsent, ScreeningSummary, SubmitApplication,
    TenantApplication, TenantScreening, TerminateLease, UpdateApplication, UpdateLease,
    UpdateLeaseTemplate, UpdateScreeningResult,
};

// Epic 20: Maintenance Scheduling & Work Orders
pub mod work_order;

pub use work_order::{
    schedule_execution_status, schedule_frequency, update_type, work_order_priority,
    work_order_source, work_order_status, work_order_type, AddWorkOrderUpdate,
    CreateMaintenanceSchedule, CreateWorkOrder, MaintenanceCostSummary, MaintenanceSchedule,
    ScheduleExecution, ScheduleQuery, ServiceHistoryEntry, UpcomingSchedule,
    UpdateMaintenanceSchedule, UpdateWorkOrder, WorkOrder, WorkOrderQuery, WorkOrderStatistics,
    WorkOrderUpdate, WorkOrderWithDetails,
};

// Epic 21: Supplier & Vendor Management
// Epic 78: Vendor Operations Portal
pub mod vendor;

pub use vendor::{
    contract_status, contract_type, invoice_status, service_type, vendor_status, AcceptJobRequest,
    AccessCodeResponse, ContractQuery, CreateVendor, CreateVendorContact, CreateVendorContract,
    CreateVendorInvoice, CreateVendorRating, DeclineJobRequest, ExpiringContract,
    GenerateAccessCode, InvoiceQuery, InvoiceSummary, MaterialItem, PropertyAccessInfo,
    ProposeAlternativeTime, ServiceCount, SubmitWorkCompletion, UpdateVendor, UpdateVendorContract,
    UpdateVendorInvoice, Vendor, VendorContact, VendorContract, VendorDashboardStats,
    VendorEarningsSummary, VendorFeedback, VendorInvoice, VendorInvoiceWithTracking, VendorJob,
    VendorJobQuery, VendorJobSummary, VendorProfile, VendorQuery, VendorRating, VendorStatistics,
    VendorWithDetails, WorkCompletion,
};

// Epic 22: Insurance Management
pub mod insurance;

pub use insurance::{
    claim_status, policy_status, policy_type, premium_frequency, reminder_type, AddClaimDocument,
    AddPolicyDocument, ClaimStatusSummary, CreateInsuranceClaim, CreateInsurancePolicy,
    CreateRenewalReminder, ExpiringPolicy, InsuranceClaim, InsuranceClaimDocument,
    InsuranceClaimHistory, InsuranceClaimQuery, InsuranceClaimWithPolicy, InsurancePolicy,
    InsurancePolicyDocument, InsurancePolicyQuery, InsuranceRenewalReminder, InsuranceStatistics,
    PolicyTypeSummary, UpdateInsuranceClaim, UpdateInsurancePolicy, UpdateRenewalReminder,
};

// Epic 23: Emergency Management
pub mod emergency;

pub use emergency::{
    acknowledgment_status, contact_type, drill_status, drill_type, incident_status, incident_type,
    protocol_type, severity, AcknowledgeBroadcast, AddIncidentAttachment, BroadcastDeliveryStats,
    CompleteDrill, CreateEmergencyBroadcast, CreateEmergencyContact, CreateEmergencyDrill,
    CreateEmergencyIncident, CreateEmergencyProtocol, CreateIncidentUpdate, EmergencyBroadcast,
    EmergencyBroadcastAcknowledgment, EmergencyBroadcastQuery, EmergencyContact,
    EmergencyContactQuery, EmergencyDrill, EmergencyDrillQuery, EmergencyIncident,
    EmergencyIncidentAttachment, EmergencyIncidentQuery, EmergencyIncidentUpdate,
    EmergencyProtocol, EmergencyProtocolQuery, EmergencyStatistics, IncidentSeveritySummary,
    IncidentTypeSummary, UpdateEmergencyContact, UpdateEmergencyDrill, UpdateEmergencyIncident,
    UpdateEmergencyProtocol,
};

// Epic 24: Budget & Financial Planning
pub mod budget;

pub use budget::{
    budget_status, capital_plan_status, forecast_type, funding_source, priority,
    variance_alert_type, AcknowledgeVarianceAlert, Budget, BudgetActual, BudgetCategory,
    BudgetDashboard, BudgetItem, BudgetQuery, BudgetSummary, BudgetVarianceAlert, CapitalPlan,
    CapitalPlanQuery, CategoryVariance, CreateBudget, CreateBudgetCategory, CreateBudgetItem,
    CreateCapitalPlan, CreateFinancialForecast, FinancialForecast, ForecastQuery,
    RecordBudgetActual, UpdateBudget, UpdateBudgetCategory, UpdateBudgetItem, UpdateCapitalPlan,
    UpdateFinancialForecast, YearlyCapitalSummary,
};

// Epic 25: Legal Document & Compliance
pub mod legal;

pub use legal::{
    compliance_category, compliance_frequency, compliance_status, delivery_method, delivery_status,
    document_type, notice_priority, notice_type, recipient_type, AcknowledgeNotice, ApplyTemplate,
    ComplianceAuditTrail, ComplianceCategoryCount, ComplianceQuery, ComplianceRequirement,
    ComplianceRequirementWithDetails, ComplianceStatistics, ComplianceTemplate,
    ComplianceVerification, CreateAuditTrailEntry, CreateComplianceRequirement,
    CreateComplianceTemplate, CreateComplianceVerification, CreateLegalDocument,
    CreateLegalDocumentVersion, CreateLegalNotice, LegalDocument, LegalDocumentQuery,
    LegalDocumentSummary, LegalDocumentVersion, LegalNotice, LegalNoticeQuery,
    LegalNoticeRecipient, NoticeAcknowledgmentStats, NoticeRecipientInput, NoticeStatistics,
    NoticeTypeCount, NoticeWithRecipients, UpcomingVerification, UpdateComplianceRequirement,
    UpdateComplianceTemplate, UpdateLegalDocument, UpdateLegalNotice,
};

// Epic 26: Platform Subscription & Billing
pub mod subscription;

pub use subscription::{
    billing_cycle, coupon_duration, discount_type, line_item_type, metric_type,
    payment_method_type, subscription_invoice_status, subscription_status,
    CancelSubscriptionRequest, ChangePlanRequest, CouponRedemption, CreateOrganizationSubscription,
    CreateSubscriptionCoupon, CreateSubscriptionEvent, CreateSubscriptionPaymentMethod,
    CreateSubscriptionPlan, CreateUsageRecord, InvoiceLineItem, InvoiceQueryParams,
    InvoiceWithDetails, OrganizationSubscription, PlanSubscriptionCount, RedeemCouponRequest,
    SubscriptionCoupon, SubscriptionEvent, SubscriptionInvoice, SubscriptionPaymentMethod,
    SubscriptionPlan, SubscriptionStatistics, SubscriptionWithPlan, UpdateOrganizationSubscription,
    UpdateSubscriptionCoupon, UpdateSubscriptionPlan, UsageRecord, UsageSummary,
};

// Epic 30: Government Portal Integration
pub mod government_portal;

pub use government_portal::{
    AddSubmissionAttachment, CreatePortalConnection, CreateRegulatorySubmission,
    CreateSubmissionAudit, CreateSubmissionSchedule, GovernmentPortalConnection,
    GovernmentPortalStats, GovernmentPortalType, RegulatoryReportTemplate, RegulatorySubmission,
    RegulatorySubmissionAttachment, RegulatorySubmissionAudit, RegulatorySubmissionSchedule,
    SubmissionQuery, SubmissionStatus, SubmissionSummary, TemplateSummaryGov, UpcomingDueDate,
    UpdatePortalConnection, UpdateRegulatorySubmission, UpdateSubmissionSchedule, ValidationError,
    ValidationResult, ValidationWarning,
};

// Epics 31-34: Reality Portal Professional
pub mod reality_portal;

pub use reality_portal::{
    AgencyMemberWithUser as RealityAgencyMemberWithUser, AgencySummary as RealityAgencySummary,
    AssignRealtorListing, CreateAgencyInvitation, CreateCrmConnection, CreateFeedSubscription,
    CreateImportJob as CreatePortalImportJob, CreateListingInquiry, CreatePortalSavedSearch,
    CreateRealityAgency, CreateRealtorProfile, CrmConnection, FavoriteAlert, FeedSubscription,
    ImportJobProgress, InquiryMessage, InquiryWithListing, ListingAnalytics,
    ListingAnalyticsSummary, ListingInquiry, ListingPriceHistory, PortalFavorite,
    PortalFavoriteWithListing, PortalImportJob, PortalImportJobWithStats, PortalSavedSearch,
    PriceChangeAlert, PublicRealtorProfile, RealityAgency, RealityAgencyInvitation,
    RealityAgencyMember, RealityFeedSubscription, RealtorListing, RealtorProfile, SavedSearchAlert,
    ScheduleViewing, SearchAlertQueueEntry, SendInquiryMessage, UndeliveredSearchAlert,
    UpdateAgencyBranding, UpdateCrmConnection, UpdateFeedSubscription,
    UpdateImportJob as UpdatePortalImportJob, UpdatePortalFavorite, UpdatePortalSavedSearch,
    UpdateRealityAgency, UpdateRealtorProfile, UpdateViewing, ViewingSchedule,
};

// Epic 37: Community & Social Features
pub mod community;

pub use community::{
    CommunityComment, CommunityEvent, CommunityEventRsvp, CommunityEventWithRsvp, CommunityGroup,
    CommunityGroupMember, CommunityGroupWithMembership, CommunityPost, CommunityPostWithAuthor,
    CreateCommunityComment, CreateCommunityEvent, CreateCommunityGroup, CreateCommunityPost,
    CreateMarketplaceInquiry, CreateMarketplaceItem, EventRsvpRequest, JoinGroupRequest,
    MarketplaceInquiry, MarketplaceItem, MarketplaceItemWithSeller, PollOption,
    UpdateCommunityEvent, UpdateCommunityGroup, UpdateCommunityPost, UpdateMarketplaceItem,
};

// Epic 38: Workflow Automation
pub mod automation;

// Epic 54: Forms Management
pub mod form;

// Epic 58: Package & Visitor Management
pub mod package_visitor;

// Epic 61: External Integrations Suite
pub mod integration;

// Epic 64: Advanced AI & LLM Capabilities
pub mod llm_document;

pub use integration::{
    accounting_system, calendar_provider, calendar_sync_status,
    delivery_status as webhook_delivery_status, esignature_provider, esignature_status,
    export_status, meeting_status, video_provider, webhook_event, webhook_status, AccountingExport,
    AccountingExportSettings, CalendarConnection, CalendarEvent as IntegrationCalendarEvent,
    CalendarSyncResult, CreateAccountingExport, CreateCalendarConnection,
    CreateCalendarEvent as CreateIntegrationCalendarEvent, CreateESignatureRecipient,
    CreateESignatureWorkflow, CreateVideoConferenceConnection, CreateVideoMeeting,
    CreateWebhookSubscription, ESignatureEvent, ESignatureRecipient, ESignatureWorkflow,
    ESignatureWorkflowWithRecipients, IntegrationStatistics, MeetingParticipant, MeetingSettings,
    PohodaExportData, PohodaInvoice, PohodaInvoiceItem, PohodaPayment, SyncCalendarRequest,
    TestWebhookRequest, TestWebhookResponse, UpdateAccountingExportSettings,
    UpdateCalendarConnection, UpdateVideoMeeting, UpdateWebhookSubscription,
    VideoConferenceConnection, VideoMeeting, WebhookDeliveryLog, WebhookDeliveryQuery,
    WebhookRetryPolicy, WebhookStatistics, WebhookSubscription,
};

pub use automation::{
    AutomationAction, AutomationLogSummary, AutomationRuleWithStats, CallWebhookConfig,
    ConditionTriggerConfig, CreateAutomationRule, CreateRuleFromTemplate, EventTriggerConfig,
    GenerateReportConfig, ScheduleTriggerConfig, SendEmailConfig, SendNotificationConfig,
    UpdateAutomationRule, WorkflowAutomationLog, WorkflowAutomationRule,
    WorkflowAutomationTemplate,
};

// Epic 54: Forms Management
pub use form::{
    field_type, form_status, submission_status, target_type as form_target_type,
    ConditionalDisplay, CreateForm, CreateFormField, CreateFormResponse,
    ExportFormat as FormExportFormat, ExportSubmissionsRequest, FieldOption, Form, FormAttachment,
    FormDownload, FormField, FormListQuery, FormListResponse, FormStatistics, FormSubmission,
    FormSubmissionParams, FormSubmissionSummary, FormSubmissionWithDetails, FormSummary,
    FormWithDetails, ReviewSubmission, SignatureData, SubmissionListQuery, SubmissionListResponse,
    SubmitForm, SubmitFormResponse, UpdateForm, UpdateFormField, ValidationRules,
};

// Epic 55: Advanced Reporting & Analytics
pub mod reports;

pub use reports::{
    CategoryTrend, ConsumptionAnomaly, ConsumptionReportData, ConsumptionSummary, DateRange,
    FaultTrends, MonthlyAverage, MonthlyConsumption, MonthlyPersonCount, OccupancyReportData,
    OccupancySummary, OccupancyTrends, ReportMonthlyCount, UnitConsumption, UnitOccupancy,
    UtilityTypeConsumption, VoteParticipationDetail, VotingParticipationSummary, YearComparison,
};

// Epic 81: Report Schedule Management & Execution History
pub mod report_schedule;

pub use report_schedule::{
    report_execution_status, report_schedule_status, ExecutionDownloadUrl, ExecutionHistoryQuery,
    ExecutionHistoryResponse, ReportExecution, ReportSchedule,
};

// Epic 58: Package & Visitor Management
pub use package_visitor::{
    package_carrier, package_status, visitor_purpose, visitor_status, AccessCodeVerification,
    BuildingPackageSettings, BuildingVisitorSettings, CheckInVisitor, CheckOutVisitor,
    CreatePackage, CreateVisitor, Package, PackageQuery, PackageStatistics, PackageSummary,
    PackageWithDetails, PickupPackage, ReceivePackage, UpdateBuildingPackageSettings,
    UpdateBuildingVisitorSettings, UpdatePackage, UpdateVisitor, VerifyAccessCode, Visitor,
    VisitorQuery, VisitorStatistics, VisitorSummary, VisitorWithDetails,
};

// Epic 59: News & Media Management
pub mod news_article;

// Epic 65: Energy & Sustainability Tracking
pub mod energy;

pub use news_article::{
    article_status, reaction_type, ArchiveArticle, ArticleComment, ArticleListQuery, ArticleMedia,
    ArticleReaction, ArticleStatistics, ArticleSummary, ArticleView, ArticleWithDetails,
    ArticleWithDetailsRow, CommentWithAuthor as ArticleCommentWithAuthor,
    CommentWithAuthorRow as ArticleCommentWithAuthorRow, CreateArticle, CreateArticleComment,
    CreateArticleMedia, ModerateComment, NewsArticle, PinArticle, PublishArticle, ReactionCounts,
    ToggleReaction, UpdateArticle, UpdateArticleComment,
};

// Epic 65: Energy & Sustainability Tracking
pub use energy::{
    BenchmarkAlert, BenchmarkAlertSeverity, BenchmarkAlertsQuery, BenchmarkDashboard,
    BenchmarkMetricType, BenchmarkQuery, BuildingBenchmark, CalculateBenchmark, CarbonDashboard,
    CarbonEmission, CarbonExportRequest, CarbonTarget, CreateBenchmarkAlert, CreateCarbonEmission,
    CreateCarbonTarget, CreateEnergyPerformanceCertificate, CreateSustainabilityScore,
    EmissionSourceType, EnergyPerformanceCertificate, EnergyRating, EpcSummary, HeatingType,
    InsulationRating, ListBenchmarkAlertsResponse, ListBenchmarksResponse, ListEmissionsResponse,
    ListEpcsResponse, MonthlyEmission, RatingCount, SourceEmission, SustainabilityFilter,
    SustainabilityScore, UpdateEnergyPerformanceCertificate, UpdateSustainabilityScore,
};

// Epic 64: Advanced AI & LLM Capabilities
// Epic 93: Voice Assistant OAuth Completion
pub use llm_document::{
    enhancement_status, enhancement_type, generation_status, llm_provider, supported_language,
    voice_intent, voice_platform, AiEscalationConfig, AiUsageQuery, AiUsageStatistics,
    AlexaApplication, AlexaCard, AlexaIntent, AlexaOutputSpeech, AlexaRequestBody,
    AlexaResponseBody, AlexaSession, AlexaSkillRequest, AlexaSkillResponse, AlexaUser,
    BatchEnhancePhotosRequest, BatchEnhancePhotosResponse, CreatePromptTemplate, DescriptionStyle,
    DocumentEmbedding, EnhancePhotoRequest, EnhancedChatRequest, EnhancedChatResponse,
    EnhancementOptions, GenerateLeaseRequest, GenerateListingDescriptionRequest,
    GeneratedListingDescription, GeneratedListingDescriptionResponse, GoogleActionsRequest,
    GoogleActionsResponse, GoogleCard, GoogleContent, GoogleHandler, GoogleImage, GoogleIntent,
    GooglePrompt, GoogleScene, GoogleSceneResponse, GoogleSession, GoogleSessionResponse,
    GoogleSimpleResponse, GoogleUser, LeaseClause, LeaseGenerationInput, LeaseGenerationResult,
    LinkVoiceDeviceRequest, LinkVoiceDeviceResponse, ListingLocation, LlmGenerationRequest,
    LlmPromptTemplate, ParsedVoiceCommand, PhotoEnhancement, PhotoEnhancementResponse,
    ProviderStats, RagContextSource, RequestTypeStats, UpdateEscalationConfig,
    UpdatePromptTemplate, VoiceActionResult, VoiceAssistantDevice, VoiceCard, VoiceCommandHistory,
    VoiceCommandRequest, VoiceCommandResponse, VoiceOAuthExchangeRequest,
    VoiceOAuthExchangeResponse, VoiceTokenRefreshRequest, VoiceTokenRefreshResult,
    WebhookVerificationResult,
};

// UC-12: Utility Outages
pub mod outage;

// Epic 132: Dynamic Rent Pricing & Market Analytics
pub mod market_pricing;

// Epic 72: Regional Legal Compliance (SK/CZ)
pub mod regional_compliance;

pub use regional_compliance::{
    slovak_accounts, ConfigureCzechSvj, ConfigureSlovakAccounting, ConfigureSlovakGdpr,
    ConfigureSlovakVoting, ConsentCategoryStatus, CzechDecisionType, CzechOrgType, CzechSvjConfig,
    CzechSvjUsneseni, CzechVoteValidation, DpoContact, ExportSlovakAccounting, GdprConsentCategory,
    GdprConsentStatus, Jurisdiction, PohodaExport, PohodaHeader, PohodaInvoiceExport,
    PohodaInvoiceItemExport, PohodaPaymentExport, ProcessingPurpose, QuestionMinutes,
    RecordGdprConsent, RegionalComplianceStatus, SetJurisdiction, SlovakAccountingConfig,
    SlovakAccountingExport, SlovakAccountingFormat, SlovakDecisionType, SlovakGdprConfig,
    SlovakGdprConsent, SlovakVoteMinutes, SlovakVoteValidation, SlovakVotingConfig,
    ValidateCzechVote, ValidateSlovakVote, VoteParticipantMinutes,
};

// Epic 76: Move-in/Move-out Workflow
pub mod move_workflow;

pub use move_workflow::{
    assignee_type, deduction_status, item_condition, key_type, task_status, workflow_status,
    workflow_type, CompleteInspection, CompleteTimelineTask, CreateDepositDeduction,
    CreateInspection, CreateInspectionItem, CreateInspectionPhoto, CreateInspectionTemplate,
    CreateKeyHandoff, CreateMoveWorkflow, CreateTimelineFromTemplate, CreateTimelineTask,
    DepositDeduction, DepositSummary, DisputeDeduction, Inspection, InspectionItem,
    InspectionItemWithPhotos, InspectionPhoto, InspectionQuery, InspectionSummary,
    InspectionTemplate, InspectionTemplateSummary, InspectionWithDetails, ItemTemplate, KeyHandoff,
    MoveTimelineTask, MoveWorkflow, MoveWorkflowStatistics, MoveWorkflowSummary,
    MoveWorkflowWithDetails, ResolveDeduction, RoomTemplate, TimelineOverview,
    TimelineTaskTemplate, UpdateInspectionItem, UpdateInspectionTemplate, UpdateMoveWorkflow,
    UpdateTimelineTask, WorkflowQuery as MoveWorkflowQuery,
};

// Epic 57: Building Registry (pet/vehicle/parking)
pub mod registry;

pub use registry::{
    pet_size, pet_type, registry_status, vehicle_type, BuildingRegistryRules, CreateParkingSpot,
    CreatePetRegistration, CreateVehicleRegistration, ParkingSpot, ParkingSpotQuery,
    ParkingSpotWithDetails, PetRegistration, PetRegistrationQuery, PetRegistrationSummary,
    PetRegistrationWithDetails, RegistryStatistics, ReviewRegistration, UpdateParkingSpot,
    UpdatePetRegistration, UpdateRegistryRules, UpdateVehicleRegistration, VehicleRegistration,
    VehicleRegistrationQuery, VehicleRegistrationSummary, VehicleRegistrationWithDetails,
};

// Epic 66: Platform Migration & Data Import
pub mod migration;

pub use migration::{
    ApproveImportRequest, ApproveImportResponse, ColumnMappingStatus,
    CreateImportJob as CreateMigrationImportJob, CreateImportTemplate, CreateMigrationExport,
    DuplicateRecord, ExportCategoriesResponse, ExportCategoryInfo, ExportColumnDefinition,
    ExportDataCategory, ExportFileEntry, ExportPrivacyOptions, ExportSchemaMetadata, FieldDataType,
    FieldDifference, FieldValidation, ImportCategoriesResponse, ImportCategoryInfo, ImportDataType,
    ImportFieldMapping, ImportJob, ImportJobFilter, ImportJobHistory, ImportJobStatus,
    ImportJobStatusResponse, ImportOptions, ImportPreviewResult, ImportRowError, ImportTemplate,
    ImportTemplateListResponse, ImportTemplateSummary, MigrationExport, MigrationExportResponse,
    MigrationExportStatus, MigrationExportStatusResponse, MigrationPagination, RecordTypeCounts,
    TemplateFormat, UpdateImportTemplate, ValidationIssue, ValidationSeverity,
};

// Epic 67: Advanced Compliance (AML/DSA)
pub mod compliance;

pub use compliance::{
    AddComplianceNote, AmlAssessmentStatus, AmlRiskAssessment, AmlRiskLevel, ComplianceNote,
    ContentOwnerInfo, ContentTypeCount, CountryRisk, CountryRiskRating, CreateAmlRiskAssessment,
    CreateDsaTransparencyReport, CreateEddDocument, CreateEnhancedDueDiligence,
    CreateModerationCase, DecideAppeal, DocumentVerificationStatus, DsaReportStatus,
    DsaReportSummary, DsaTransparencyReport, DsaTransparencyReportResponse, EddDocument, EddStatus,
    EnhancedDueDiligence, FileAppeal, ModeratedContentType, ModerationActionTemplate,
    ModerationActionType, ModerationCase, ModerationCaseResponse, ModerationQueueQuery,
    ModerationQueueStats, ModerationStatus, PriorityCount as AmlPriorityCount, ReportSource,
    RiskFactor, SuspiciousActivity, TakeModerationAction, ViolationType, ViolationTypeCount,
    AML_THRESHOLD_EUR_CENTS,
};

// Epic 68: Service Provider Marketplace
pub mod marketplace;

pub use marketplace::{
    badge_type, pricing_type, profile_status, quote_status, review_status, rfq_status,
    service_category, verification_status, verification_type,
    CategoryCount as MarketplaceCategoryCount, CreateProviderQuote, CreateProviderReview,
    CreateProviderVerification, CreateRequestForQuote, CreateServiceProviderProfile,
    ExpiringVerification, ManagerMarketplaceDashboard, MarketplaceSearchQuery,
    MarketplaceStatistics, ModerateReviewRequest, PendingAction, ProviderBadge, ProviderDashboard,
    ProviderDetailView, ProviderProfileComplete, ProviderQuote, ProviderReview,
    ProviderReviewResponse, ProviderReviewWithResponse, ProviderSearchResult, ProviderVerification,
    QuoteComparisonView, QuoteWithProvider, RatingBreakdown, RatingDistribution, RequestForQuote,
    ReviewQuery, ReviewStatistics, ReviewVerificationRequest, RfqInvitation, RfqQuery, RfqSummary,
    ServiceProviderProfile, UpdateProviderQuote, UpdateProviderReview, UpdateRequestForQuote,
    UpdateServiceProviderProfile, VerificationQuery, VerificationQueueItem,
};

// Epic 71: Cross-Cutting Infrastructure
pub mod infrastructure;

pub use infrastructure::{
    job_type, queue, AcknowledgeAlert, AlertSeverity, AlertStatus, BackgroundJob,
    BackgroundJobExecution, BackgroundJobQuery, BackgroundJobQueueStats, BackgroundJobStatus,
    BackgroundJobTypeStats, CreateBackgroundJob, CreateFeatureFlag as CreateInfraFeatureFlag,
    CreateHealthAlertRule, CreateSpan, CreateTrace, DependencyHealth, EvaluateFeatureFlag,
    FeatureFlag as InfraFeatureFlag, FeatureFlagAuditAction, FeatureFlagAuditLog,
    FeatureFlagEvaluation, FeatureFlagOverride as InfraFeatureFlagOverride,
    FeatureFlagOverrideType, FeatureFlagValueType, HealthAlert, HealthAlertRule, HealthCheckConfig,
    HealthCheckResult, HealthCheckType, HealthStatus, InfrastructureDashboard,
    PaginatedResponse as InfrastructurePaginatedResponse, PrometheusMetric, ResolveAlert,
    RetryBackgroundJob, Span, SpanKind, SpanStatus, SystemHealth, SystemMetrics, Trace, TraceQuery,
    TraceWithSpans, UpdateFeatureFlag as UpdateInfraFeatureFlag, UpdateHealthAlertRule,
};

// Epic 73: Infrastructure & Operations
pub mod operations;

pub use operations::{
    Backup, BackupQuery, BackupStatus, BackupType, CloudServiceType, CostAlert, CostAlertQuery,
    CostAlertSeverity, CostBudget, CostDashboard, CostOptimizationRecommendation, CostQuery,
    CostTrendPoint, CreateBackup, CreateCostBudget, CreateDeployment, CreateMigration,
    DatabaseMigration, Deployment, DeploymentDashboard, DeploymentEnvironment,
    DeploymentHealthCheck, DeploymentQuery, DeploymentStatus, DisasterRecoveryDashboard,
    DisasterRecoveryDrill, InfrastructureCost, InitiateRecovery, ListBackupsResponse,
    ListBudgetsResponse, ListCostAlertsResponse, ListCostsResponse, ListDeploymentsResponse,
    ListMigrationsResponse, ListRecommendationsResponse, ListUtilizationResponse, MigrationLog,
    MigrationQuery, MigrationSafetyCheck, MigrationStatus, MigrationStrategy, RecordDrDrill,
    RecordInfrastructureCost, RecoveryOperation, RecoveryStatus, ResourceUtilization,
    SchemaVersion, ServiceCostSummary, SwitchTraffic, UpdateDeploymentStatus,
    UpdateMigrationProgress,
};

// Epic 74: Owner Investment Analytics
pub mod owner_analytics;

pub use owner_analytics::{
    AddComparableProperty, CalculateROIRequest, CashFlowBreakdown, CashFlowExpenses,
    CashFlowIncome, ComparableProperty as OwnerAnalyticsComparableProperty, ComparisonMetrics,
    CreateAutoApprovalRule, CreatePropertyValuation, ExpenseApprovalRequest,
    ExpenseApprovalResponse, ExpenseApprovalStatus, ExpenseAutoApprovalRule, ExpenseRequestsQuery,
    ListExpenseRequestsResponse, MonthlyCashFlow, OwnerPropertiesQuery, PortfolioComparisonRequest,
    PortfolioProperty, PortfolioSummary, PropertyComparison, PropertyROI, PropertyValuation,
    PropertyValuationWithComparables, PropertyValueHistory, ROIDashboard, ReviewExpenseRequest,
    SubmitExpenseForApproval, UpdateAutoApprovalRule, ValuationMethod, ValueHistoryQuery,
    ValueTrendAnalysis,
};

// Epic 77: Dispute Resolution
pub mod disputes;

// Epic 108: Feature Packages & Bundles
pub mod feature_package;

// Epic 109: User Type Feature Experience
pub mod feature_analytics;

pub use disputes::{
    action_status, activity_type, dispute_category, dispute_priority, dispute_state_machine,
    dispute_status, escalation_severity, party_role, resolution_status, session_status,
    session_type, ActionItem, AddEvidence, CategoryCount as DisputeCategoryCount,
    CompleteActionItem, CreateActionItem, CreateEscalation, Dispute, DisputeActivity,
    DisputeEvidence, DisputeFunnelKpis, DisputeKpis, DisputeParty, DisputePartyWithUser,
    DisputeQuery, DisputeResolution, DisputeStatistics, DisputeSummary, DisputeTtrKpis,
    DisputeWithDetails, Escalation, FileDispute, MediationCase, MediationSession,
    MediationSessionWithAttendance, PartyActionsDashboard, PartySubmission,
    PriorityCount as DisputePriorityCount, ProposeResolution, RecordSessionNotes, ResolutionTerm,
    ResolutionVote, ResolutionWithVotes, ResolveDispute, ResolveEscalation, ScheduleSession,
    SessionAttendance, StatusCount as DisputeStatusCount, SubmitResponse, UpdateDisputeStatus,
    UpdateMediationNotes, VoteOnResolution,
};

// Epic 108: Feature Packages & Bundles
pub use feature_package::{
    package_source, BatchAddFeatures, CreateFeaturePackage as CreateFeaturePackage108,
    CreateFeaturePackageItem, CreateOrganizationPackage, FeatureComparisonRow,
    FeaturePackage as FeaturePackage108, FeaturePackageItem as FeaturePackageItem108,
    FeaturePackageItemWithDetails, FeaturePackageQuery, FeaturePackageSummary,
    FeaturePackageWithFeatures as FeaturePackageWithFeatures108, OrganizationPackage,
    OrganizationPackageWithDetails, PackageComparison, PackageType, PublicPackage,
    UpdateFeaturePackage as UpdateFeaturePackage108, UpdateOrganizationPackage,
};

// Epic 109: User Type Feature Experience
pub use feature_analytics::{
    CreateFeaturePackage, FeatureAccessState as FeatureAccessState109,
    FeatureDescriptor as FeatureDescriptor109, FeatureDescriptorSummary, FeatureEventType,
    FeaturePackage, FeaturePackageFeature, FeaturePackageItem, FeaturePackageWithFeatures,
    FeatureStatsByUserType, FeatureStatsQuery, FeatureUsageEvent, FeatureUsageStats,
    LogFeatureEvent, OrganizationFeaturePackage, ResolvedFeature, ResolvedFeaturesQuery,
    SetFeaturePreference, SetUserTypeAccess, SubscribeToPackage, UpdateFeaturePackage,
    UpgradeOptionsResponse, UpsertFeatureDescriptor,
    UserFeaturePreference as UserFeaturePreference109, UserTypeFeatureAccess,
};

// UC-12: Utility Outages
pub use outage::{
    outage_commodity, outage_severity, outage_status, CancelOutage, CommodityCount, CreateOutage,
    CreateOutageNotification, MarkOutageRead, Outage, OutageDashboard, OutageListQuery,
    OutageNotification, OutageStatistics, OutageSummary, OutageWithDetails, ResolveOutage,
    StartOutage, UpdateOutage,
};

// Epic 132: Dynamic Rent Pricing & Market Analytics
pub use market_pricing::{
    pricing_recommendation_status, pricing_source, AcceptPricingRecommendation, AddCmaProperty,
    CmaAnalysisSummary, CmaPropertyComparison, CmaSummary, CmaWithProperties,
    ComparativeMarketAnalysis, CreateComparativeMarketAnalysis, CreateMarketDataPoint,
    CreateMarketRegion, ExportPricingDataRequest, GenerateStatisticsRequest, MarketComparable,
    MarketDataPoint, MarketDataQuery, MarketRegion, MarketRegionSummary, MarketStatistics,
    MarketStatisticsSummary, MarketTrendPoint, PortfolioPricingSummary,
    PriceRange as MarketPriceRange, PricingDashboard, PricingDashboardQuery,
    PricingFactor as MarketPricingFactor, PricingHistoryEntry, PricingRecommendation,
    PricingRecommendationSummary, PricingRecommendationWithDetails, RecordPriceChange,
    RejectPricingRecommendation, RequestPricingRecommendation, UnitPricingHistory,
    UnitRecommendationSummary, UpdateComparativeMarketAnalysis, UpdateMarketRegion,
    VacancyTrendPoint, YieldRange,
};

// Epic 137: Smart Building Certification
pub mod building_certification;

pub use building_certification::{
    BuildingCertification, CertificationAuditLog, CertificationBenchmark, CertificationCost,
    CertificationCredit, CertificationDashboard, CertificationDocument, CertificationFilters,
    CertificationLevel, CertificationLevelCount, CertificationMilestone, CertificationProgram,
    CertificationProgramCount, CertificationReminder, CertificationStatus,
    CertificationWithCredits, CreateBuildingCertification, CreateCertificationBenchmark,
    CreateCertificationCost, CreateCertificationCredit, CreateCertificationDocument,
    CreateCertificationMilestone, CreateCertificationReminder, CreditCategoryType,
    UpdateBuildingCertification, UpdateCertificationCredit, UpdateCertificationMilestone,
};

// Epic 133: AI Lease Abstraction & Document Intelligence
pub mod lease_abstraction;
pub use lease_abstraction::{
    document_status as lease_document_status, import_status as lease_import_status,
    review_status as extraction_review_status, ApproveExtraction, CreateExtractionCorrection,
    CreateLeaseDocument, CreateLeaseExtraction, ExtractedField, ExtractionCorrection,
    ExtractionSummary, ExtractionWithFields, ImportExtractionRequest,
    ImportResult as LeaseImportResult, ImportValidationResult, LeaseDocument, LeaseDocumentQuery,
    LeaseDocumentSummary, LeaseExtraction, LeaseImport, ProcessDocumentRequest, ProcessingStatus,
    RejectExtraction, ValidationIssue as ExtractionValidationIssue,
};

// Epic 134: Predictive Maintenance & Equipment Intelligence
pub mod predictive_maintenance;

pub use predictive_maintenance::{
    AcknowledgeAlertRequest, AlertSeverity as PredictiveAlertSeverity,
    AlertStatus as PredictiveAlertStatus, AlertType as PredictiveAlertType, AlertWithEquipment,
    AlertsBySeverity, CreateEquipment as CreatePredictiveEquipment, CreateEquipmentDocument,
    CreateMaintenanceLog, Equipment as PredictiveEquipment, EquipmentByStatus, EquipmentDocument,
    EquipmentPrediction, EquipmentQuery as PredictiveEquipmentQuery,
    EquipmentStatus as PredictiveEquipmentStatus, EquipmentSummary as PredictiveEquipmentSummary,
    EquipmentType, HealthDistribution, HealthThreshold, MaintenanceAlert, MaintenanceDashboard,
    MaintenanceLog, MaintenanceLogPhoto, MaintenanceOutcome, MaintenanceTrend,
    MaintenanceType as PredictiveMaintenanceType, PredictionFactor, PredictionResult,
    RecommendedAction, ResolveAlertRequest, RunPredictionRequest, SetHealthThreshold,
    UpdateEquipment as UpdatePredictiveEquipment, UpdateMaintenanceLog,
};

// Epic 140: Multi-Property Portfolio Analytics
pub mod portfolio_analytics;

pub use portfolio_analytics::{
    AcknowledgeAlert as AcknowledgePortfolioAlert, AggregationPeriod, AlertStats,
    BenchmarkCategory, BenchmarkPerformance, ComparisonScope, CreateAlertRule,
    CreatePortfolioBenchmark, CreatePropertyComparison, CreatePropertyMetrics,
    PortfolioAggregatedMetrics, PortfolioAlert, PortfolioAlertRule, PortfolioAnalyticsDashboard,
    PortfolioAnalyticsQuery, PortfolioBenchmark, PortfolioPropertyComparison,
    PortfolioSummary as PortfolioAnalyticsSummary, PortfolioTrend, PropertyPerformanceMetrics,
    PropertyRanking, RecordTrend, ResolveAlert as ResolvePortfolioAlert, TrendAnalysis,
    TrendDataPoint, UpdateAlertRule, UpdatePortfolioBenchmark,
};

// Epic 135: Enhanced Tenant Screening with AI Risk Scoring
pub mod enhanced_tenant_screening;

pub use enhanced_tenant_screening::{
    AiResultWithFactors, AiRiskCategory, AiRiskScoringModel, CompleteScreeningData,
    CreateAiRiskScoringModel, CreateScreeningBackgroundResult, CreateScreeningCreditResult,
    CreateScreeningEvictionResult, CreateScreeningProviderConfig, CreateScreeningQueueItem,
    CreateScreeningReport, CreateScreeningRiskFactor, InitiateScreeningRequest,
    ProviderIntegrationStatus, RiskCategoryDistribution, RiskFactorCategory, RiskFactorImpact,
    RunAiScoringRequest, ScreeningAiResult, ScreeningBackgroundResult, ScreeningCreditResult,
    ScreeningEvictionResult, ScreeningProviderConfig, ScreeningProviderType, ScreeningReport,
    ScreeningRequestQueueItem, ScreeningRiskFactor, ScreeningStatistics,
    ScreeningSummary as EnhancedScreeningSummary, UpdateAiRiskScoringModel,
    UpdateScreeningProviderConfig,
};

// Epic 136: ESG Reporting Dashboard
pub mod esg_reporting;

pub use esg_reporting::{
    CalculateCarbonFootprintRequest, CarbonFootprint, CarbonFootprintQuery, CarbonFootprintSummary,
    CreateCarbonFootprint, CreateEsgBenchmark, CreateEsgConfiguration, CreateEsgImportJob,
    CreateEsgMetric, CreateEsgReport, CreateEsgTarget, CreateEuTaxonomyAssessment,
    EnergySourceType, EsgBenchmark, EsgBenchmarkCategory, EsgBenchmarkComparison,
    EsgComplianceFramework, EsgConfiguration, EsgDashboardMetrics, EsgDataEntryMethod,
    EsgEmissionScope, EsgImportJob, EsgMetric, EsgMetricCategory, EsgMetricsQuery, EsgReport,
    EsgReportStatus, EsgStatistics, EsgSummaryScores, EsgTarget, EuTaxonomyAssessment,
    GenerateEsgReportRequest, UpdateEsgConfiguration, UpdateEsgMetric, UpdateEsgReport,
    UpdateEsgTarget, UpdateEuTaxonomyAssessment,
};

// Epic 138: Automated Property Valuation Model
pub mod property_valuation;

pub use property_valuation::{
    AdjustmentType,
    ComparableAdjustment,
    CreateAdjustment,
    CreateComparable,
    CreateMarketData,
    CreatePropertyFeatures,
    // Aliased to avoid conflicts with owner_analytics
    CreatePropertyValuation as CreateAvmValuation,
    CreateValuationAuditLog,
    CreateValuationModel,
    CreateValuationReport,
    CreateValuationRequest,
    CreateValueHistory,
    MarketAnalysisSummary,
    MarketTrend as AvmMarketTrend,
    PropertyCondition,
    PropertyValuation as AvmValuation,
    PropertyValuationFeatures,
    PropertyValuationModel,
    PropertyValueHistory as AvmValueHistory,
    PropertyValueTrend as AvmValueTrend,
    UpdateComparable,
    UpdateMarketData,
    UpdatePropertyFeatures,
    UpdatePropertyValuation as UpdateAvmValuation,
    UpdateValuationModel,
    UpdateValuationReport,
    UpdateValuationRequest,
    ValuationAuditLog,
    ValuationComparable,
    ValuationConfidence,
    ValuationDashboard,
    ValuationMarketData,
    ValuationModelType,
    ValuationReport,
    ValuationRequest,
    ValuationStatus,
    ValuationWithDetails,
    ValueHistoryPoint,
};

// Epic 139: Investor Portal & ROI Reporting
pub mod investor_portal;

pub use investor_portal::{
    CapitalCall, CreateCapitalCall, CreateDashboardMetrics, CreateDistribution,
    CreateInvestmentPortfolio, CreateInvestorPortfolioProperty, CreateInvestorProfile,
    CreateInvestorReport, CreateRoiCalculation, DistributionType, InvestmentPortfolio,
    InvestmentStatus, InvestorDashboardMetrics, InvestorDistribution, InvestorPortalDashboard,
    InvestorPortfolioProperty, InvestorPortfolioSummary, InvestorProfile, InvestorReport,
    InvestorReportType, InvestorSummary, InvestorType, PortfolioWithDetails, RoiCalculation,
    RoiCalculationQuery, RoiPeriod, RoiSummary, UpdateCapitalCall, UpdateDistribution,
    UpdateInvestmentPortfolio, UpdateInvestorPortfolioProperty, UpdateInvestorProfile,
};

// Epic 141: Reserve Fund Management
pub mod reserve_funds;

pub use reserve_funds::{
    AcknowledgeFundAlert, ComponentReplacementSchedule, ContributionFrequency,
    CreateContributionSchedule, CreateFundComponent, CreateFundProjection, CreateInvestmentPolicy,
    CreateProjectionItem, CreateReserveFund as CreateReserveFund141, FundAlert, FundComponent,
    FundContributionSchedule, FundDashboard, FundHealthReport, FundInvestmentPolicy,
    FundProjection, FundProjectionItem, FundSummary, FundTransaction, FundTransactionType,
    FundTransferRequest, FundType, InvestmentRiskLevel, RecordFundTransaction,
    ReserveFund as ReserveFund141, ReserveStudySummary, ResolveFundAlert, TransactionQuery,
    UpdateContributionSchedule, UpdateFundComponent, UpdateReserveFund as UpdateReserveFund141,
};

// Epic 142: Violation Tracking & Enforcement
pub mod violations;
pub use violations::{
    AppealQuery, AppealStatus, CategoryCount as ViolationCategoryCount, CommunityRule,
    CreateCommunityRule, CreateEnforcementAction, CreateViolation, CreateViolationAppeal,
    CreateViolationComment, CreateViolationEvidence, EnforcementAction, EnforcementActionType,
    EnforcementQuery, EnforcementStatus, FinePayment, RecordFinePayment, RuleComplianceSummary,
    StatusCount as ViolationStatusCount, UpdateCommunityRule, UpdateEnforcementAction,
    UpdateViolation, UpdateViolationAppeal, Violation, ViolationAppeal, ViolationCategory,
    ViolationComment, ViolationDashboard, ViolationDetail, ViolationEvidence,
    ViolationNotification, ViolationQuery, ViolationSeverity, ViolationStatistics, ViolationStatus,
    ViolationSummary, ViolatorHistory,
};

// Epic 143: Board Meeting Management
pub mod board_meetings;

// Epic 144: Portfolio Performance Analytics
pub mod portfolio_performance;
pub use board_meetings::{
    ActionItemQuery as BoardMeetingActionItemQuery,
    ActionItemSummary as BoardMeetingActionItemSummary, AgendaItemStatus, AttendanceHistory,
    AttendanceStatus, BoardMeeting, BoardMember, BoardMemberQuery, BoardMemberSummary, BoardRole,
    CastVote as CastBoardVote, CreateActionItem as CreateBoardMeetingActionItem, CreateAgendaItem,
    CreateBoardMeeting, CreateBoardMember, CreateMinutes, CreateMotion as CreateBoardMotion,
    MeetingActionItem, MeetingAgendaItem, MeetingAttendance, MeetingDashboard, MeetingDetail,
    MeetingDocument, MeetingMinutes, MeetingMotion, MeetingQuery, MeetingStatistics, MeetingStatus,
    MeetingSummary, MeetingType, MeetingTypeCount, MotionDetail, MotionQuery, MotionStatus,
    MotionStatusCount, MotionSummary, MotionVote, RecordAttendance,
    UpdateActionItem as UpdateBoardMeetingActionItem, UpdateAgendaItem, UpdateBoardMeeting,
    UpdateBoardMember, UpdateMinutes, UpdateMotion as UpdateBoardMotion, UploadMeetingDocument,
    VoteChoice, VoteSummary as BoardMeetingVoteSummary,
};
pub use portfolio_performance::{
    BenchmarkComparison, BenchmarkComparisonSummary, BenchmarkSource, CalculateMetricsRequest,
    CashFlowTrendPoint, CreateBenchmarkComparison, CreateMarketBenchmark, CreatePerformanceAlert,
    CreatePerformancePortfolio, CreatePortfolioProperty,
    CreatePropertyTransaction as CreatePortfolioPropertyTransaction,
    DashboardQuery as PerformanceDashboardQuery, DashboardSummary, ExportPortfolioReport,
    ExportPortfolioReportResponse, FinancialMetrics, FinancingType, MarketBenchmark,
    MetricComparison, MetricPeriod, MetricsSummary, PerformanceAlert, PerformancePortfolio,
    PortfolioAnalyticsDashboard144, PortfolioMetricsSummary,
    PortfolioProperty as PerformancePortfolioProperty, PortfolioTransactionType, PropertyCashFlow,
    PropertyPerformanceCard, PropertyTransaction, TransactionQuery as PortfolioTransactionQuery,
    UpdateMarketBenchmark, UpdatePerformancePortfolio,
    UpdatePortfolioProperty as UpdatePerformancePortfolioProperty, UpdatePropertyTransaction,
    UpsertPropertyCashFlow, ValueTrendPoint,
};

// Epic 145: Multi-Currency & Cross-Border Support
pub mod multi_currency;

pub use multi_currency::{
    ConversionStatus, CountryCode, CreateCrossBorderLease, CreateCurrencyConfig,
    CreateExchangeRate, CreateMultiCurrencyTransaction, CreatePropertyCurrencyConfig,
    CreateReportConfig, CrossBorderComplianceRequirement, CrossBorderComplianceStatus,
    CrossBorderLease, CrossBorderLeaseQuery, CurrencyBreakdown, CurrencyConversionAudit,
    CurrencyDistribution, CurrencyExposureAnalysis, CurrencySummary, ExchangeRate,
    ExchangeRateFetchLog, ExchangeRateQuery, ExchangeRateSource, ExchangeRateSummary,
    GenerateReportRequest, MultiCurrencyDashboard, MultiCurrencyReportConfig,
    MultiCurrencyReportSnapshot, MultiCurrencyStatistics, MultiCurrencyTransaction,
    OrganizationCurrencyConfig, OverrideExchangeRate, PropertyBreakdown, PropertyCurrencyConfig,
    SupportedCurrency, TransactionQuery as MultiCurrencyTransactionQuery, UpdateCrossBorderLease,
    UpdateCurrencyConfig, UpdatePropertyCurrencyConfig, UpdateReportConfig, UpdateTransactionRate,
};

// Epic 146: Enhanced Data Residency Controls
pub mod data_residency;
pub use data_residency::{
    AccessType, AccessTypeCount, AuditChange, AuditLogEntry as ResidencyAuditLogEntry,
    AuditLogQuery as ResidencyAuditLogQuery, AuditLogResponse as ResidencyAuditLogResponse,
    AvailableRegionsResponse, ComplianceImplication, ComplianceIssue, ComplianceStatus,
    ComplianceVerificationResponse, ComplianceVerificationResult, ConfigureDataResidency,
    CreateAuditLogEntry, CrossRegionAccessLog, CrossRegionStats, DataLocationSummary, DataRegion,
    DataResidencyAuditLog, DataResidencyConfig, DataResidencyConfigResponse,
    DataResidencyDashboard, DataRoutingRule, DataRoutingStatus, DataTypeCategory, DataTypeOverride,
    ImplicationLevel, IssueSeverity, LogCrossRegionAccess, OutOfRegionData, RegionAccessSummary,
    RegionInfo, ResidencyAuditEvent, ResidencyStatus, RoutingRuleSummary,
    RunComplianceVerification,
};

// Epic 150: API Ecosystem Expansion
pub mod api_ecosystem;

pub use api_ecosystem::{
    api_doc_category, code_sample_language, connector_auth_type, connector_status,
    developer_status, ecosystem_webhook_event, installation_status, integration_category,
    marketplace_integration_status, webhook_auth_type, ApiCodeSample, ApiDocumentation,
    ApiEcosystemDashboard, ApiEcosystemStatistics, Connector, ConnectorAction,
    ConnectorExecutionLog, ConnectorExecutionQuery, CreateApiCodeSample, CreateApiDocumentation,
    CreateConnector, CreateConnectorAction, CreateDeveloperApiKey, CreateDeveloperApiKeyResponse,
    CreateDeveloperRegistration, CreateEnhancedWebhookSubscription, CreateIntegrationRating,
    CreateMarketplaceIntegration, CreatePreBuiltIntegrationConnection, CreateSandboxConfig,
    DeveloperApiKey, DeveloperApiKeyDisplay, DeveloperPortalStatistics, DeveloperRegistration,
    DeveloperUsageStats, EndpointUsageStats, EnhancedWebhookDeliveryLog, EnhancedWebhookStatistics,
    EnhancedWebhookSubscription, GoogleCalendarConfig, HubSpotConfig, InstallIntegration,
    IntegrationActivityLog, IntegrationCategoryCount, IntegrationRating, IntegrationRatingWithUser,
    MarketplaceIntegration, MarketplaceIntegrationQuery, MarketplaceIntegrationSummary,
    OrganizationIntegration, OutlookCalendarConfig, PreBuiltIntegrationConnection,
    PreBuiltIntegrationSyncResult, PreBuiltIntegrationType, QuickBooksConfig,
    ReviewDeveloperRegistration, SalesforceConfig, SandboxConfig, SandboxTestRequestPayload,
    SandboxTestResponsePayload, SlackConfig, SyncPreBuiltIntegrationRequest, TeamsConfig,
    UpdateApiDocumentation, UpdateConnector, UpdateEnhancedWebhookSubscription,
    UpdateMarketplaceIntegration, UpdateOrganizationIntegration,
    UpdatePreBuiltIntegrationConnection, WebhookRetryPolicyConfig, XeroConfig,
};

// Phase 3: Hosting & Theming — per-tenant feature flags + kill switches.
pub mod tenant_feature_flag;

pub use tenant_feature_flag::{TenantFeatureFlag, UpsertTenantFeatureFlag};

// Phase 2 — Defense N2: per-org auth policy (defends leak #13).
pub mod auth_policy;

pub use auth_policy::AuthPolicy;
