/**
 * Lazy-loaded Route Components
 * Epic 130: Performance Optimization
 *
 * Implements code splitting for feature pages to reduce initial bundle size.
 * Each feature module is loaded on-demand when the user navigates to it.
 */

import { lazy } from 'react';

// Documents feature (Epic 39)
export const DocumentsPage = lazy(() =>
  import('../features/documents').then((m) => ({ default: m.DocumentsPage }))
);
export const DocumentUploadPage = lazy(() =>
  import('../features/documents').then((m) => ({ default: m.DocumentUploadPage }))
);
export const DocumentDetailPage = lazy(() =>
  import('../features/documents').then((m) => ({ default: m.DocumentDetailPage }))
);
export const FolderTreePage = lazy(() =>
  import('../features/documents').then((m) => ({ default: m.FolderTreePage }))
);

// Document templates feature (Epic 7B, Story 7B.2)
export const DocumentTemplatesPage = lazy(() =>
  import('../features/document-templates').then((m) => ({ default: m.DocumentTemplatesPage }))
);

// News feature (Epic 59)
export const NewsListPage = lazy(() =>
  import('../features/news').then((m) => ({ default: m.NewsListPage }))
);
export const ArticleDetailPage = lazy(() =>
  import('../features/news').then((m) => ({ default: m.ArticleDetailPage }))
);

// Emergency feature (Epic 62)
export const EmergencyContactDirectoryPage = lazy(() =>
  import('../features/emergency').then((m) => ({ default: m.EmergencyContactDirectoryPage }))
);

// Voting feature (Epic 5: Building Voting & Decisions, FR37-44)
export const VotingPage = lazy(() =>
  import('../features/voting').then((m) => ({ default: m.VotingPage }))
);
export const VoteCreatePage = lazy(() =>
  import('../features/voting').then((m) => ({ default: m.VoteCreatePage }))
);
export const VoteDetailPage = lazy(() =>
  import('../features/voting').then((m) => ({ default: m.VoteDetailPage }))
);

// IoT / Smart-Building feature (Epic 14)
export const IotDashboardPage = lazy(() =>
  import('../features/iot').then((m) => ({ default: m.IotDashboardPage }))
);
export const IotSensorListPage = lazy(() =>
  import('../features/iot').then((m) => ({ default: m.SensorListPage }))
);
export const IotSensorFormPage = lazy(() =>
  import('../features/iot').then((m) => ({ default: m.SensorFormPage }))
);
export const IotAlertsPage = lazy(() =>
  import('../features/iot').then((m) => ({ default: m.IotAlertsPage }))
);
export const IotThresholdConfigPage = lazy(() =>
  import('../features/iot').then((m) => ({ default: m.IotThresholdConfigPage }))
);

// Disputes feature (Epic 77)
export const DisputesPage = lazy(() =>
  import('../features/disputes').then((m) => ({ default: m.DisputesPage }))
);
export const FileDisputePage = lazy(() =>
  import('../features/disputes').then((m) => ({ default: m.FileDisputePage }))
);
export const DisputeDetailPage = lazy(() =>
  import('../features/disputes').then((m) => ({ default: m.DisputeDetailPage }))
);
export const MediationWorkspacePage = lazy(() =>
  import('../features/disputes').then((m) => ({ default: m.MediationWorkspacePage }))
);

// Outages feature (UC-12)
export const OutagesPage = lazy(() =>
  import('../features/outages').then((m) => ({ default: m.OutagesPage }))
);
export const CreateOutagePage = lazy(() =>
  import('../features/outages').then((m) => ({ default: m.CreateOutagePage }))
);
export const ViewOutagePage = lazy(() =>
  import('../features/outages').then((m) => ({ default: m.ViewOutagePage }))
);
export const EditOutagePage = lazy(() =>
  import('../features/outages').then((m) => ({ default: m.EditOutagePage }))
);

// Settings features (Epic 60, 63)
export const AccessibilitySettingsPage = lazy(() =>
  import('../features/settings').then((m) => ({ default: m.AccessibilitySettingsPage }))
);
export const PrivacySettingsPage = lazy(() =>
  import('../features/privacy').then((m) => ({ default: m.PrivacySettingsPage }))
);

// Notification settings (Epic 8A + Epic 40)
export const NotificationSettingsPage = lazy(() =>
  import('../features/settings/notifications').then((m) => ({
    default: m.NotificationSettingsPage,
  }))
);
export const AdvancedNotificationSettingsPage = lazy(() =>
  import('../features/settings/notifications/advanced').then((m) => ({
    default: m.AdvancedNotificationSettingsPage,
  }))
);

// OAuth Grants feature (Epic 10A, Story 10A-3)
export const OAuthGrantsPage = lazy(() =>
  import('../features/oauth-grants').then((m) => ({ default: m.OAuthGrantsPage }))
);

// Neighbors feature (Epic 6, Story 6.6)
export const NeighborsPage = lazy(() =>
  import('../features/neighbors').then((m) => ({ default: m.NeighborsPage }))
);
export const NeighborDetailPage = lazy(() =>
  import('../features/neighbors').then((m) => ({ default: m.NeighborDetailPage }))
);
export const NeighborsPrivacySettingsPage = lazy(() =>
  import('../features/neighbors').then((m) => ({ default: m.PrivacySettingsPage }))
);

// Error / empty / loading state surfaces
export const NotFoundPage = lazy(() =>
  import('../features/errors').then((m) => ({ default: m.NotFoundPage }))
);
export const ForbiddenPage = lazy(() =>
  import('../features/errors').then((m) => ({ default: m.ForbiddenPage }))
);
export const ServerErrorPage = lazy(() =>
  import('../features/errors').then((m) => ({ default: m.ServerErrorPage }))
);
export const SessionExpiredPage = lazy(() =>
  import('../features/errors').then((m) => ({ default: m.SessionExpiredPage }))
);

// Auth pages (UC-14)
export const AuthCallbackPage = lazy(() =>
  import('../pages/AuthCallbackPage').then((m) => ({ default: m.AuthCallbackPage }))
);
export const LoginPage = lazy(() =>
  import('../pages/LoginPage').then((m) => ({ default: m.LoginPage }))
);
export const RegisterPage = lazy(() =>
  import('../features/auth').then((m) => ({ default: m.RegisterPage }))
);
export const ForgotPasswordPage = lazy(() =>
  import('../features/auth').then((m) => ({ default: m.ForgotPasswordPage }))
);
export const ResetPasswordPage = lazy(() =>
  import('../features/auth').then((m) => ({ default: m.ResetPasswordPage }))
);
export const ChangePasswordPage = lazy(() =>
  import('../features/auth').then((m) => ({ default: m.ChangePasswordPage }))
);
export const TwoFactorAuthPage = lazy(() =>
  import('../features/auth').then((m) => ({ default: m.TwoFactorAuthPage }))
);
export const ProfileEditPage = lazy(() =>
  import('../features/auth').then((m) => ({ default: m.ProfileEditPage }))
);

// Announcements feature (UC-06)
export const AnnouncementsPage = lazy(() =>
  import('../features/announcements').then((m) => ({ default: m.AnnouncementsPage }))
);
export const CreateAnnouncementPage = lazy(() =>
  import('../features/announcements').then((m) => ({ default: m.CreateAnnouncementPage }))
);
export const ViewAnnouncementPage = lazy(() =>
  import('../features/announcements').then((m) => ({ default: m.ViewAnnouncementPage }))
);
export const EditAnnouncementPage = lazy(() =>
  import('../features/announcements').then((m) => ({ default: m.EditAnnouncementPage }))
);

// Messaging feature (UC-07)
export const MessagesPage = lazy(() =>
  import('../features/messaging').then((m) => ({ default: m.MessagesPage }))
);
export const ThreadDetailPage = lazy(() =>
  import('../features/messaging').then((m) => ({ default: m.ThreadDetailPage }))
);
export const NewMessagePage = lazy(() =>
  import('../features/messaging').then((m) => ({ default: m.NewMessagePage }))
);

// Faults feature (UC-03)
export const FaultsPage = lazy(() =>
  import('../features/faults').then((m) => ({ default: m.FaultsPage }))
);
export const FaultDetailPage = lazy(() =>
  import('../features/faults').then((m) => ({ default: m.FaultDetailPage }))
);
export const CreateFaultPage = lazy(() =>
  import('../features/faults').then((m) => ({ default: m.CreateFaultPage }))
);
export const EditFaultPage = lazy(() =>
  import('../features/faults').then((m) => ({ default: m.EditFaultPage }))
);
export const FaultReportsPage = lazy(() =>
  import('../features/faults').then((m) => ({ default: m.FaultReportsPage }))
);

// Community feature (Epic 42)
export const FeedPage = lazy(() =>
  import('../features/community').then((m) => ({ default: m.FeedPage }))
);
export const GroupsPage = lazy(() =>
  import('../features/community').then((m) => ({ default: m.GroupsPage }))
);
export const GroupDetailPage = lazy(() =>
  import('../features/community').then((m) => ({ default: m.GroupDetailPage }))
);
export const CreateGroupPage = lazy(() =>
  import('../features/community').then((m) => ({ default: m.CreateGroupPage }))
);
export const EventsPage = lazy(() =>
  import('../features/community').then((m) => ({ default: m.EventsPage }))
);
export const MarketplacePage = lazy(() =>
  import('../features/community').then((m) => ({ default: m.MarketplacePage }))
);

// Financial feature (Epic 52)
export const FinancialDashboardPage = lazy(() =>
  import('../features/financial').then((m) => ({ default: m.FinancialDashboardPage }))
);
export const InvoiceManagementPage = lazy(() =>
  import('../features/financial').then((m) => ({ default: m.InvoiceManagementPage }))
);
export const PaymentManagementPage = lazy(() =>
  import('../features/financial').then((m) => ({ default: m.PaymentManagementPage }))
);
export const BudgetManagementPage = lazy(() =>
  import('../features/financial').then((m) => ({ default: m.BudgetManagementPage }))
);
// Story 11.7 - Financial statement reports (income statement / balance sheet / cash flow)
export const FinancialReportsPage = lazy(() =>
  import('../features/financial').then((m) => ({ default: m.FinancialReportsPage }))
);

// --- Accounting ---

export const AccountingInvoiceManagementPage = lazy(() =>
  import('../features/accounting').then((m) => ({ default: m.AccountingInvoiceManagementPage }))
);

export const PaymentMatchingPage = lazy(() =>
  import('../features/accounting').then((m) => ({ default: m.PaymentMatchingPage }))
);

// Reports feature (Epic 81)
export const ReportsPage = lazy(() =>
  import('../features/reports').then((m) => ({ default: m.ReportsPage }))
);
// Story 81.2 - Schedule detail with execution history table
export const ScheduleDetailPage = lazy(() =>
  import('../features/reports').then((m) => ({ default: m.ScheduleDetailPage }))
);

// Buildings feature (Epic 3, Story 3.1) — built + API-wired, now routed (gap-sweep)
export const BuildingsPage = lazy(() =>
  import('../features/buildings').then((m) => ({ default: m.BuildingsPage }))
);
export const BuildingDetailPage = lazy(() =>
  import('../features/buildings').then((m) => ({ default: m.BuildingDetailPage }))
);

// Resident "My Unit" view (Epic 3, Story 3.6) — gap-sweep
export const MyUnitPage = lazy(() =>
  import('../features/my-unit').then((m) => ({ default: m.MyUnitPage }))
);

// Facilities & bookings feature (Epic 3, Story 3.7) — API-wired, now routed (gap-sweep)
export const FacilitiesPage = lazy(() =>
  import('../features/facilities').then((m) => ({ default: m.FacilitiesPage }))
);
export const CreateFacilityPage = lazy(() =>
  import('../features/facilities').then((m) => ({ default: m.CreateFacilityPage }))
);
export const EditFacilityPage = lazy(() =>
  import('../features/facilities').then((m) => ({ default: m.EditFacilityPage }))
);
export const BookFacilityPage = lazy(() =>
  import('../features/facilities').then((m) => ({ default: m.BookFacilityPage }))
);
export const MyBookingsPage = lazy(() =>
  import('../features/facilities').then((m) => ({ default: m.MyBookingsPage }))
);
export const PendingBookingsPage = lazy(() =>
  import('../features/facilities').then((m) => ({ default: m.PendingBookingsPage }))
);

// AI Assistant chat (Epic 13, Story 13.1) — built, now routed (gap-sweep)
export const AiChatPage = lazy(() =>
  import('../features/ai-chat').then((m) => ({ default: m.AiChatPage }))
);

// Sentiment Dashboard (Epic 13, Story 13.2) — gap-sweep
export const SentimentDashboardPage = lazy(() =>
  import('../features/sentiment').then((m) => ({ default: m.SentimentDashboardPage }))
);

// Notification delivery analytics (Story 2B-C.3, PM #969 gap 4 / BIT-214)
export const NotificationAnalyticsPage = lazy(() =>
  import('../features/notification-analytics').then((m) => ({
    default: m.NotificationAnalyticsPage,
  }))
);

// Predictive Maintenance Dashboard (Epic 13, Story 13.3) — gap-sweep
export const PredictiveMaintenancePage = lazy(() =>
  import('../features/predictive-maintenance').then((m) => ({
    default: m.PredictiveMaintenancePage,
  }))
);

// Workflow Automation (Epic 13) — built + API-wired, now routed (gap-sweep)
export const AutomationRulesPage = lazy(() =>
  import('../features/workflow-automation').then((m) => ({ default: m.AutomationRulesPage }))
);
export const CreateRulePage = lazy(() =>
  import('../features/workflow-automation').then((m) => ({ default: m.CreateRulePage }))
);
export const EditRulePage = lazy(() =>
  import('../features/workflow-automation').then((m) => ({ default: m.EditRulePage }))
);
export const TemplateLibraryPage = lazy(() =>
  import('../features/workflow-automation').then((m) => ({ default: m.TemplateLibraryPage }))
);
export const ExecutionMonitoringPage = lazy(() =>
  import('../features/workflow-automation').then((m) => ({ default: m.ExecutionMonitoringPage }))
);

// Command Palette (Epic 129) - loaded eagerly since it's used globally
// CommandPaletteDialog and CommandPaletteProvider are NOT lazy loaded
// because they need to be available immediately for keyboard shortcuts
