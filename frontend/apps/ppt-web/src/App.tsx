import { MfaChallengeProvider } from '@ppt/admin-ui';
import type {
  Building as ApiBuilding,
  Dispute as ApiDispute,
  DisputeStatus as ApiDisputeStatus,
  DisputeType as ApiDisputeType,
  FaultSummary as ApiFaultSummary,
  OutageCommodity as ApiOutageCommodity,
  OutageSeverity as ApiOutageSeverity,
  OutageStatus as ApiOutageStatus,
  OutageSummary as ApiOutageSummary,
  FaultListQuery,
  OutageListQuery,
} from '@ppt/api-client';
import {
  uploadEvidence as apiUploadEvidence,
  getARAgingReport,
  getOverdueInvoices,
  getToken,
  listInvoices,
  ShortTermRentalsService,
  sendInvoice,
  setMfaChallengeHandler,
  useAcknowledgeAnnouncement,
  useAddAttachment,
  useAddComment,
  useAnnouncement,
  useAnnouncementAcknowledgmentStats,
  useAnnouncementComments,
  useAnnouncements,
  useArchiveAnnouncement,
  useBuildings,
  useCancelOutage,
  useConfirmFault,
  useCreateAnnouncementComment,
  useCreateDispute,
  useCreateFault,
  useCreateOutage,
  useDeleteAnnouncement,
  useDeleteAnnouncementComment,
  useDeleteAttachment,
  useDispute,
  useDisputeEvidence,
  useDisputes,
  useDisputeTimeline,
  useDownloadReport,
  useFault,
  useFaults,
  useMarkReadAnnouncement,
  useOutage,
  useOutages,
  usePinAnnouncement,
  usePinnedAnnouncements,
  usePublishAnnouncement,
  useReopenFault,
  useReportExecutionHistory,
  useResolveFault,
  useResolveOutage,
  useRetryReportExecution,
  useStartOutage,
  useTriageFault,
  useUpdateDisputeStatus,
  useUpdateFault,
  useUpdateOutage,
  useUpdateScheduleCron,
} from '@ppt/api-client';
import { AccessibilityProvider, SkipNavigation } from '@ppt/ui-kit';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { lazy, type ReactNode, Suspense, useCallback, useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  BrowserRouter,
  Link,
  Navigate,
  Route,
  Routes,
  useNavigate,
  useParams,
  useSearchParams,
} from 'react-router-dom';
import {
  AnnouncerProvider,
  AuthRequiredGate,
  ConnectionStatus,
  LanguageSwitcher,
  OfflineIndicator,
  ProtectedRoute,
  ToastProvider,
  useToast,
} from './components';
import {
  toApiSendMessageRequest,
  toStartThreadRequest,
  useDeleteMessage,
  useMarkThreadRead,
  useMessageRecipients,
  useSendMessage,
  useStartThread,
  useThread,
  useThreads,
  useUnreadCount,
} from './features/messaging/hooks/useMessaging';
import './styles/accessibility.css';
import './features/settings/styles/accessibility.css';
import { AuthProvider, OrganizationProvider, useAuth, WebSocketProvider } from './contexts';
import {
  CommandPaletteDialog,
  CommandPaletteProvider,
  useNavigationCommands,
} from './features/command-palette';
import { ManagerDashboardPage, ResidentDashboardPage } from './features/dashboard';
import type {
  DisputeCategory,
  DisputePriority,
  DisputeSummary,
  DisputeStatus as UiDisputeStatus,
} from './features/disputes/components/DisputeCard';
import type { ActivityType } from './features/disputes/components/DisputeTimeline';
import type { DisputeDetail as UiDisputeDetail } from './features/disputes/pages/DisputeDetailPage';
import { useNeighbors, usePrivacySettings } from './features/neighbors';
import type { ListOutagesParams, OutageDetail } from './features/outages';
import type {
  BookingListParams,
  BookingWithGuests,
  RentalBooking,
  BookingStatus as RentalBookingStatus,
  CalendarEvent as RentalCalendarEvent,
  RentalGuest,
  PlatformConnection as RentalPlatformConnection,
  PlatformType as RentalPlatformType,
  RentalStatistics,
} from './features/rentals/types';
// Lazy-loaded route components for code splitting (Epic 130)
import {
  AccessibilitySettingsPage,
  AiChatPage,
  AnnouncementsPage,
  ArticleDetailPage,
  AuthCallbackPage,
  AutomationRulesPage,
  BookFacilityPage,
  BudgetManagementPage,
  BuildingDetailPage,
  BuildingsPage,
  ChangePasswordPage,
  CreateAnnouncementPage,
  CreateFacilityPage,
  CreateFaultPage,
  CreateGroupPage,
  CreateOutagePage,
  CreateRulePage,
  DisputeDetailPage,
  DisputesPage,
  DocumentDetailPage,
  DocumentsPage,
  DocumentUploadPage,
  EditAnnouncementPage,
  EditFacilityPage,
  EditFaultPage,
  EditOutagePage,
  EditRulePage,
  EmergencyContactDirectoryPage,
  EventsPage,
  ExecutionMonitoringPage,
  FacilitiesPage,
  FaultDetailPage,
  FaultsPage,
  FeedPage,
  FileDisputePage,
  FinancialDashboardPage,
  FolderTreePage,
  ForbiddenPage,
  ForgotPasswordPage,
  GroupDetailPage,
  GroupsPage,
  InvoiceManagementPage,
  LoginPage,
  MarketplacePage,
  MediationWorkspacePage,
  MessagesPage,
  MyBookingsPage,
  NeighborDetailPage,
  NeighborsPage,
  NeighborsPrivacySettingsPage,
  NewMessagePage,
  NewsListPage,
  NotFoundPage,
  OAuthGrantsPage,
  OutagesPage,
  PaymentManagementPage,
  PendingBookingsPage,
  PrivacySettingsPage,
  ProfileEditPage,
  RegisterPage,
  ReportsPage,
  ResetPasswordPage,
  ScheduleDetailPage,
  ServerErrorPage,
  SessionExpiredPage,
  TemplateLibraryPage,
  ThreadDetailPage,
  TwoFactorAuthPage,
  ViewAnnouncementPage,
  ViewOutagePage,
} from './routes';

// Rentals feature pages (Epic 18 / UC-29, UC-30) — lazy-loaded for code splitting.
const RentalsDashboardPage = lazy(() =>
  import('./features/rentals').then((m) => ({ default: m.RentalsDashboardPage }))
);
const PlatformConnectionsPage = lazy(() =>
  import('./features/rentals').then((m) => ({ default: m.PlatformConnectionsPage }))
);
const BookingsPage = lazy(() =>
  import('./features/rentals').then((m) => ({ default: m.BookingsPage }))
);
const BookingDetailPage = lazy(() =>
  import('./features/rentals').then((m) => ({ default: m.BookingDetailPage }))
);
const RentalsCalendarPage = lazy(() =>
  import('./features/rentals').then((m) => ({ default: m.CalendarPage }))
);
const GuestRegistrationPage = lazy(() =>
  import('./features/rentals').then((m) => ({ default: m.GuestRegistrationPage }))
);
const TaxReportPage = lazy(() =>
  import('./features/rentals').then((m) => ({ default: m.TaxReportPage }))
);

// Sessions management page (#966) — lazy-loaded.
const SessionsPage = lazy(() =>
  import('./features/settings/pages/SessionsPage').then((m) => ({ default: m.SessionsPage }))
);

// ============================================
// Type Mapping Utilities (API <-> UI)
// ============================================

/** Map API DisputeType to UI DisputeCategory */
function mapTypeToCategory(type: ApiDisputeType): DisputeCategory {
  const mapping: Record<ApiDisputeType, DisputeCategory> = {
    noise: 'noise',
    damage: 'damage',
    payment: 'payment',
    lease: 'lease_terms',
    maintenance: 'maintenance',
    other: 'other',
  };
  return mapping[type];
}

/** Map UI DisputeCategory to API DisputeType */
function mapCategoryToType(category: DisputeCategory): ApiDisputeType {
  const mapping: Record<DisputeCategory, ApiDisputeType> = {
    noise: 'noise',
    damage: 'damage',
    payment: 'payment',
    lease_terms: 'lease',
    common_area: 'other',
    parking: 'other',
    pets: 'other',
    maintenance: 'maintenance',
    privacy: 'other',
    harassment: 'other',
    other: 'other',
  };
  return mapping[category];
}

/** Map API DisputeStatus to UI DisputeStatus */
function mapApiStatusToUiStatus(status: ApiDisputeStatus): UiDisputeStatus {
  const mapping: Record<ApiDisputeStatus, UiDisputeStatus> = {
    filed: 'filed',
    under_review: 'under_review',
    mediation: 'mediation',
    escalated: 'escalated',
    resolved: 'resolved',
    closed: 'closed',
  };
  return mapping[status];
}

/** Map UI DisputeStatus to API DisputeStatus (for filtering) */
function mapUiStatusToApiStatus(status: UiDisputeStatus): ApiDisputeStatus | undefined {
  const mapping: Record<UiDisputeStatus, ApiDisputeStatus | undefined> = {
    filed: 'filed',
    under_review: 'under_review',
    mediation: 'mediation',
    awaiting_response: 'under_review', // No direct mapping, use under_review
    resolved: 'resolved',
    escalated: 'escalated',
    withdrawn: 'closed', // No direct mapping, use closed
    closed: 'closed',
  };
  return mapping[status];
}

/**
 * Map API TimelineEventType to UI ActivityType (story 80-3).
 * Most event names overlap; the few that differ are approximated.
 */
function mapTimelineEventType(
  eventType: import('@ppt/api-client').TimelineEventType
): ActivityType {
  const mapping: Record<import('@ppt/api-client').TimelineEventType, ActivityType> = {
    dispute_filed: 'dispute_filed',
    status_changed: 'status_changed',
    mediator_assigned: 'party_added',
    evidence_added: 'evidence_added',
    note_added: 'comment_added',
    meeting_scheduled: 'session_scheduled',
    resolution_proposed: 'resolution_proposed',
    resolution_accepted: 'resolution_accepted',
    escalated: 'escalated',
    closed: 'closed',
  };
  return mapping[eventType] ?? 'status_changed';
}

/** Format API Building address to string for UI components */
function formatBuildingAddress(building: ApiBuilding): string {
  const { street, city, postalCode } = building.address;
  return `${street}, ${postalCode} ${city}`;
}

/** Transform API Building to UI building format */
function transformBuildingForUI(building: ApiBuilding): {
  id: string;
  name: string;
  address: string;
} {
  return {
    id: building.id,
    name: building.name,
    address: formatBuildingAddress(building),
  };
}

/** Transform API Dispute to UI DisputeSummary */
function transformDisputeToSummary(dispute: ApiDispute): DisputeSummary {
  return {
    id: dispute.id,
    referenceNumber: `DSP-${dispute.id.toUpperCase()}`,
    category: mapTypeToCategory(dispute.type),
    title: dispute.subject,
    status: mapApiStatusToUiStatus(dispute.status),
    // Priority is UI-only; API does not support priority field yet
    priority: 'medium' as DisputePriority,
    filedByName: dispute.filedBy,
    assignedToName: dispute.assignedMediator,
    partyCount: dispute.respondentId || dispute.respondent ? 2 : 1,
    createdAt: dispute.createdAt,
    updatedAt: dispute.updatedAt,
  };
}

/**
 * MFA challenge wrapper (N9).
 *
 * Mounts `<MfaChallengeProvider>` so the api-client's mfa_required
 * interceptor has somewhere to send the challenge. The provider needs
 * two things from this layer that the package itself cannot reach:
 *
 *   * `setMfaChallengeHandler` — the api-client global registration
 *     point. Passed in so admin-ui has no @ppt/api-client dependency.
 *   * `verify(code)` — POST {code} to /api/v1/auth/mfa/verify with the
 *     current bearer token. Built from `AuthContext.getAccessToken`.
 *
 * The verify endpoint is `/api/v1/auth/mfa/verify` (api-server's
 * routes/mfa.rs). On 200 the user is freshly MFA-verified for the
 * RECENT_MFA_WINDOW (15 min); the api-client retries the original
 * mutation transparently.
 *
 * Must be rendered inside AuthProvider so getAccessToken is available.
 */
function MfaWrapper({ children }: { children: ReactNode }) {
  const { getAccessToken } = useAuth();
  const { t } = useTranslation();
  // Resolve labels once per render so the modal sees stable strings even
  // when the active locale changes (the next render swaps them out).
  const mfaLabels = {
    title: t('admin.mfa.title'),
    description: t('admin.mfa.description'),
    descriptionForAction: t('admin.mfa.descriptionForAction', { action: '{action}' }),
    codeLabel: t('admin.mfa.label'),
    verify: t('admin.mfa.verify'),
    verifying: t('admin.mfa.verifying'),
    cancel: t('admin.mfa.cancel'),
    invalidFormat: t('admin.mfa.invalidFormat'),
    invalidCode: t('admin.mfa.invalidCode'),
    verificationFailed: t('admin.mfa.verificationFailed'),
  };
  return (
    <MfaChallengeProvider
      registerHandler={setMfaChallengeHandler}
      labels={mfaLabels}
      verify={async (code: string) => {
        const token = getAccessToken();
        const headers: Record<string, string> = { 'Content-Type': 'application/json' };
        if (token) headers.Authorization = `Bearer ${token}`;
        try {
          const response = await fetch('/api/v1/auth/mfa/verify', {
            method: 'POST',
            headers,
            body: JSON.stringify({ code }),
          });
          return response.ok;
        } catch {
          return false;
        }
      }}
    >
      {children}
    </MfaChallengeProvider>
  );
}

/**
 * WebSocket wrapper that bridges AuthContext to WebSocketProvider.
 * Must be rendered inside AuthProvider to access auth state.
 */
function WebSocketWrapper({ children }: { children: ReactNode }) {
  const { getAccessToken, isAuthenticated } = useAuth();
  return (
    <WebSocketProvider
      auth={{
        accessToken: getAccessToken(),
        isAuthenticated,
      }}
    >
      {children}
    </WebSocketProvider>
  );
}

function AppNavigation() {
  const { t } = useTranslation();
  const { isAuthenticated, logout } = useAuth();
  const navigate = useNavigate();

  const handleLogout = async () => {
    await logout();
    navigate('/login', { replace: true });
  };

  return (
    <nav className="app-nav" aria-label="Main navigation">
      <Link to="/">{t('nav.home')}</Link>
      <Link to="/buildings">{t('nav.buildings')}</Link>
      <Link to="/documents">{t('nav.documents')}</Link>
      <Link to="/ai-assistant">{t('nav.aiChat')}</Link>
      <Link to="/news">{t('nav.news')}</Link>
      <Link to="/emergency">{t('nav.emergency')}</Link>
      <Link to="/disputes">{t('nav.disputes')}</Link>
      <Link to="/outages">{t('nav.outages')}</Link>
      <Link to="/rentals">{t('nav.rentals', { defaultValue: 'Rentals' })}</Link>
      <Link to="/settings/accessibility">{t('nav.accessibility')}</Link>
      <Link to="/settings/privacy">{t('nav.privacy')}</Link>
      <Link to="/settings/sessions">{t('nav.sessions', { defaultValue: 'Sessions' })}</Link>
      {/* Super-admin moved to admin.rlt.sk (separate SPA) — no nav link here. */}
      <div className="ml-auto flex items-center gap-3">
        {isAuthenticated && <ConnectionStatus />}
        <LanguageSwitcher />
        {isAuthenticated && (
          <button
            type="button"
            className="btn-outline-token px-3 py-1.5 rounded-lg text-sm font-medium"
            onClick={handleLogout}
            aria-label={t('auth.signOut', { defaultValue: 'Sign out' })}
          >
            {t('auth.signOut', { defaultValue: 'Sign out' })}
          </button>
        )}
        {!isAuthenticated && (
          <Link
            to="/login"
            className="btn-primary-token px-3 py-1.5 rounded-lg text-sm font-medium"
          >
            {t('auth.signIn')}
          </Link>
        )}
      </div>
    </nav>
  );
}

function RouteLoading() {
  const { t } = useTranslation();
  return (
    <div
      className="flex items-center justify-center py-16"
      role="status"
      aria-live="polite"
      aria-busy="true"
    >
      <div
        className="h-8 w-8 animate-spin rounded-full border-2 border-blue-600 border-t-transparent"
        aria-hidden="true"
      />
      <span className="sr-only">{t('common.loading')}</span>
    </div>
  );
}

/**
 * Command Palette wrapper that registers navigation commands.
 * Must be rendered inside BrowserRouter for useNavigate to work.
 */
function CommandPaletteWrapper({ children }: { children: ReactNode }) {
  useNavigationCommands();
  return (
    <>
      <CommandPaletteDialog />
      {children}
    </>
  );
}

function App() {
  return (
    <AccessibilityProvider>
      <AuthProvider>
        <OrganizationProvider>
          <ToastProvider>
            <AnnouncerProvider>
              <MfaWrapper>
                <WebSocketWrapper>
                  <BrowserRouter>
                    <CommandPaletteProvider>
                      <CommandPaletteWrapper>
                        <SkipNavigation mainContentId="main-content" />
                        <OfflineIndicator />
                        <div className="app">
                          {/* Phase 5 impersonation banner moved with the
                              admin SPA to admin.rlt.sk. ppt-web tenants
                              never see impersonation tokens directly. */}
                          <AppNavigation />
                          <main id="main-content">
                            <Suspense fallback={<RouteLoading />}>
                              <Routes>
                                <Route path="/" element={<Home />} />
                                <Route path="/login" element={<LoginPage />} />
                                {/* SSO / OAuth callback route (gap-79-2).
                                    Handles provider redirect: reads ?code=…&state=…,
                                    exchanges for PPT JWT tokens via tokenProvider,
                                    then redirects to /dashboard or stored return URL. */}
                                <Route path="/auth/callback" element={<AuthCallbackPage />} />
                                <Route path="/register" element={<RegisterPage />} />
                                <Route path="/forgot-password" element={<ForgotPasswordPage />} />
                                <Route path="/reset-password" element={<ResetPasswordPage />} />
                                <Route path="/settings/password" element={<ChangePasswordPage />} />
                                <Route
                                  path="/settings/two-factor"
                                  element={<TwoFactorAuthPage />}
                                />
                                <Route path="/settings/profile" element={<ProfileEditPage />} />
                                {/* Dashboard routes (Epic 124).
                                  Bare /dashboard 404'd previously — redirect
                                  it to the manager dashboard so users typing
                                  the obvious URL get something useful (the
                                  ProtectedRoute / role check on the target
                                  page handles auth + role-based fan-out). */}
                                <Route
                                  path="/dashboard"
                                  element={<Navigate to="/dashboard/manager" replace />}
                                />
                                <Route
                                  path="/dashboard/manager"
                                  element={<ManagerDashboardPage />}
                                />
                                <Route
                                  path="/dashboard/resident"
                                  element={<ResidentDashboardPage />}
                                />
                                {/* Document Intelligence routes (Epic 39) */}
                                <Route
                                  path="/documents"
                                  element={
                                    <ProtectedRoute>
                                      <DocumentsPageRoute />
                                    </ProtectedRoute>
                                  }
                                />
                                <Route
                                  path="/documents/folders"
                                  element={
                                    <ProtectedRoute>
                                      <FolderTreePageRoute />
                                    </ProtectedRoute>
                                  }
                                />
                                <Route
                                  path="/documents/upload"
                                  element={
                                    <ProtectedRoute>
                                      <DocumentUploadPage />
                                    </ProtectedRoute>
                                  }
                                />
                                <Route
                                  path="/documents/:documentId"
                                  element={
                                    <ProtectedRoute>
                                      <DocumentDetailRoute />
                                    </ProtectedRoute>
                                  }
                                />
                                {/* News routes (Epic 59) */}
                                <Route
                                  path="/news"
                                  element={
                                    <ProtectedRoute>
                                      <NewsListPage />
                                    </ProtectedRoute>
                                  }
                                />
                                <Route
                                  path="/news/:articleId"
                                  element={
                                    <ProtectedRoute>
                                      <ArticleDetailRoute />
                                    </ProtectedRoute>
                                  }
                                />
                                {/* Emergency contacts route (Epic 62) */}
                                <Route
                                  path="/emergency"
                                  element={
                                    <ProtectedRoute>
                                      <EmergencyContactDirectoryPage />
                                    </ProtectedRoute>
                                  }
                                />
                                {/* Accessibility settings route (Epic 60) */}
                                <Route
                                  path="/settings/accessibility"
                                  element={<AccessibilitySettingsPage />}
                                />
                                {/* Privacy settings route (Epic 63) */}
                                <Route path="/settings/privacy" element={<PrivacySettingsPage />} />
                                {/* OAuth Grants management route (Epic 10A, Story 10A-3) */}
                                <Route
                                  path="/settings/oauth-grants"
                                  element={
                                    <ProtectedRoute>
                                      <OAuthGrantsPage />
                                    </ProtectedRoute>
                                  }
                                />
                                {/* Dispute Resolution routes (Epic 77) */}
                                <Route path="/disputes" element={<DisputesPageRoute />} />
                                <Route path="/disputes/new" element={<FileDisputePageRoute />} />
                                <Route
                                  path="/disputes/:disputeId"
                                  element={<DisputeDetailRoute />}
                                />
                                <Route
                                  path="/disputes/:disputeId/mediation"
                                  element={<DisputeMediationRoute />}
                                />
                                {/* Outages routes (UC-12) */}
                                <Route path="/outages" element={<OutagesPageRoute />} />
                                <Route path="/outages/new" element={<CreateOutagePageRoute />} />
                                <Route
                                  path="/outages/:outageId"
                                  element={<ViewOutagePageRoute />}
                                />
                                <Route
                                  path="/outages/:outageId/edit"
                                  element={<EditOutagePageRoute />}
                                />
                                {/* Announcements routes (UC-06) */}
                                <Route path="/announcements" element={<AnnouncementsPageRoute />} />
                                <Route
                                  path="/announcements/new"
                                  element={<CreateAnnouncementPageRoute />}
                                />
                                <Route
                                  path="/announcements/:announcementId"
                                  element={<ViewAnnouncementPageRoute />}
                                />
                                <Route
                                  path="/announcements/:announcementId/edit"
                                  element={<EditAnnouncementPageRoute />}
                                />
                                {/* Messaging routes (UC-07) */}
                                <Route path="/messages" element={<MessagesPageRoute />} />
                                <Route path="/messages/new" element={<NewMessagePageRoute />} />
                                <Route
                                  path="/messages/:threadId"
                                  element={<ThreadDetailPageRoute />}
                                />
                                {/* Neighbors routes (Epic 6, Story 6.6) */}
                                <Route path="/neighbors" element={<NeighborsPageRoute />} />
                                <Route
                                  path="/neighbors/:neighborId"
                                  element={<NeighborDetailRoute />}
                                />
                                <Route
                                  path="/neighbors/privacy"
                                  element={<NeighborsPrivacySettingsRoute />}
                                />
                                {/* Faults routes (UC-03) */}
                                <Route path="/faults" element={<FaultsPageRoute />} />
                                <Route path="/faults/new" element={<CreateFaultPageRoute />} />
                                <Route path="/faults/:faultId" element={<FaultDetailPageRoute />} />
                                <Route
                                  path="/faults/:faultId/edit"
                                  element={<EditFaultPageRoute />}
                                />
                                {/* Community routes (Epic 42) */}
                                <Route path="/community" element={<FeedPageRoute />} />
                                <Route path="/community/groups" element={<GroupsPageRoute />} />
                                <Route
                                  path="/community/groups/new"
                                  element={<CreateGroupPageRoute />}
                                />
                                <Route
                                  path="/community/groups/:groupId"
                                  element={<GroupDetailPageRoute />}
                                />
                                <Route path="/community/events" element={<EventsPageRoute />} />
                                <Route
                                  path="/community/marketplace"
                                  element={<MarketplacePageRoute />}
                                />
                                {/* Financial routes (Epic 52) */}
                                <Route
                                  path="/financial"
                                  element={<FinancialDashboardPageRoute />}
                                />
                                <Route
                                  path="/financial/invoices"
                                  element={<InvoiceManagementPageRoute />}
                                />
                                <Route
                                  path="/financial/payments"
                                  element={<PaymentManagementPageRoute />}
                                />
                                <Route
                                  path="/financial/budgets"
                                  element={<BudgetManagementPageRoute />}
                                />

                                {/* Short-Term Rentals routes (Epic 18, UC-29/30) */}
                                <Route
                                  path="/rentals"
                                  element={
                                    <ProtectedRoute>
                                      <RentalsDashboardPageRoute />
                                    </ProtectedRoute>
                                  }
                                />
                                <Route
                                  path="/rentals/connections"
                                  element={
                                    <ProtectedRoute>
                                      <PlatformConnectionsPageRoute />
                                    </ProtectedRoute>
                                  }
                                />
                                <Route
                                  path="/rentals/bookings"
                                  element={
                                    <ProtectedRoute>
                                      <BookingsPageRoute />
                                    </ProtectedRoute>
                                  }
                                />
                                <Route
                                  path="/rentals/bookings/:bookingId"
                                  element={
                                    <ProtectedRoute>
                                      <BookingDetailPageRoute />
                                    </ProtectedRoute>
                                  }
                                />
                                <Route
                                  path="/rentals/calendar"
                                  element={
                                    <ProtectedRoute>
                                      <RentalsCalendarPageRoute />
                                    </ProtectedRoute>
                                  }
                                />
                                <Route
                                  path="/rentals/guests"
                                  element={
                                    <ProtectedRoute>
                                      <GuestRegistrationPageRoute />
                                    </ProtectedRoute>
                                  }
                                />
                                <Route
                                  path="/rentals/reports"
                                  element={
                                    <ProtectedRoute>
                                      <TaxReportPageRoute />
                                    </ProtectedRoute>
                                  }
                                />

                                {/* Active sessions management (#966) */}
                                <Route
                                  path="/settings/sessions"
                                  element={
                                    <ProtectedRoute>
                                      <SessionsPage />
                                    </ProtectedRoute>
                                  }
                                />

                                {/* Reports routes (Epic 81) */}
                                <Route
                                  path="/reports"
                                  element={
                                    <ProtectedRoute>
                                      <ReportsPageRoute />
                                    </ProtectedRoute>
                                  }
                                />
                                {/* Story 81.2: Schedule detail with inline execution history table */}
                                <Route
                                  path="/reports/schedules/:scheduleId"
                                  element={
                                    <ProtectedRoute>
                                      <ScheduleDetailPageRoute />
                                    </ProtectedRoute>
                                  }
                                />

                                {/* Buildings management (Epic 3, Story 3.1) — gap-sweep */}
                                <Route
                                  path="/buildings"
                                  element={
                                    <ProtectedRoute>
                                      <BuildingsPageRoute />
                                    </ProtectedRoute>
                                  }
                                />
                                <Route
                                  path="/buildings/:buildingId"
                                  element={
                                    <ProtectedRoute>
                                      <BuildingDetailPageRoute />
                                    </ProtectedRoute>
                                  }
                                />
                                {/* Facilities & bookings (Epic 3, Story 3.7) — gap-sweep.
                                  Pages self-resolve params via useParams/useNavigate. */}
                                <Route
                                  path="/buildings/:buildingId/facilities"
                                  element={
                                    <ProtectedRoute>
                                      <FacilitiesPage />
                                    </ProtectedRoute>
                                  }
                                />
                                <Route
                                  path="/buildings/:buildingId/facilities/new"
                                  element={
                                    <ProtectedRoute>
                                      <CreateFacilityPage />
                                    </ProtectedRoute>
                                  }
                                />
                                <Route
                                  path="/buildings/:buildingId/facilities/:facilityId/edit"
                                  element={
                                    <ProtectedRoute>
                                      <EditFacilityPage />
                                    </ProtectedRoute>
                                  }
                                />
                                <Route
                                  path="/buildings/:buildingId/facilities/:facilityId/book"
                                  element={
                                    <ProtectedRoute>
                                      <BookFacilityPage />
                                    </ProtectedRoute>
                                  }
                                />
                                <Route
                                  path="/buildings/:buildingId/facilities/pending"
                                  element={
                                    <ProtectedRoute>
                                      <PendingBookingsPage />
                                    </ProtectedRoute>
                                  }
                                />
                                <Route
                                  path="/facilities/bookings/my"
                                  element={
                                    <ProtectedRoute>
                                      <MyBookingsPage />
                                    </ProtectedRoute>
                                  }
                                />
                                {/* AI Assistant chat (Epic 13, Story 13.1) — gap-sweep */}
                                <Route
                                  path="/ai-assistant"
                                  element={
                                    <ProtectedRoute>
                                      <AiChatPage />
                                    </ProtectedRoute>
                                  }
                                />
                                {/* Workflow Automation (Epic 13) — gap-sweep.
                                  Paths match the pages' hard-coded navigate() targets. */}
                                <Route
                                  path="/automations/rules"
                                  element={
                                    <ProtectedRoute>
                                      <AutomationRulesPage />
                                    </ProtectedRoute>
                                  }
                                />
                                <Route
                                  path="/automations/rules/new"
                                  element={
                                    <ProtectedRoute>
                                      <CreateRulePage />
                                    </ProtectedRoute>
                                  }
                                />
                                <Route
                                  path="/automations/rules/:id/edit"
                                  element={
                                    <ProtectedRoute>
                                      <EditRulePage />
                                    </ProtectedRoute>
                                  }
                                />
                                <Route
                                  path="/automations/templates"
                                  element={
                                    <ProtectedRoute>
                                      <TemplateLibraryPage />
                                    </ProtectedRoute>
                                  }
                                />
                                <Route
                                  path="/automations/executions"
                                  element={
                                    <ProtectedRoute>
                                      <ExecutionMonitoringPage />
                                    </ProtectedRoute>
                                  }
                                />

                                {/* Phase 5 admin is at admin.rlt.sk now;
                                  see frontend/apps/admin-web. */}

                                {/* Error / state surfaces */}
                                <Route path="/forbidden" element={<ForbiddenPage />} />
                                <Route path="/server-error" element={<ServerErrorPage />} />
                                <Route path="/session-expired" element={<SessionExpiredPage />} />
                                <Route path="*" element={<NotFoundPage />} />
                              </Routes>
                            </Suspense>
                          </main>
                        </div>
                      </CommandPaletteWrapper>
                    </CommandPaletteProvider>
                  </BrowserRouter>
                </WebSocketWrapper>
              </MfaWrapper>
            </AnnouncerProvider>
          </ToastProvider>
        </OrganizationProvider>
      </AuthProvider>
    </AccessibilityProvider>
  );
}

/** Route wrapper for documents page */
function DocumentsPageRoute() {
  const { user } = useAuth();
  const organizationId = user?.organizationId ?? 'default-org';
  return <DocumentsPage organizationId={organizationId} />;
}

/** Route wrapper for buildings list (Epic 3, Story 3.1) — gap-sweep */
function BuildingsPageRoute() {
  const navigate = useNavigate();
  return (
    <BuildingsPage
      onNavigateToView={(id) => navigate(`/buildings/${id}`)}
      onNavigateToEdit={(id) => navigate(`/buildings/${id}`)}
    />
  );
}

/** Route wrapper for building detail (Epic 3, Story 3.1) — gap-sweep */
function BuildingDetailPageRoute() {
  const { buildingId } = useParams<{ buildingId: string }>();
  if (!buildingId) {
    return <div>Building not found</div>;
  }
  return <BuildingDetailPage buildingId={buildingId} />;
}

/** Route wrapper for folder-tree page (gap-7a-2) */
function FolderTreePageRoute() {
  const { user } = useAuth();
  const organizationId = user?.organizationId ?? 'default-org';
  return <FolderTreePage organizationId={organizationId} />;
}

/** Route wrapper for document detail page to extract params */
function DocumentDetailRoute() {
  const { t } = useTranslation();
  const { documentId } = useParams<{ documentId: string }>();
  if (!documentId) {
    return (
      <div className="error-page">
        <h1>{t('errors.documentNotFound')}</h1>
        <p>{t('errors.documentNotFoundDesc')}</p>
        <Link to="/documents">{t('common.backToDocuments')}</Link>
      </div>
    );
  }
  return <DocumentDetailPage documentId={documentId} />;
}

/** Route wrapper for article detail page to extract params */
function ArticleDetailRoute() {
  const { t } = useTranslation();
  const { articleId } = useParams<{ articleId: string }>();
  if (!articleId) return <div>{t('errors.articleNotFound')}</div>;
  return <ArticleDetailPage articleId={articleId} />;
}

/**
 * Route wrapper for disputes page (Epic 77, Story 80.1).
 *
 * Uses useDisputes hook from @ppt/api-client for data fetching.
 * Implements real API integration with TanStack Query.
 * Transforms API types to UI types for component compatibility.
 */
function DisputesPageRoute() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { user } = useAuth();
  const { showToast } = useToast();

  // Require organization context for disputes
  if (!user?.organizationId) {
    return <AuthRequiredGate />;
  }

  const organizationId = user.organizationId;

  // Filter state for API query (UI types)
  // Note: priority and search are accepted from UI for future compatibility,
  // but the current DisputeListQuery API does not support these fields.
  // These will be ignored until backend API is extended to support them.
  const [filters, setFilters] = useState<{
    status?: UiDisputeStatus;
    priority?: DisputePriority; // UI-only: API does not support priority filtering yet
    category?: DisputeCategory;
    search?: string; // UI-only: API does not support text search yet
    page: number;
    pageSize: number;
  }>({ page: 1, pageSize: 10 });

  // Map UI filters to API query parameters
  // Note: priority and search are not passed to API as DisputeListQuery does not support them.
  // When backend adds support, update apiQuery to include these fields.
  const apiQuery = {
    status: filters.status ? mapUiStatusToApiStatus(filters.status) : undefined,
    type: filters.category ? mapCategoryToType(filters.category) : undefined,
    limit: filters.pageSize,
    page: filters.page,
  };

  // Use the disputes API hook
  const { data, isLoading, error } = useDisputes(organizationId, apiQuery);

  // Show error toast if query fails (use useEffect to prevent toast spam)
  useEffect(() => {
    if (error) {
      showToast({
        type: 'error',
        title: t('disputes.failedToLoad'),
        message: error instanceof Error ? error.message : t('auth.unexpectedError'),
      });
    }
  }, [error, showToast, t]);

  // Transform API response to match component interface
  const disputes: DisputeSummary[] = (data?.data ?? []).map(transformDisputeToSummary);
  const total = data?.total ?? 0;

  const handleNavigateToCreate = () => {
    navigate('/disputes/new');
  };
  const handleNavigateToView = (id: string) => {
    navigate(`/disputes/${id}`);
  };
  const handleNavigateToManage = (id: string) => {
    navigate(`/disputes/${id}`);
  };
  const handleFilterChange = (newFilters: {
    status?: UiDisputeStatus;
    priority?: DisputePriority;
    category?: DisputeCategory;
    search?: string;
    page: number;
    pageSize: number;
  }) => {
    setFilters(newFilters);
  };

  return (
    <DisputesPage
      disputes={disputes}
      total={total}
      isLoading={isLoading}
      onNavigateToCreate={handleNavigateToCreate}
      onNavigateToView={handleNavigateToView}
      onNavigateToManage={handleNavigateToManage}
      onFilterChange={handleFilterChange}
    />
  );
}

/**
 * Route wrapper for file dispute page (Epic 80, Story 80.2).
 *
 * Step sequence:
 *  1. POST /api/v1/disputes/organizations/:orgId  → creates the dispute (returns id)
 *  2. For each valid PendingEvidence file: POST /api/v1/disputes/:id/evidence
 *  3. Navigate to /disputes/:id (detail page) on success.
 *
 * FileDisputePage is a pure presentational component; all API side-effects live here.
 */
function FileDisputePageRoute() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { user } = useAuth();
  const { showToast } = useToast();
  const [isSubmitting, setIsSubmitting] = useState(false);

  const organizationId = user?.organizationId ?? '';

  const createDispute = useCreateDispute(organizationId);

  if (!user?.organizationId) {
    return <AuthRequiredGate />;
  }

  const handleSubmit = async (
    payload: import('./features/disputes/pages/FileDisputePage').FileDisputeSubmitPayload
  ) => {
    setIsSubmitting(true);
    try {
      // Step 1 — create the dispute
      const created = await createDispute.mutateAsync({
        type: payload.values.type as ApiDisputeType,
        subject: payload.values.subject,
        description: payload.values.description,
        unitId: payload.values.unitId,
        respondentId: payload.values.respondentId || undefined,
      });

      // Step 2 — upload evidence files sequentially; skip errored entries.
      // We call the raw API function (not the hook) because the hook must be
      // keyed to a disputeId at render time and we only know the id after step 1.
      const validFiles = payload.evidence.filter((e) => e.status !== 'error');
      const failedEvidence: typeof validFiles = [];
      for (const item of validFiles) {
        try {
          await apiUploadEvidence(created.id, item.file, item.description || item.file.name);
        } catch {
          failedEvidence.push(item);
        }
      }
      const evidenceErrors = failedEvidence.length;

      // Step 3 — toast + navigate to the new dispute detail
      if (evidenceErrors > 0) {
        showToast({
          type: 'warning',
          title: t('disputes.filedWithEvidenceErrors', 'Dispute filed (some files failed)'),
          message: t('disputes.evidenceUploadErrorsMsg', { count: evidenceErrors }),
        });
      } else {
        showToast({
          type: 'success',
          title: t('disputes.filedSuccessfully', 'Dispute filed successfully'),
          message: t('disputes.submittedMsg', 'Your dispute has been submitted for review.'),
        });
      }
      // TODO(#627): consume failedEvidence on DisputeDetailPage to surface a retry
      // prompt once evidence-retry UI lands. For now we just thread it through
      // router state so the detail page can pick it up when ready.
      navigate(`/disputes/${created.id}`, {
        state: evidenceErrors > 0 ? { failedEvidence } : undefined,
      });
    } catch (error) {
      showToast({
        type: 'error',
        title: t('disputes.failedToFile', 'Failed to file dispute'),
        message: error instanceof Error ? error.message : t('auth.unexpectedError'),
      });
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <FileDisputePage
      onSubmit={handleSubmit}
      isSubmitting={isSubmitting || createDispute.isPending}
    />
  );
}

/**
 * Route wrapper for dispute detail page (Epic 77, Story 80-3).
 *
 * Replaces the inline JSX stub with the full DisputeDetailPage component.
 * Wires useDispute, useDisputeTimeline, useDisputeEvidence, and
 * useUpdateDisputeStatus from @ppt/api-client.
 *
 * API DisputeWithDetails → UI DisputeDetail mapping:
 *   - subject           → title
 *   - type              → category (via mapTypeToCategory)
 *   - status            → status (via mapApiStatusToUiStatus)
 *   - filedBy           → filedBy + filedByName
 *   - assignedMediator  → assignedToName
 *   - referenceNumber   synthesised as DSP-{id.toUpperCase()}
 *   - priority          UI-only, hardcoded 'medium' until API exposes it
 */
function DisputeDetailRoute() {
  const { t } = useTranslation();
  const { disputeId } = useParams<{ disputeId: string }>();
  const navigate = useNavigate();
  const { user } = useAuth();
  const { showToast } = useToast();

  const { data: dispute, isLoading, error } = useDispute(disputeId ?? '');
  const { data: timelineData, isLoading: timelineLoading } = useDisputeTimeline(disputeId ?? '');
  const { data: evidenceData, isLoading: evidenceLoading } = useDisputeEvidence(disputeId ?? '');
  const updateStatus = useUpdateDisputeStatus(user?.organizationId ?? '');

  useEffect(() => {
    if (error) {
      showToast({
        type: 'error',
        title: t('disputes.failedToLoadDetail'),
        message: error instanceof Error ? error.message : t('auth.unexpectedError'),
      });
    }
  }, [error, showToast, t]);

  if (!disputeId) {
    return (
      <div className="error-page">
        <h1>{t('errors.disputeNotFound')}</h1>
        <p>{t('errors.disputeNotFoundDesc')}</p>
        <Link to="/disputes">{t('common.backToDisputes')}</Link>
      </div>
    );
  }

  if (!dispute && !isLoading && error) {
    return (
      <div className="error-page">
        <h1>{t('errors.errorLoadingDispute')}</h1>
        <p>{t('errors.disputeLoadError')}</p>
        <Link to="/disputes" className="px-4 py-2 border border-gray-300 rounded-lg">
          {t('common.backToDisputes')}
        </Link>
      </div>
    );
  }

  // Map API DisputeWithDetails → UI DisputeDetail
  const uiDispute: UiDisputeDetail | undefined = dispute
    ? {
        id: dispute.id,
        organizationId: dispute.organizationId,
        unitId: dispute.unitId,
        referenceNumber: `DSP-${dispute.id.toUpperCase()}`,
        category: mapTypeToCategory(dispute.type),
        title: dispute.subject,
        description: dispute.description,
        status: mapApiStatusToUiStatus(dispute.status),
        // priority is UI-only; API does not expose it yet
        priority: 'medium' as DisputePriority,
        filedBy: dispute.filedBy,
        filedByName: dispute.filerDetails?.name ?? dispute.filedBy,
        assignedTo: dispute.assignedMediatorId,
        assignedToName: dispute.assignedMediator,
        createdAt: dispute.createdAt,
        updatedAt: dispute.updatedAt,
      }
    : undefined;

  // Map API TimelineEvent[] → UI TimelineEntry[]
  const timeline = (timelineData ?? []).map((ev) => ({
    id: ev.id,
    actorId: ev.actorId,
    actorName: ev.actorName,
    activityType: mapTimelineEventType(ev.eventType),
    description: ev.description,
    metadata: ev.metadata,
    createdAt: ev.createdAt,
  }));

  // Map API DisputeEvidence[] → UI DisputeEvidence[]
  const evidence = (evidenceData ?? []).map((ev) => ({
    id: ev.id,
    uploadedBy: ev.uploadedBy,
    uploaderName: ev.uploadedBy,
    filename: ev.fileName,
    originalFilename: ev.fileName,
    contentType: ev.fileType,
    sizeBytes: ev.fileSize,
    storageUrl: ev.fileUrl,
    description: ev.description,
    createdAt: ev.createdAt,
  }));

  const isManager =
    user?.role === 'manager' ||
    user?.role === 'org_admin' ||
    user?.role === 'super_admin' ||
    user?.role === 'technical_manager' ||
    user?.role === 'property_manager';

  // #516 — these dispute actions are not yet wired to a backend mutation.
  // Render-time `() => {}` no-ops silently swallowed clicks (user clicks,
  // nothing happens). Until the API surface lands, surface a toast so the
  // user knows the action exists but is pending implementation.
  const notImplemented = useCallback(
    (label: string) => () => {
      showToast({
        type: 'info',
        title: t('common.notImplemented', { defaultValue: 'Not yet available' }),
        message: t('disputes.actionPendingImpl', {
          defaultValue: '{{action}} will be available in a future release.',
          action: label,
        }),
      });
    },
    [showToast, t]
  );

  const handleUpdateStatus = async (status: UiDisputeStatus, reason?: string) => {
    const apiStatus = mapUiStatusToApiStatus(status);
    if (!apiStatus || !disputeId) return;
    try {
      await updateStatus.mutateAsync({ disputeId, data: { status: apiStatus, reason } });
      showToast({
        type: 'success',
        title: t('disputes.statusUpdated', 'Status updated'),
        message: '',
      });
    } catch (err) {
      showToast({
        type: 'error',
        title: t('disputes.statusUpdateFailed', 'Failed to update status'),
        message: err instanceof Error ? err.message : t('auth.unexpectedError'),
      });
    }
  };

  return (
    <DisputeDetailPage
      dispute={uiDispute!}
      parties={[]}
      evidence={evidence}
      timeline={timeline}
      resolutions={[]}
      actionItems={[]}
      isManager={isManager}
      isLoading={isLoading || timelineLoading || evidenceLoading}
      currentUserId={user?.id}
      onBack={() => navigate('/disputes')}
      onUpdateStatus={handleUpdateStatus}
      onAddEvidence={notImplemented('Add evidence')}
      onDeleteEvidence={notImplemented('Delete evidence')}
      onProposeResolution={notImplemented('Propose resolution')}
      onVoteResolution={notImplemented('Vote on resolution')}
      onAcceptResolution={notImplemented('Accept resolution')}
      onImplementResolution={notImplemented('Implement resolution')}
      onCompleteResolutionTerm={notImplemented('Complete resolution term')}
      onCreateAction={notImplemented('Create action')}
      onCompleteAction={notImplemented('Complete action')}
      onSendReminder={notImplemented('Send reminder')}
      onEscalate={notImplemented('Escalate')}
      onNavigateToMediation={() => navigate(`/disputes/${disputeId}/mediation`)}
    />
  );
}

/**
 * Route wrapper for dispute mediation workspace (Epic 80, Story 80-3).
 *
 * Route: /disputes/:disputeId/mediation
 *
 * Replaces the previous MediationPage stub with the full MediationWorkspacePage:
 *   - Dispute timeline wired to useDisputeTimeline (real API)
 *   - Manager/tenant chat thread via useMediationNotes + useAddMediationNote
 *   - Resolution form using useResolveDispute
 *   - Escalate dialog using useEscalateDispute
 *   - Assign mediator dialog using useAssignMediator
 *
 * The legacy MediationPage (sessions/submissions) remains in the codebase for
 * the session-scheduling sub-feature pending a future backend endpoint.
 */
function DisputeMediationRoute() {
  const { t } = useTranslation();
  const { disputeId } = useParams<{ disputeId: string }>();
  const navigate = useNavigate();
  const { user } = useAuth();
  const { showToast } = useToast();

  if (!disputeId) {
    return (
      <div className="error-page">
        <h1>{t('errors.disputeNotFound')}</h1>
        <p>{t('errors.disputeNotFoundDesc')}</p>
        <Link to="/disputes">{t('common.backToDisputes')}</Link>
      </div>
    );
  }

  if (!user?.organizationId) {
    return <AuthRequiredGate />;
  }

  const organizationId = user.organizationId;

  const isManager =
    user?.role === 'manager' ||
    user?.role === 'org_admin' ||
    user?.role === 'super_admin' ||
    user?.role === 'technical_manager' ||
    user?.role === 'property_manager';

  return (
    <MediationWorkspacePage
      disputeId={disputeId}
      currentUserId={user?.id}
      currentUserName={
        user ? [user.firstName, user.lastName].filter(Boolean).join(' ') || user.email : undefined
      }
      organizationId={organizationId}
      isManager={isManager}
      onBack={() => navigate(`/disputes/${disputeId}`)}
      onToastSuccess={(title, message) =>
        showToast({ type: 'success', title, message: message ?? '' })
      }
      onToastError={(title, message) =>
        showToast({ type: 'error', title, message: message ?? t('auth.unexpectedError') })
      }
    />
  );
}

/**
 * Route wrapper for outages list page (UC-12).
 * Manages filter state and navigation callbacks.
 */
function OutagesPageRoute() {
  const navigate = useNavigate();
  const { user } = useAuth();
  const [queryParams, setQueryParams] = useState<OutageListQuery>({ limit: 10, offset: 0 });

  const { data, isLoading } = useOutages(queryParams);

  if (!user?.organizationId) {
    return <AuthRequiredGate />;
  }

  const outages: ApiOutageSummary[] = data?.outages ?? [];
  const total = data?.total ?? 0;

  const handleNavigateToCreate = () => navigate('/outages/new');
  const handleNavigateToView = (id: string) => navigate(`/outages/${id}`);
  const handleNavigateToEdit = (id: string) => navigate(`/outages/${id}/edit`);
  const handleFilterChange = (params: ListOutagesParams) => {
    setQueryParams({
      status: params.status as ApiOutageStatus | undefined,
      commodity: params.commodity as ApiOutageCommodity | undefined,
      severity: params.severity as ApiOutageSeverity | undefined,
      limit: params.pageSize,
      offset: (params.page - 1) * params.pageSize,
    });
  };

  return (
    <OutagesPage
      outages={outages}
      total={total}
      isLoading={isLoading}
      onNavigateToCreate={handleNavigateToCreate}
      onNavigateToView={handleNavigateToView}
      onNavigateToEdit={handleNavigateToEdit}
      onFilterChange={handleFilterChange}
    />
  );
}

/**
 * Route wrapper for create outage page (UC-12).
 */
function CreateOutagePageRoute() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { user } = useAuth();
  const { showToast } = useToast();
  const createOutage = useCreateOutage();

  // Fetch buildings from API
  const { data: buildingsData, isLoading: isLoadingBuildings } = useBuildings();

  if (!user?.organizationId) {
    return <AuthRequiredGate />;
  }

  // Transform API buildings to UI format
  const buildings = (buildingsData?.items ?? []).map(transformBuildingForUI);

  const handleSubmit = async (data: {
    title: string;
    description: string;
    commodity: ApiOutageCommodity;
    severity: ApiOutageSeverity;
    buildingIds: string[];
    scheduledStart: string;
    scheduledEnd: string;
  }) => {
    try {
      await createOutage.mutateAsync({
        title: data.title,
        description: data.description,
        commodity: data.commodity,
        severity: data.severity,
        buildingIds: data.buildingIds,
        scheduledStart: data.scheduledStart,
        scheduledEnd: data.scheduledEnd || undefined,
      });
      showToast({
        type: 'success',
        title: t('outages.createdSuccessfully'),
        message: t('outages.outageCreatedMsg'),
      });
      navigate('/outages');
    } catch (error) {
      showToast({
        type: 'error',
        title: t('outages.failedToCreate'),
        message: error instanceof Error ? error.message : t('auth.unexpectedError'),
      });
    }
  };

  const handleCancel = () => navigate('/outages');

  return (
    <CreateOutagePage
      buildings={buildings}
      isLoading={createOutage.isPending || isLoadingBuildings}
      onSubmit={handleSubmit}
      onCancel={handleCancel}
    />
  );
}

/**
 * Route wrapper for view outage page (UC-12).
 */
function ViewOutagePageRoute() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { outageId } = useParams<{ outageId: string }>();
  const { showToast } = useToast();

  const { data: outageData, isLoading } = useOutage(outageId ?? '');
  const startOutage = useStartOutage();
  const resolveOutage = useResolveOutage();
  const cancelOutage = useCancelOutage();

  if (!outageId) {
    return (
      <div className="error-page">
        <h1>{t('errors.outageNotFound')}</h1>
        <p>{t('errors.outageNotFoundDesc')}</p>
        <Link to="/outages">{t('common.backToOutages')}</Link>
      </div>
    );
  }

  const handleEdit = () => navigate(`/outages/${outageId}/edit`);

  const handleStart = async () => {
    try {
      await startOutage.mutateAsync({ id: outageId });
      showToast({ type: 'success', title: t('outages.started'), message: '' });
    } catch (error) {
      showToast({
        type: 'error',
        title: t('outages.failedToStart'),
        message: error instanceof Error ? error.message : t('auth.unexpectedError'),
      });
    }
  };

  const handleResolve = async (notes: string) => {
    try {
      await resolveOutage.mutateAsync({ id: outageId, data: { resolutionNotes: notes } });
      showToast({ type: 'success', title: t('outages.resolved'), message: '' });
    } catch (error) {
      showToast({
        type: 'error',
        title: t('outages.failedToResolve'),
        message: error instanceof Error ? error.message : t('auth.unexpectedError'),
      });
    }
  };

  const handleCancel = async (reason: string) => {
    try {
      await cancelOutage.mutateAsync({ id: outageId, data: { reason } });
      showToast({ type: 'success', title: t('outages.cancelled'), message: '' });
    } catch (error) {
      showToast({
        type: 'error',
        title: t('outages.failedToCancel'),
        message: error instanceof Error ? error.message : t('auth.unexpectedError'),
      });
    }
  };

  const handleBack = () => navigate('/outages');

  if (isLoading || !outageData) {
    return (
      <div className="flex items-center justify-center py-12">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600" />
      </div>
    );
  }

  // Map API outage to UI OutageDetail format
  const outage: OutageDetail = {
    id: outageData.id,
    organizationId: outageData.organizationId,
    createdBy: outageData.createdBy,
    title: outageData.title,
    description: outageData.description ?? '',
    commodity: outageData.commodity,
    severity: outageData.severity,
    status: outageData.status,
    buildingIds: outageData.buildingIds,
    scheduledStart: outageData.scheduledStart,
    scheduledEnd: outageData.scheduledEnd,
    actualStart: outageData.actualStart,
    actualEnd: outageData.actualEnd,
    resolutionNotes: outageData.resolutionNotes,
    cancelReason: outageData.cancelReason,
    createdAt: outageData.createdAt,
    updatedAt: outageData.updatedAt,
    createdByName: outageData.creatorName ?? outageData.createdBy ?? 'Unknown',
    buildingNames: outageData.buildings?.map((b) => b.name) ?? [],
  };

  return (
    <ViewOutagePage
      outage={outage}
      isLoading={isLoading}
      onEdit={handleEdit}
      onStart={handleStart}
      onResolve={handleResolve}
      onCancel={handleCancel}
      onBack={handleBack}
    />
  );
}

/**
 * Route wrapper for edit outage page (UC-12).
 */
function EditOutagePageRoute() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { outageId } = useParams<{ outageId: string }>();
  const { showToast } = useToast();

  const { data: outageData, isLoading: isLoadingOutage } = useOutage(outageId ?? '');
  const updateOutage = useUpdateOutage();

  // Fetch buildings from API
  const { data: buildingsData, isLoading: isLoadingBuildings } = useBuildings();

  if (!outageId) {
    return (
      <div className="error-page">
        <h1>{t('errors.outageNotFound')}</h1>
        <p>{t('errors.outageNotFoundDesc')}</p>
        <Link to="/outages">{t('common.backToOutages')}</Link>
      </div>
    );
  }

  // Transform API buildings to UI format
  const buildings = (buildingsData?.items ?? []).map(transformBuildingForUI);

  const handleSubmit = async (data: {
    title: string;
    description: string;
    commodity: ApiOutageCommodity;
    severity: ApiOutageSeverity;
    buildingIds: string[];
    scheduledStart: string;
    scheduledEnd: string;
  }) => {
    try {
      await updateOutage.mutateAsync({
        id: outageId,
        data: {
          title: data.title,
          description: data.description,
          commodity: data.commodity,
          severity: data.severity,
          buildingIds: data.buildingIds,
          scheduledStart: data.scheduledStart,
          scheduledEnd: data.scheduledEnd || undefined,
        },
      });
      showToast({
        type: 'success',
        title: t('outages.updatedSuccessfully'),
        message: t('outages.outageUpdatedMsg'),
      });
      navigate(`/outages/${outageId}`);
    } catch (error) {
      showToast({
        type: 'error',
        title: t('outages.failedToUpdate'),
        message: error instanceof Error ? error.message : t('auth.unexpectedError'),
      });
    }
  };

  const handleCancel = () => navigate(`/outages/${outageId}`);

  if (isLoadingOutage || isLoadingBuildings || !outageData) {
    return (
      <div className="flex items-center justify-center py-12">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600" />
      </div>
    );
  }

  // Map API outage to UI OutageDetail format
  const outage: OutageDetail = {
    id: outageData.id,
    organizationId: outageData.organizationId,
    createdBy: outageData.createdBy,
    title: outageData.title,
    description: outageData.description ?? '',
    commodity: outageData.commodity,
    severity: outageData.severity,
    status: outageData.status,
    buildingIds: outageData.buildingIds,
    scheduledStart: outageData.scheduledStart,
    scheduledEnd: outageData.scheduledEnd,
    actualStart: outageData.actualStart,
    actualEnd: outageData.actualEnd,
    resolutionNotes: outageData.resolutionNotes,
    cancelReason: outageData.cancelReason,
    createdAt: outageData.createdAt,
    updatedAt: outageData.updatedAt,
    createdByName: outageData.creatorName ?? outageData.createdBy ?? 'Unknown',
    buildingNames: outageData.buildings?.map((b) => b.name) ?? [],
  };

  return (
    <EditOutagePage
      outage={outage}
      buildings={buildings}
      isLoading={updateOutage.isPending}
      onSubmit={handleSubmit}
      onCancel={handleCancel}
    />
  );
}

// ============================================
// Announcements Route Wrappers (UC-06)
// ============================================

/**
 * Route wrapper for announcements list page (UC-06, gap-79-1).
 *
 * Wired to @ppt/api-client standalone hooks:
 *   useAnnouncements — TanStack Query list with filter params
 *   useDeleteAnnouncement / usePublishAnnouncement / useArchiveAnnouncement / usePinAnnouncement
 *
 * AnnouncementSummary from @ppt/api-client matches the page's prop type
 * directly — no type mapping required.
 */
function AnnouncementsPageRoute() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { showToast } = useToast();
  const [listParams, setListParams] = useState<import('@ppt/api-client').ListAnnouncementsParams>({
    page: 1,
    pageSize: 10,
  });

  const { data, isLoading, error, refetch } = useAnnouncements(listParams);
  // Story 6.4 — separate query for the sticky pinned band; immune to list filters.
  // Routes through the shared hook (#486 / #516) so the URL + staleTime + pageSize
  // can't drift between this callsite and other consumers (mobile).
  const { data: pinnedData } = usePinnedAnnouncements();
  const deleteAnnouncement = useDeleteAnnouncement();
  const publishAnnouncement = usePublishAnnouncement();
  const archiveAnnouncement = useArchiveAnnouncement();
  const pinAnnouncement = usePinAnnouncement();

  useEffect(() => {
    if (error) {
      showToast({
        type: 'error',
        title: t('announcements.failedToLoad', { defaultValue: 'Failed to load announcements' }),
        message: error instanceof Error ? error.message : t('auth.unexpectedError'),
      });
    }
  }, [error, showToast, t]);

  const announcements = data?.items ?? [];
  const total = data?.total ?? 0;
  const pinnedAnnouncements = pinnedData?.items ?? [];

  const handleDelete = async (id: string) => {
    try {
      await deleteAnnouncement.mutateAsync(id);
      showToast({
        type: 'success',
        title: t('announcements.deleted', { defaultValue: 'Deleted' }),
        message: '',
      });
    } catch (err) {
      showToast({
        type: 'error',
        title: t('announcements.deleteFailed', { defaultValue: 'Delete failed' }),
        message: err instanceof Error ? err.message : '',
      });
    }
  };

  const handlePublish = async (id: string) => {
    try {
      await publishAnnouncement.mutateAsync(id);
      showToast({
        type: 'success',
        title: t('announcements.published', { defaultValue: 'Published' }),
        message: '',
      });
    } catch (err) {
      showToast({
        type: 'error',
        title: t('announcements.publishFailed', { defaultValue: 'Publish failed' }),
        message: err instanceof Error ? err.message : '',
      });
    }
  };

  const handleArchive = async (id: string) => {
    try {
      await archiveAnnouncement.mutateAsync(id);
      showToast({
        type: 'info',
        title: t('announcements.archived', { defaultValue: 'Archived' }),
        message: '',
      });
    } catch (err) {
      showToast({
        type: 'error',
        title: t('announcements.archiveFailed', { defaultValue: 'Archive failed' }),
        message: err instanceof Error ? err.message : '',
      });
    }
  };

  const handleAnnouncementsFilterChange = useCallback(
    (params: Partial<import('@ppt/api-client').ListAnnouncementsParams>) =>
      setListParams((prev) => ({ ...prev, ...params })),
    []
  );

  const handlePin = async (id: string, pinned: boolean) => {
    try {
      await pinAnnouncement.mutateAsync({ id, pinned });
      showToast({
        type: 'info',
        title: pinned
          ? t('announcements.pinned', { defaultValue: 'Pinned' })
          : t('announcements.unpinned', { defaultValue: 'Unpinned' }),
        message: '',
      });
    } catch (err) {
      showToast({
        type: 'error',
        title: t('announcements.pinFailed', { defaultValue: 'Pin action failed' }),
        message: err instanceof Error ? err.message : '',
      });
    }
  };

  return (
    <AnnouncementsPage
      announcements={announcements}
      total={total}
      isLoading={isLoading}
      isError={!!error}
      onRetry={() => {
        void refetch();
      }}
      pinnedAnnouncements={pinnedAnnouncements}
      onNavigateToCreate={() => navigate('/announcements/new')}
      onNavigateToView={(id) => navigate(`/announcements/${id}`)}
      onNavigateToEdit={(id) => navigate(`/announcements/${id}/edit`)}
      onDelete={handleDelete}
      onPublish={handlePublish}
      onArchive={handleArchive}
      onPin={handlePin}
      onFilterChange={handleAnnouncementsFilterChange}
    />
  );
}

function CreateAnnouncementPageRoute() {
  const navigate = useNavigate();
  const { showToast } = useToast();

  return (
    <CreateAnnouncementPage
      buildings={[]}
      units={[]}
      roles={[]}
      onSubmit={() => {
        showToast({ type: 'success', title: 'Created', message: 'Announcement created' });
        navigate('/announcements');
      }}
      onCancel={() => navigate('/announcements')}
    />
  );
}

/**
 * Inner component for announcement detail — all hooks called unconditionally.
 *
 * Wired to @ppt/api-client standalone hooks:
 *   useAnnouncement                    — TanStack Query detail (announcement + attachments)
 *   useMarkReadAnnouncement            — POST /api/v1/announcements/:id/read
 *   useAcknowledgeAnnouncement         — POST /api/v1/announcements/:id/acknowledge
 *   useAnnouncementAcknowledgmentStats — GET  /api/v1/announcements/:id/acknowledgments
 *   useAnnouncementComments            — GET  /api/v1/announcements/:id/comments (Story 6.3)
 *   useCreateAnnouncementComment       — POST /api/v1/announcements/:id/comments (Story 6.3)
 *   useDeleteAnnouncementComment       — DELETE /api/v1/announcements/:id/comments/:cid (Story 6.3)
 *   usePublishAnnouncement / useArchiveAnnouncement / usePinAnnouncement / useDeleteAnnouncement
 */
function ViewAnnouncementPageInner({ announcementId }: { announcementId: string }) {
  const navigate = useNavigate();
  const { showToast } = useToast();
  const { t } = useTranslation();
  const { user } = useAuth();

  // Guard ref: fire mark-read exactly once per announcement view, even in StrictMode.
  const autoMarkReadFired = useRef(false);

  const { data, isLoading, error } = useAnnouncement(announcementId);
  const markRead = useMarkReadAnnouncement();
  const acknowledge = useAcknowledgeAnnouncement();
  const publishAnnouncement = usePublishAnnouncement();
  const archiveAnnouncement = useArchiveAnnouncement();
  const pinAnnouncement = usePinAnnouncement();
  const deleteAnnouncement = useDeleteAnnouncement();

  // Fetch ack stats only when the announcement requires acknowledgment (manager view)
  const { data: ackStatsData } = useAnnouncementAcknowledgmentStats(
    announcementId,
    !!data?.announcement?.acknowledgmentRequired
  );

  // Story 6.3: Comments — only fetch when commentsEnabled
  const commentsEnabled = !!data?.announcement?.commentsEnabled;
  const { data: commentsData, isLoading: commentsLoading } = useAnnouncementComments(
    announcementId,
    undefined,
    commentsEnabled
  );
  const createComment = useCreateAnnouncementComment();
  const deleteComment = useDeleteAnnouncementComment();

  useEffect(() => {
    if (error) {
      showToast({
        type: 'error',
        title: t('announcements.failedToLoad', { defaultValue: 'Failed to load announcement' }),
        message: error instanceof Error ? error.message : t('auth.unexpectedError'),
      });
    }
  }, [error, showToast, t]);

  // Story 6.2: auto-fire read-receipt when announcement loads.
  // Fire-and-forget — the user shouldn't have to click "Mark as Read".
  // The mutation is idempotent (upsert on server), so double-firing is safe.
  // Retry is handled by TanStack Mutation default (3 attempts, exponential back-off).
  // markRead.mutate is intentionally excluded from deps — including a new mutate
  // instance on each render would cause an infinite loop; the ref guard ensures
  // the call fires exactly once per viewed announcement.
  // biome-ignore lint/correctness/useExhaustiveDependencies: markRead.mutate excluded intentionally (see comment)
  useEffect(() => {
    if (data?.announcement && !autoMarkReadFired.current) {
      autoMarkReadFired.current = true;
      markRead.mutate(announcementId);
    }
  }, [data?.announcement, announcementId]);

  const announcement = data?.announcement;
  const attachments = data?.attachments ?? [];

  const handleMarkRead = async () => {
    try {
      await markRead.mutateAsync(announcementId);
      showToast({
        type: 'success',
        title: t('announcements.markedRead', { defaultValue: 'Marked as read' }),
        message: '',
      });
    } catch (err) {
      showToast({
        type: 'error',
        title: t('announcements.markReadFailed', { defaultValue: 'Failed to mark as read' }),
        message: err instanceof Error ? err.message : '',
      });
    }
  };

  const handleAcknowledge = async () => {
    try {
      await acknowledge.mutateAsync(announcementId);
      showToast({
        type: 'success',
        title: t('announcements.acknowledged', { defaultValue: 'Acknowledged' }),
        message: '',
      });
    } catch (err) {
      showToast({
        type: 'error',
        title: t('announcements.acknowledgeFailed', { defaultValue: 'Failed to acknowledge' }),
        message: err instanceof Error ? err.message : '',
      });
    }
  };

  const handlePublish = async () => {
    try {
      await publishAnnouncement.mutateAsync(announcementId);
      showToast({
        type: 'success',
        title: t('announcements.published', { defaultValue: 'Published' }),
        message: '',
      });
    } catch (err) {
      showToast({
        type: 'error',
        title: t('announcements.publishFailed', { defaultValue: 'Publish failed' }),
        message: err instanceof Error ? err.message : '',
      });
    }
  };

  const handleArchive = async () => {
    try {
      await archiveAnnouncement.mutateAsync(announcementId);
      showToast({
        type: 'info',
        title: t('announcements.archived', { defaultValue: 'Archived' }),
        message: '',
      });
    } catch (err) {
      showToast({
        type: 'error',
        title: t('announcements.archiveFailed', { defaultValue: 'Archive failed' }),
        message: err instanceof Error ? err.message : '',
      });
    }
  };

  const handlePin = async (pinned: boolean) => {
    try {
      await pinAnnouncement.mutateAsync({ id: announcementId, pinned });
      showToast({
        type: 'info',
        title: pinned
          ? t('announcements.pinned', { defaultValue: 'Pinned' })
          : t('announcements.unpinned', { defaultValue: 'Unpinned' }),
        message: '',
      });
    } catch (err) {
      showToast({
        type: 'error',
        title: t('announcements.pinFailed', { defaultValue: 'Pin action failed' }),
        message: err instanceof Error ? err.message : '',
      });
    }
  };

  const handleDelete = async () => {
    try {
      await deleteAnnouncement.mutateAsync(announcementId);
      showToast({
        type: 'success',
        title: t('announcements.deleted', { defaultValue: 'Deleted' }),
        message: '',
      });
      navigate('/announcements');
    } catch (err) {
      showToast({
        type: 'error',
        title: t('announcements.deleteFailed', { defaultValue: 'Delete failed' }),
        message: err instanceof Error ? err.message : '',
      });
    }
  };

  // Story 6.3: Comment handlers
  const handleAddComment = async (content: string, parentId?: string) => {
    try {
      await createComment.mutateAsync({ announcementId, content, parentId });
      showToast({
        type: 'success',
        title: t('announcements.commentPosted', { defaultValue: 'Comment posted' }),
        message: '',
      });
    } catch (err) {
      showToast({
        type: 'error',
        title: t('announcements.commentFailed', { defaultValue: 'Failed to post comment' }),
        message: err instanceof Error ? err.message : '',
      });
    }
  };

  const handleDeleteComment = async (commentId: string, reason?: string) => {
    try {
      await deleteComment.mutateAsync({ announcementId, commentId, reason });
      showToast({
        type: 'info',
        title: t('announcements.commentDeleted', { defaultValue: 'Comment deleted' }),
        message: '',
      });
    } catch (err) {
      showToast({
        type: 'error',
        title: t('announcements.commentDeleteFailed', { defaultValue: 'Failed to delete comment' }),
        message: err instanceof Error ? err.message : '',
      });
    }
  };

  const EMPTY_ANNOUNCEMENT: import('@ppt/api-client').AnnouncementWithDetails = {
    id: announcementId,
    organizationId: '',
    authorId: '',
    authorName: '',
    title: '',
    content: '',
    status: 'draft',
    targetType: 'all',
    targetIds: [],
    pinned: false,
    acknowledgmentRequired: false,
    commentsEnabled: false,
    readCount: 0,
    acknowledgedCount: 0,
    commentCount: 0,
    attachmentCount: 0,
    createdAt: '',
    updatedAt: '',
  };

  // Build commentsProps when comments are enabled (Story 6.3)
  const isManager =
    user?.role === 'manager' ||
    user?.role === 'org_admin' ||
    user?.role === 'super_admin' ||
    user?.role === 'technical_manager' ||
    user?.role === 'property_manager';
  const commentsProps = commentsEnabled
    ? {
        comments: commentsData?.comments ?? [],
        total: commentsData?.total ?? 0,
        isLoading: commentsLoading,
        currentUserId: user?.id,
        isManager,
        onAddComment: handleAddComment,
        onDeleteComment: handleDeleteComment,
      }
    : undefined;

  if (isLoading || !announcement) {
    return (
      <ViewAnnouncementPage
        announcement={EMPTY_ANNOUNCEMENT}
        attachments={[]}
        isLoading
        onEdit={() => navigate(`/announcements/${announcementId}/edit`)}
        onPublish={handlePublish}
        onArchive={handleArchive}
        onPin={handlePin}
        onDelete={handleDelete}
        onBack={() => navigate('/announcements')}
      />
    );
  }

  return (
    <ViewAnnouncementPage
      announcement={announcement}
      attachments={attachments}
      onEdit={() => navigate(`/announcements/${announcementId}/edit`)}
      onPublish={handlePublish}
      onArchive={handleArchive}
      onPin={handlePin}
      onDelete={handleDelete}
      onBack={() => navigate('/announcements')}
      onMarkRead={handleMarkRead}
      onAcknowledge={announcement.acknowledgmentRequired ? handleAcknowledge : undefined}
      acknowledgmentStats={ackStatsData?.stats}
      commentsProps={commentsProps}
    />
  );
}

/** Route wrapper — guards for missing param before mounting inner component */
function ViewAnnouncementPageRoute() {
  const { announcementId } = useParams<{ announcementId: string }>();
  if (!announcementId) {
    return <div>Announcement not found</div>;
  }
  return <ViewAnnouncementPageInner announcementId={announcementId} />;
}

function EditAnnouncementPageRoute() {
  const { announcementId } = useParams<{ announcementId: string }>();
  const navigate = useNavigate();
  const { showToast } = useToast();

  if (!announcementId) {
    return <div>Announcement not found</div>;
  }

  // Mock announcement data - would use useAnnouncement hook
  const mockAnnouncement: import('@ppt/api-client').Announcement = {
    id: announcementId,
    organizationId: 'org-1',
    authorId: 'user-1',
    title: 'Sample Announcement',
    content: 'This is a sample announcement content.',
    status: 'draft',
    targetType: 'all',
    targetIds: [],
    pinned: false,
    acknowledgmentRequired: false,
    commentsEnabled: true,
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
  };

  return (
    <EditAnnouncementPage
      announcement={mockAnnouncement}
      buildings={[]}
      units={[]}
      roles={[]}
      onSubmit={() => {
        showToast({ type: 'success', title: 'Updated', message: 'Announcement updated' });
        navigate(`/announcements/${announcementId}`);
      }}
      onCancel={() => navigate(`/announcements/${announcementId}`)}
    />
  );
}

// ============================================
// Messaging Route Wrappers (UC-07)
// ============================================

function MessagesPageRoute() {
  const navigate = useNavigate();
  const { t } = useTranslation();
  const { showToast } = useToast();
  const [msgQueryParams, setMsgQueryParams] = useState<{
    limit: number;
    offset: number;
    search?: string;
  }>({
    limit: 20,
    offset: 0,
  });

  const { threads, total, isLoading: threadsLoading } = useThreads(msgQueryParams);
  const { data: unreadData } = useUnreadCount();
  const unreadCount = unreadData?.unreadCount ?? 0;

  return (
    <MessagesPage
      threads={threads}
      total={total}
      unreadCount={unreadCount}
      isLoading={threadsLoading}
      onNavigateToThread={(threadId) => navigate(`/messages/${threadId}`)}
      onNavigateToCreate={() => navigate('/messages/new')}
      onFilterChange={(params) => {
        setMsgQueryParams({
          limit: params.pageSize ?? 20,
          offset: ((params.page ?? 1) - 1) * (params.pageSize ?? 20),
          search: params.search,
        });
      }}
      onDeleteThreads={() => {
        // Thread deletion is not yet supported by the API
        showToast({
          type: 'error',
          title: t('common.error'),
          message: t('messaging.deleteNotSupported', 'Deleting threads is not yet supported'),
        });
      }}
    />
  );
}

function NewMessagePageRoute() {
  const navigate = useNavigate();
  const { showToast } = useToast();
  const { t } = useTranslation();
  const [searchParams] = useSearchParams();

  // Fetch potential recipients from the user's building neighbors. Without a
  // building id the recipients query stays disabled and the list is always
  // empty — making the page unusable. Mirror the NeighborsPage convention and
  // use the first building the user has access to (building-selector deferred).
  const { data: buildingsData, isLoading: isLoadingBuildings } = useBuildings();
  const buildingId = buildingsData?.items?.[0]?.id;
  const { recipients, isLoading: isLoadingRecipients } = useMessageRecipients(buildingId);
  const startThread = useStartThread();

  const handleNewMsgSubmit = async (data: import('./features/messaging').CreateThreadRequest) => {
    if (!data.recipientIds[0]) return;
    try {
      const result = await startThread.mutateAsync(toStartThreadRequest(data));
      showToast({
        type: 'success',
        title: t('common.success'),
        message: t('messaging.messageSent', 'Message sent'),
      });
      navigate(`/messages/${result.thread.id}`);
    } catch {
      showToast({
        type: 'error',
        title: t('common.error'),
        message: t('messaging.sendFailed', 'Failed to send message'),
      });
    }
  };

  return (
    <NewMessagePage
      recipients={recipients}
      initialRecipientIds={
        searchParams.get('recipientId') ? [searchParams.get('recipientId') as string] : undefined
      }
      isLoadingRecipients={isLoadingRecipients || isLoadingBuildings}
      isSubmitting={startThread.isPending}
      onSubmit={handleNewMsgSubmit}
      onCancel={() => navigate('/messages')}
    />
  );
}

function ThreadDetailPageRoute() {
  const { threadId } = useParams<{ threadId: string }>();
  const navigate = useNavigate();
  const { user } = useAuth();
  const { showToast } = useToast();
  const { t } = useTranslation();

  const { thread, isLoading: threadLoading } = useThread(threadId ?? '', !!threadId);
  const sendMessage = useSendMessage();
  const markRead = useMarkThreadRead();
  const deleteMessage = useDeleteMessage();

  if (!threadId) {
    return <div>Thread not found</div>;
  }

  const handleThreadSendMessage = async (
    data: import('./features/messaging').SendMessageRequest
  ) => {
    try {
      await sendMessage.mutateAsync({ threadId, data: toApiSendMessageRequest(data) });
    } catch {
      showToast({
        type: 'error',
        title: t('common.error'),
        message: t('messaging.sendFailed', 'Failed to send message'),
      });
    }
  };

  const handleThreadMarkAsRead = () => {
    markRead.mutate(threadId);
  };

  // Show loading skeleton until thread data arrives
  if (threadLoading || !thread) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600" />
      </div>
    );
  }

  return (
    <ThreadDetailPage
      thread={thread}
      currentUserId={user?.id ?? ''}
      isLoading={false}
      isSending={sendMessage.isPending}
      onSendMessage={handleThreadSendMessage}
      onDeleteMessage={(messageId) => deleteMessage.mutate({ threadId, messageId })}
      onMarkAsRead={handleThreadMarkAsRead}
      onBack={() => navigate('/messages')}
    />
  );
}

// ============================================
// Faults Route Wrappers (UC-03)
// ============================================

/**
 * Map API FaultStatus (snake_case values) → UI FaultStatus.
 *
 * API: 'reported' | 'triaged' | 'in_progress' | 'scheduled' | 'on_hold' | 'resolved' | 'closed' | 'reopened'
 * UI:  'new'      | 'triaged' | 'in_progress' | 'scheduled' | 'waiting_parts' | 'resolved' | 'closed' | 'reopened'
 */
function mapApiFaultStatusToUi(
  status: ApiFaultSummary['status']
): import('./features/faults/components/FaultCard').FaultStatus {
  const mapping: Record<
    ApiFaultSummary['status'],
    import('./features/faults/components/FaultCard').FaultStatus
  > = {
    new: 'new',
    triaged: 'triaged',
    in_progress: 'in_progress',
    scheduled: 'scheduled',
    waiting_parts: 'waiting_parts',
    resolved: 'resolved',
    closed: 'closed',
    reopened: 'reopened',
  };
  return mapping[status] ?? 'new';
}

/**
 * Map UI fault status (used by FaultCard) to the API status enum.
 * Module-level (#486) so the route wrapper doesn't recreate it per call.
 */
const UI_FAULT_STATUS_TO_API: Record<
  import('./features/faults/components/FaultCard').FaultStatus,
  ApiFaultSummary['status'] | undefined
> = {
  new: 'new',
  triaged: 'triaged',
  in_progress: 'in_progress',
  waiting_parts: 'waiting_parts',
  scheduled: 'scheduled',
  resolved: 'resolved',
  closed: 'closed',
  reopened: 'reopened',
};

/** Transform API FaultSummary (snake_case) → UI FaultSummary (camelCase) */
function transformApiFaultToUi(
  fault: ApiFaultSummary
): import('./features/faults/components/FaultCard').FaultSummary {
  return {
    id: fault.id,
    buildingId: fault.building_id,
    unitId: fault.unit_id,
    title: fault.title,
    category: fault.category,
    priority: fault.priority ?? 'medium',
    status: mapApiFaultStatusToUi(fault.status),
    createdAt: fault.created_at,
  };
}

/**
 * Map API FaultWithDetails (snake_case) → page-level FaultDetail (camelCase).
 *
 * The api-client `FaultWithDetails` mirrors the backend model: nullable columns
 * are typed `T | null`, while the page `FaultDetail` uses optional (`T | undefined`)
 * fields, so we strip `null` → `undefined` on the nullable fields.
 */
function mapApiFaultDetailToUi(
  fault: import('@ppt/api-client').FaultWithDetails
): import('./features/faults').FaultDetail {
  return {
    id: fault.id,
    organizationId: fault.organization_id,
    buildingId: fault.building_id,
    unitId: fault.unit_id ?? undefined,
    reporterId: fault.reporter_id,
    title: fault.title,
    description: fault.description,
    locationDescription: fault.location_description ?? undefined,
    category: fault.category,
    priority: fault.priority ?? 'medium',
    status: mapApiFaultStatusToUi(fault.status),
    aiCategory: fault.ai_category ?? undefined,
    aiPriority: fault.ai_priority ?? undefined,
    aiConfidence: fault.ai_confidence != null ? Number(fault.ai_confidence) : undefined,
    assignedTo: fault.assigned_to ?? undefined,
    assignedAt: fault.assigned_at ?? undefined,
    triagedBy: fault.triaged_by ?? undefined,
    triagedAt: fault.triaged_at ?? undefined,
    resolvedAt: fault.resolved_at ?? undefined,
    resolvedBy: fault.resolved_by ?? undefined,
    resolutionNotes: fault.resolution_notes ?? undefined,
    confirmedAt: fault.confirmed_at ?? undefined,
    confirmedBy: fault.confirmed_by ?? undefined,
    rating: fault.rating ?? undefined,
    feedback: fault.feedback ?? undefined,
    scheduledDate: fault.scheduled_date ?? undefined,
    estimatedCompletion: fault.estimated_completion ?? undefined,
    createdAt: fault.created_at,
    updatedAt: fault.updated_at,
    reporterName: fault.reporter_name,
    reporterEmail: fault.reporter_email,
    buildingName: fault.building_name,
    buildingAddress: fault.building_address,
    unitDesignation: fault.unit_designation ?? undefined,
    assignedToName: fault.assigned_to_name ?? undefined,
    attachmentCount: fault.attachment_count,
    commentCount: fault.comment_count,
  };
}

/**
 * Map an API fault `event_type` string to the page TimelineEntry `action`.
 * Unknown event types fall back to `status_changed`.
 */
function mapFaultEventTypeToAction(
  eventType: string
): import('./features/faults/components/FaultTimeline').TimelineAction {
  const mapping: Record<
    string,
    import('./features/faults/components/FaultTimeline').TimelineAction
  > = {
    created: 'created',
    reported: 'created',
    triaged: 'triaged',
    assigned: 'assigned',
    status_changed: 'status_changed',
    priority_changed: 'priority_changed',
    work_note: 'work_note',
    comment: 'comment',
    comment_added: 'comment',
    attachment_added: 'attachment_added',
    scheduled: 'scheduled',
    resolved: 'resolved',
    confirmed: 'confirmed',
    reopened: 'reopened',
    rated: 'rated',
  };
  return mapping[eventType] ?? 'status_changed';
}

/** Map API FaultTimelineEntry (snake_case) → page TimelineEntry. */
function mapApiFaultTimelineToUi(
  entry: import('@ppt/api-client').FaultTimelineEntry
): import('./features/faults/components/FaultTimeline').TimelineEntry {
  return {
    id: entry.id,
    faultId: entry.fault_id,
    userId: entry.user_id,
    action: mapFaultEventTypeToAction(entry.action),
    note: entry.note ?? undefined,
    oldValue: entry.old_value ?? undefined,
    newValue: entry.new_value ?? undefined,
    isInternal: entry.is_internal,
    createdAt: entry.created_at,
    userName: entry.user_name ?? '',
    userEmail: entry.user_email ?? '',
  };
}

/** Map API FaultAttachment (snake_case) → page FaultAttachment. */
function mapApiFaultAttachmentToUi(
  att: import('@ppt/api-client').FaultAttachment
): import('./features/faults').FaultAttachment {
  return {
    id: att.id,
    faultId: att.fault_id,
    filename: att.file_name,
    originalFilename: att.file_name,
    contentType: att.file_type,
    sizeBytes: att.file_size,
    storageUrl: att.file_url,
    description: undefined,
    createdAt: att.created_at,
  };
}

/**
 * Route wrapper for faults list page (UC-03, gap-79-1).
 *
 * Wired to useFaults from @ppt/api-client (TanStack Query).
 * API FaultSummary uses snake_case and a slightly different status enum;
 * transformApiFaultToUi() bridges the gap to the UI FaultCard type.
 */
function FaultsPageRoute() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { showToast } = useToast();
  const [faultQuery, setFaultQuery] = useState<FaultListQuery>({ page: 1, limit: 10 });

  const { data, isLoading, error, refetch } = useFaults(faultQuery);

  useEffect(() => {
    if (error) {
      showToast({
        type: 'error',
        title: t('faults.failedToLoad', { defaultValue: 'Failed to load faults' }),
        message: error instanceof Error ? error.message : t('auth.unexpectedError'),
      });
    }
  }, [error, showToast, t]);

  const faults = (data?.faults ?? []).map(transformApiFaultToUi);
  const total = data?.count ?? 0;

  // useCallback so child FaultsPage doesn't re-render on every parent render
  // (#486). Status mapping uses the module-level UI_FAULT_STATUS_TO_API.
  const handleFaultsFilterChange = useCallback(
    (params: {
      status?: import('./features/faults/components/FaultCard').FaultStatus;
      category?: ApiFaultSummary['category'];
      priority?: ApiFaultSummary['priority'];
      search?: string;
      page?: number;
      pageSize?: number;
    }) => {
      setFaultQuery({
        status: params.status ? UI_FAULT_STATUS_TO_API[params.status] : undefined,
        category: params.category,
        priority: params.priority,
        search: params.search,
        page: params.page,
        limit: params.pageSize,
      });
    },
    []
  );

  return (
    <FaultsPage
      faults={faults}
      total={total}
      isLoading={isLoading}
      isError={!!error}
      onRetry={() => {
        void refetch();
      }}
      onNavigateToCreate={() => navigate('/faults/new')}
      onNavigateToView={(id) => navigate(`/faults/${id}`)}
      onNavigateToEdit={(id) => navigate(`/faults/${id}/edit`)}
      onNavigateToTriage={(id) => navigate(`/faults/${id}`)}
      onFilterChange={handleFaultsFilterChange}
    />
  );
}

function CreateFaultPageRoute() {
  const navigate = useNavigate();
  const { showToast } = useToast();
  const { data: buildingsData } = useBuildings();
  const createFault = useCreateFault();

  const buildings = (buildingsData?.items ?? []).map((b) => ({ id: b.id, name: b.name }));

  return (
    <CreateFaultPage
      buildings={buildings}
      units={[]}
      isSubmitting={createFault.isPending}
      onSubmit={async (data) => {
        try {
          // Map FaultFormData (camelCase) -> CreateFaultRequest (snake_case).
          // NOTE: photos (File[]) are not sent here — base64/upload wiring via
          // useAddAttachment is a separate follow-up (see fault-new screen-map).
          const created = await createFault.mutateAsync({
            building_id: data.buildingId,
            unit_id: data.unitId,
            title: data.title,
            description: data.description,
            location_description: data.locationDescription,
            category: data.category,
            priority: data.priority,
          });
          showToast({ type: 'success', title: 'Created', message: 'Fault reported' });
          navigate(created?.id ? `/faults/${created.id}` : '/faults');
        } catch (err) {
          showToast({
            type: 'error',
            title: 'Failed to report fault',
            message: err instanceof Error ? err.message : 'Please try again.',
          });
        }
      }}
      onCancel={() => navigate('/faults')}
    />
  );
}

/**
 * Route wrapper for fault detail page (UC-03, #970.1).
 *
 * Wired to @ppt/api-client fault hooks (TanStack Query). `useFault` returns the
 * `FaultDetailResponse` ({ fault, timeline, attachments }); the snake_case API
 * shapes are mapped to the page's camelCase `FaultDetail` / `TimelineEntry` /
 * `FaultAttachment` via the module-level mappers near `transformApiFaultToUi`.
 */
function FaultDetailPageRoute() {
  const { faultId } = useParams<{ faultId: string }>();
  const navigate = useNavigate();
  const { showToast } = useToast();
  const { t } = useTranslation();
  const { user } = useAuth();

  const { data, isLoading, error } = useFault(faultId ?? '');
  const triage = useTriageFault(faultId ?? '');
  const resolve = useResolveFault(faultId ?? '');
  const confirm = useConfirmFault(faultId ?? '');
  const reopen = useReopenFault(faultId ?? '');
  const addComment = useAddComment(faultId ?? '');
  const addAttachment = useAddAttachment(faultId ?? '');
  const deleteAttachment = useDeleteAttachment(faultId ?? '');

  useEffect(() => {
    if (error) {
      showToast({
        type: 'error',
        title: t('faults.failedToLoad', { defaultValue: 'Failed to load fault' }),
        message: error instanceof Error ? error.message : t('auth.unexpectedError'),
      });
    }
  }, [error, showToast, t]);

  if (!faultId) {
    return <div>Fault not found</div>;
  }

  const isManager =
    user?.role === 'manager' ||
    user?.role === 'org_admin' ||
    user?.role === 'super_admin' ||
    user?.role === 'technical_manager' ||
    user?.role === 'property_manager';

  const fault = data?.fault ? mapApiFaultDetailToUi(data.fault) : undefined;
  const timeline = (data?.timeline ?? []).map(mapApiFaultTimelineToUi);
  const attachments = (data?.attachments ?? []).map(mapApiFaultAttachmentToUi);

  if (isLoading || !fault) {
    return (
      <div className="flex items-center justify-center py-12">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600" />
      </div>
    );
  }

  return (
    <FaultDetailPage
      fault={fault}
      timeline={timeline}
      attachments={attachments}
      isManager={isManager}
      isLoading={isLoading}
      onBack={() => navigate('/faults')}
      onEdit={() => navigate(`/faults/${faultId}/edit`)}
      onTriage={(triageData) =>
        triage.mutate({
          priority: triageData.priority,
          category: triageData.category,
          assigned_to: triageData.assignedTo,
        })
      }
      onResolve={(notes) => resolve.mutate({ resolution_notes: notes })}
      onConfirm={(rating) => confirm.mutate(rating)}
      onReopen={(reason) => reopen.mutate(reason)}
      onAddComment={(content, isInternal) =>
        addComment.mutate({ content, is_internal: isInternal })
      }
      onAddAttachment={(file) => addAttachment.mutate(file)}
      onDeleteAttachment={(id) => deleteAttachment.mutate(id)}
    />
  );
}

function EditFaultPageRoute() {
  const { faultId } = useParams<{ faultId: string }>();
  const navigate = useNavigate();
  const { showToast } = useToast();
  const { data, isLoading } = useFault(faultId ?? '');
  const { data: buildingsData } = useBuildings();
  const updateFault = useUpdateFault(faultId ?? '');

  if (!faultId) {
    return <div>Fault not found</div>;
  }
  if (isLoading || !data) {
    return <div className="p-6">Loading…</div>;
  }

  const fault = data.fault;
  const buildings = (buildingsData?.items ?? []).map((b) => ({ id: b.id, name: b.name }));

  return (
    <EditFaultPage
      faultId={faultId}
      initialData={{
        buildingId: fault.building_id,
        unitId: fault.unit_id,
        title: fault.title,
        description: fault.description,
        locationDescription: fault.location_description,
        category: fault.category,
        priority: fault.priority,
      }}
      buildings={buildings}
      units={[]}
      isSubmitting={updateFault.isPending}
      onSubmit={async (formData) => {
        try {
          await updateFault.mutateAsync({
            title: formData.title,
            description: formData.description,
            location_description: formData.locationDescription,
            category: formData.category,
          });
          showToast({ type: 'success', title: 'Updated', message: 'Fault updated' });
          navigate(`/faults/${faultId}`);
        } catch (err) {
          showToast({
            type: 'error',
            title: 'Failed to update fault',
            message: err instanceof Error ? err.message : 'Please try again.',
          });
        }
      }}
      onCancel={() => navigate(`/faults/${faultId}`)}
    />
  );
}

// ============================================
// Community Route Wrappers (Epic 42)
// ============================================

function FeedPageRoute() {
  const navigate = useNavigate();
  const { user } = useAuth();

  return (
    <FeedPage
      posts={[]}
      total={0}
      userName={user?.firstName ?? 'User'}
      onCreatePost={() => {}}
      onLikePost={() => {}}
      onViewPost={(id) => navigate(`/community/posts/${id}`)}
      onCommentPost={() => {}}
      onSharePost={() => {}}
      onEditPost={() => {}}
      onDeletePost={() => {}}
      onFilterChange={() => {}}
      onLoadMore={() => {}}
    />
  );
}

function GroupsPageRoute() {
  const navigate = useNavigate();

  return (
    <GroupsPage
      groups={[]}
      total={0}
      onNavigateToGroup={(id) => navigate(`/community/groups/${id}`)}
      onNavigateToCreate={() => navigate('/community/groups/new')}
      onJoinGroup={() => {}}
      onLeaveGroup={() => {}}
      onFilterChange={() => {}}
    />
  );
}

function CreateGroupPageRoute() {
  const navigate = useNavigate();
  const { showToast } = useToast();

  return (
    <CreateGroupPage
      onSubmit={() => {
        showToast({ type: 'success', title: 'Created', message: 'Group created' });
        navigate('/community/groups');
      }}
      onCancel={() => navigate('/community/groups')}
    />
  );
}

function GroupDetailPageRoute() {
  const { groupId } = useParams<{ groupId: string }>();
  const navigate = useNavigate();

  if (!groupId) {
    return <div>Group not found</div>;
  }

  // Mock group data
  const mockGroup: import('@ppt/api-client').CommunityGroup = {
    id: groupId,
    buildingId: 'bld-1',
    name: 'Sample Group',
    description: 'A sample community group',
    category: 'general',
    visibility: 'public',
    memberCount: 10,
    postCount: 0,
    isOfficial: false,
    createdBy: 'user-1',
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
  };

  return (
    <GroupDetailPage
      group={mockGroup}
      members={[]}
      onNavigateBack={() => navigate('/community/groups')}
      onJoin={() => {}}
      onLeave={() => {}}
      onEdit={() => {}}
      onDelete={() => navigate('/community/groups')}
      onNavigateToSettings={() => {}}
      onPromoteMember={() => {}}
      onRemoveMember={() => {}}
      onBanMember={() => {}}
    />
  );
}

function EventsPageRoute() {
  const navigate = useNavigate();

  return (
    <EventsPage
      events={[]}
      total={0}
      onNavigateToEvent={(id) => navigate(`/community/events/${id}`)}
      onNavigateToCreate={() => navigate('/community/events/new')}
      onRsvp={() => {}}
      onEditEvent={() => {}}
      onDeleteEvent={() => {}}
      onExportCalendar={() => {}}
      onFilterChange={() => {}}
    />
  );
}

function MarketplacePageRoute() {
  const navigate = useNavigate();

  return (
    <MarketplacePage
      items={[]}
      total={0}
      onNavigateToItem={(id) => navigate(`/community/marketplace/${id}`)}
      onNavigateToCreate={() => navigate('/community/marketplace/new')}
      onContactSeller={() => {}}
      onEditItem={() => {}}
      onDeleteItem={() => {}}
      onMarkSold={() => {}}
      onFilterChange={() => {}}
    />
  );
}

// ============================================
// Financial Route Wrappers (Epic 52)
// ============================================

/**
 * Route wrapper for financial dashboard (Epic 52, #975.1).
 *
 * Wires the hand-written financial api-client functions via TanStack Query:
 *   getARAgingReport({ organization_id })  — AR aging buckets + totals
 *   getOverdueInvoices(orgId)              — overdue invoice list
 *   listInvoices({ organization_id })      — full invoice list (for status counts)
 *
 * Metrics are derived from `arReport.totals` plus invoice sums; invoiceCounts are
 * derived client-side from the invoice list. recentPayments stays `[]` — there is
 * no org-wide payments endpoint in the api-client.
 */
function FinancialDashboardPageRoute() {
  const { user } = useAuth();
  const orgId = user?.organizationId ?? '';

  const { data: arReportData, isLoading: arLoading } = useQuery({
    queryKey: ['financial', 'ar-aging', orgId],
    queryFn: () => getARAgingReport({ organization_id: orgId }),
    enabled: !!orgId,
  });
  const { data: overdueData, isLoading: overdueLoading } = useQuery({
    queryKey: ['financial', 'overdue-invoices', orgId],
    queryFn: () => getOverdueInvoices(orgId),
    enabled: !!orgId,
  });
  const { data: invoicesData, isLoading: invoicesLoading } = useQuery({
    queryKey: ['financial', 'invoices', orgId],
    queryFn: () => listInvoices({ organization_id: orgId }),
    enabled: !!orgId,
  });

  const isLoading = arLoading || overdueLoading || invoicesLoading;

  const totals = arReportData?.totals ?? {
    current: 0,
    days_30: 0,
    days_60: 0,
    days_90_plus: 0,
    total: 0,
  };
  const invoices = invoicesData?.invoices ?? [];
  const overdueInvoices = overdueData ?? [];

  // Outstanding = sum of balances still due across all invoices.
  const totalOutstanding = invoices.reduce((sum, inv) => sum + (inv.balance_due ?? 0), 0);
  const totalOverdue = overdueInvoices.reduce((sum, inv) => sum + (inv.balance_due ?? 0), 0);
  const currency = invoices[0]?.currency ?? 'EUR';

  // Derive per-status counts client-side from the invoice list.
  const invoiceCounts = invoices.reduce(
    (acc, inv) => {
      if (inv.status === 'draft') acc.draft += 1;
      else if (inv.status === 'sent') acc.sent += 1;
      else if (inv.status === 'overdue') acc.overdue += 1;
      else if (inv.status === 'paid') acc.paid += 1;
      return acc;
    },
    { draft: 0, sent: 0, overdue: 0, paid: 0 }
  );

  return (
    <FinancialDashboardPage
      organizationId={orgId}
      buildings={[]}
      metrics={{
        totalBalance: totals.total,
        totalOutstanding,
        totalOverdue,
        currency,
      }}
      invoiceCounts={invoiceCounts}
      recentPayments={[]}
      overdueInvoices={overdueInvoices}
      arReport={{
        entries: arReportData?.entries ?? [],
        totals,
      }}
      isLoading={isLoading}
    />
  );
}

/**
 * Route wrapper for invoice management (Epic 52, #975.2).
 *
 * Lists invoices via listInvoices({ organization_id, status, limit, offset }) with
 * status + pagination state, and sends invoices via sendInvoice (invalidating the
 * list on success). ListInvoicesParams only supports status + unit_id + limit/offset
 * (no building or search), so only status + pagination are wired; buildings stays `[]`.
 */
function InvoiceManagementPageRoute() {
  const navigate = useNavigate();
  const { t } = useTranslation();
  const { user } = useAuth();
  const { showToast } = useToast();
  const queryClient = useQueryClient();
  const orgId = user?.organizationId ?? '';

  const [page, setPage] = useState(1);
  const [pageSize, setPageSize] = useState(10);
  const [statusFilter, setStatusFilter] = useState<
    import('@ppt/api-client').InvoiceStatus | undefined
  >();

  const { data, isLoading } = useQuery({
    queryKey: ['financial', 'invoices', orgId, statusFilter, page, pageSize],
    queryFn: () =>
      listInvoices({
        organization_id: orgId,
        status: statusFilter,
        limit: pageSize,
        offset: (page - 1) * pageSize,
      }),
    enabled: !!orgId,
  });

  const sendInvoiceMutation = useMutation({
    mutationFn: (invoiceId: string) => sendInvoice(invoiceId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['financial', 'invoices', orgId] });
      showToast({
        type: 'success',
        title: t('financial.invoices.sent', { defaultValue: 'Invoice sent' }),
        message: '',
      });
    },
    onError: (err) => {
      showToast({
        type: 'error',
        title: t('financial.invoices.sendFailed', { defaultValue: 'Failed to send invoice' }),
        message: err instanceof Error ? err.message : '',
      });
    },
  });

  return (
    <InvoiceManagementPage
      invoices={data?.invoices ?? []}
      total={data?.total ?? 0}
      buildings={[]}
      isLoading={isLoading}
      onNavigateToCreate={() => navigate('/financial/invoices/new')}
      onNavigateToDetail={(id: string) => navigate(`/financial/invoices/${id}`)}
      onSendInvoice={(id: string) => sendInvoiceMutation.mutate(id)}
      onFilterChange={(params) => {
        setStatusFilter(params.status);
        setPage(params.page);
        setPageSize(params.pageSize);
      }}
    />
  );
}

function PaymentManagementPageRoute() {
  const navigate = useNavigate();

  return (
    <PaymentManagementPage
      payments={[]}
      total={0}
      buildings={[]}
      unallocatedPayments={[]}
      unpaidInvoices={[]}
      metrics={{ totalReceived: 0, pendingReconciliation: 0, currency: 'EUR' }}
      onNavigateToRecord={() => navigate('/financial/payments/new')}
      onNavigateToDetail={(id: string) => navigate(`/financial/payments/${id}`)}
      onMatch={() => {}}
      onAutoMatch={() => {}}
      onFilterChange={() => {}}
    />
  );
}

function BudgetManagementPageRoute() {
  const navigate = useNavigate();

  return (
    <BudgetManagementPage
      budgets={[]}
      currentYear={new Date().getFullYear()}
      summary={{ totalBudget: 0, totalActual: 0, overallVariance: 0, currency: 'EUR' }}
      buildings={[]}
      onNavigateToCreate={() => navigate('/financial/budgets/new')}
      onNavigateToDetail={(id: string) => navigate(`/financial/budgets/${id}`)}
      onYearChange={() => {}}
      onBuildingChange={() => {}}
    />
  );
}

// ============================================
// Short-Term Rentals Route Wrappers (Epic 18, #976.1)
// ============================================

/**
 * Map a generated Rentals_RentalPlatform → the page-level BookingSource.
 * The generated platform enum carries 'vrbo' which the page lacks; fold it to 'other'.
 */
function mapRentalPlatformToSource(
  platform: import('@ppt/api-client').Rentals_RentalPlatform
): RentalBooking['source'] {
  switch (platform) {
    case 'airbnb':
      return 'airbnb';
    case 'booking':
      return 'booking';
    case 'direct':
      return 'direct';
    default:
      return 'other';
  }
}

/** Map a generated Rentals_RentalPlatform → the page PlatformType (airbnb | booking). */
function mapRentalPlatformToType(
  platform: import('@ppt/api-client').Rentals_RentalPlatform
): RentalPlatformType {
  return platform === 'booking' ? 'booking' : 'airbnb';
}

/** Map a generated Rentals_SyncStatus → the page ConnectionStatus. */
function mapSyncStatusToConnectionStatus(
  syncStatus: import('@ppt/api-client').Rentals_SyncStatus,
  isActive: boolean
): RentalPlatformConnection['status'] {
  if (!isActive) return 'disconnected';
  switch (syncStatus) {
    case 'synced':
      return 'connected';
    case 'pending':
      return 'pending';
    case 'error':
      return 'error';
    default:
      return 'disconnected';
  }
}

/** Map a generated Rentals_Reservation → the page RentalBooking. */
function mapReservationToBooking(
  res: import('@ppt/api-client').Rentals_Reservation
): RentalBooking {
  return {
    id: res.id,
    unitId: res.unitId,
    // The reservation payload does not carry building context; left blank.
    buildingId: '',
    guestName: res.guestName,
    guestEmail: res.guestEmail,
    guestPhone: res.guestPhone,
    checkIn: res.checkIn,
    checkOut: res.checkOut,
    status: res.status as RentalBookingStatus,
    source: mapRentalPlatformToSource(res.platform),
    platformBookingId: res.externalId,
    totalPrice: res.totalPrice?.amount,
    currency: res.totalPrice?.currency,
    guestCount: res.guestCount,
    notes: res.notes,
    createdAt: res.createdAt,
    updatedAt: res.updatedAt,
  };
}

/** Map a generated Rentals_PlatformConnection → the page PlatformConnection. */
function mapApiConnectionToUi(
  conn: import('@ppt/api-client').Rentals_PlatformConnection
): RentalPlatformConnection {
  return {
    id: conn.id,
    unitId: conn.unitId,
    platform: mapRentalPlatformToType(conn.platform),
    status: mapSyncStatusToConnectionStatus(conn.syncStatus, conn.isActive),
    platformPropertyId: conn.externalListingId,
    lastSyncAt: conn.lastSyncAt,
    isAutoSyncEnabled: conn.isActive,
    createdAt: conn.createdAt,
  };
}

/** Map a generated Rentals_GuestRegistration → the page RentalGuest. */
function mapApiGuestToUi(guest: import('@ppt/api-client').Rentals_GuestRegistration): RentalGuest {
  return {
    id: guest.id,
    bookingId: guest.reservationId ?? '',
    firstName: guest.firstName,
    lastName: guest.lastName,
    dateOfBirth: guest.dateOfBirth,
    nationality: guest.nationality,
    documentType: guest.documentType,
    documentNumber: guest.documentNumber,
    documentExpiry: guest.documentExpiry,
    registrationStatus: guest.submittedToAuthorities ? 'registered' : 'pending',
    registeredAt: guest.submittedAt,
    isPrimary: false,
    createdAt: guest.createdAt,
  };
}

/**
 * Build the auth header + tenant id required by the generated
 * ShortTermRentalsService methods. Returns null when either is missing so
 * the calling query stays disabled.
 */
function useRentalsAuth(): { authorization: string; xTenantId: string } | null {
  const { user } = useAuth();
  const token = getToken();
  if (!token || !user?.organizationId) return null;
  return { authorization: `Bearer ${token}`, xTenantId: user.organizationId };
}

/** Route wrapper for the rentals dashboard. */
function RentalsDashboardPageRoute() {
  const navigate = useNavigate();
  const auth = useRentalsAuth();

  const { data, isLoading } = useQuery({
    queryKey: ['rentals', 'reservations', auth?.xTenantId],
    queryFn: () =>
      ShortTermRentalsService.rentalsApiListReservations({
        authorization: auth!.authorization,
        xTenantId: auth!.xTenantId,
      }),
    enabled: !!auth,
  });
  const { data: connData } = useQuery({
    queryKey: ['rentals', 'connections', auth?.xTenantId],
    queryFn: () =>
      ShortTermRentalsService.rentalsApiListConnections({
        authorization: auth!.authorization,
        xTenantId: auth!.xTenantId,
      }),
    enabled: !!auth,
  });

  const bookings = (data?.data ?? []).map(mapReservationToBooking);
  const connections = (connData ?? []).map(mapApiConnectionToUi);
  const now = Date.now();
  const upcomingBookings = bookings.filter((b) => new Date(b.checkIn).getTime() >= now);
  const activeBookings = bookings.filter((b) => b.status === 'checked_in');

  const statistics: RentalStatistics = {
    totalBookings: bookings.length,
    activeBookings: activeBookings.length,
    upcomingBookings: upcomingBookings.length,
    totalRevenue: bookings.reduce((sum, b) => sum + (b.totalPrice ?? 0), 0),
    occupancyRate: 0,
    averageStayDuration: 0,
    pendingGuestRegistrations: 0,
    currency: bookings[0]?.currency ?? 'EUR',
  };

  return (
    <RentalsDashboardPage
      statistics={statistics}
      upcomingBookings={upcomingBookings}
      activeBookings={activeBookings}
      platformConnections={connections}
      isLoading={isLoading}
      onViewBooking={(id) => navigate(`/rentals/bookings/${id}`)}
      onCheckIn={(id) => navigate(`/rentals/bookings/${id}`)}
      onCheckOut={(id) => navigate(`/rentals/bookings/${id}`)}
      onViewAllBookings={() => navigate('/rentals/bookings')}
      onViewCalendar={() => navigate('/rentals/calendar')}
      onViewConnections={() => navigate('/rentals/connections')}
      onViewGuestRegistration={() => navigate('/rentals/guests')}
      onViewTaxReport={() => navigate('/rentals/reports')}
    />
  );
}

/** Route wrapper for platform connections. */
function PlatformConnectionsPageRoute() {
  const navigate = useNavigate();
  const auth = useRentalsAuth();
  const queryClient = useQueryClient();

  const { data, isLoading } = useQuery({
    queryKey: ['rentals', 'connections', auth?.xTenantId],
    queryFn: () =>
      ShortTermRentalsService.rentalsApiListConnections({
        authorization: auth!.authorization,
        xTenantId: auth!.xTenantId,
      }),
    enabled: !!auth,
  });

  const createConnection = useMutation({
    mutationFn: (vars: { unitId: string; platform: RentalPlatformType }) =>
      ShortTermRentalsService.rentalsApiCreateConnection({
        authorization: auth!.authorization,
        xTenantId: auth!.xTenantId,
        requestBody: { unitId: vars.unitId, platform: vars.platform },
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['rentals', 'connections'] });
    },
  });

  const syncPlatforms = useMutation({
    mutationFn: (unitId: string) =>
      ShortTermRentalsService.rentalsApiSyncPlatforms({
        authorization: auth!.authorization,
        xTenantId: auth!.xTenantId,
        unitId,
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['rentals', 'connections'] });
    },
  });

  const connections = (data ?? []).map(mapApiConnectionToUi);

  return (
    <PlatformConnectionsPage
      connections={connections}
      units={[]}
      isLoading={isLoading}
      onConnect={() => {}}
      onDisconnect={() => {}}
      onSync={(connectionId) => {
        const conn = connections.find((c) => c.id === connectionId);
        if (conn) syncPlatforms.mutate(conn.unitId);
      }}
      onSettings={() => {}}
      onCreateConnection={(unitId, platform) => createConnection.mutate({ unitId, platform })}
      onBack={() => navigate('/rentals')}
    />
  );
}

/** Route wrapper for the bookings list. */
function BookingsPageRoute() {
  const navigate = useNavigate();
  const auth = useRentalsAuth();
  const [filters, setFilters] = useState<BookingListParams>({});

  const { data, isLoading } = useQuery({
    queryKey: ['rentals', 'reservations', auth?.xTenantId, filters],
    queryFn: () =>
      ShortTermRentalsService.rentalsApiListReservations({
        authorization: auth!.authorization,
        xTenantId: auth!.xTenantId,
        unitId: filters.unitId,
        status: filters.status,
        from: filters.fromDate,
        to: filters.toDate,
        page: filters.page,
        limit: filters.limit,
      }),
    enabled: !!auth,
  });

  const bookings = (data?.data ?? []).map(mapReservationToBooking);

  return (
    <BookingsPage
      bookings={bookings}
      total={data?.pagination?.totalItems ?? bookings.length}
      buildings={[]}
      units={[]}
      isLoading={isLoading}
      onFilterChange={setFilters}
      onViewBooking={(id) => navigate(`/rentals/bookings/${id}`)}
      onEditBooking={(id) => navigate(`/rentals/bookings/${id}`)}
      onCancelBooking={() => {}}
      onCheckIn={() => {}}
      onCheckOut={() => {}}
      onCreateBooking={() => navigate('/rentals/bookings')}
      onBack={() => navigate('/rentals')}
    />
  );
}

/** Route wrapper for a single booking detail. */
function BookingDetailPageRoute() {
  const { bookingId } = useParams<{ bookingId: string }>();
  const navigate = useNavigate();
  const auth = useRentalsAuth();
  const queryClient = useQueryClient();

  const { data, isLoading } = useQuery({
    queryKey: ['rentals', 'reservation', bookingId, auth?.xTenantId],
    queryFn: () =>
      ShortTermRentalsService.rentalsApiGetReservation({
        authorization: auth!.authorization,
        xTenantId: auth!.xTenantId,
        id: bookingId!,
      }),
    enabled: !!auth && !!bookingId,
  });

  const checkIn = useMutation({
    mutationFn: () =>
      ShortTermRentalsService.rentalsApiCheckIn({
        authorization: auth!.authorization,
        xTenantId: auth!.xTenantId,
        id: bookingId!,
      }),
    onSuccess: () =>
      queryClient.invalidateQueries({ queryKey: ['rentals', 'reservation', bookingId] }),
  });
  const checkOut = useMutation({
    mutationFn: () =>
      ShortTermRentalsService.rentalsApiCheckOut({
        authorization: auth!.authorization,
        xTenantId: auth!.xTenantId,
        id: bookingId!,
      }),
    onSuccess: () =>
      queryClient.invalidateQueries({ queryKey: ['rentals', 'reservation', bookingId] }),
  });

  if (!bookingId) {
    return <div>Booking not found</div>;
  }

  if (isLoading || !data) {
    return (
      <div className="flex items-center justify-center py-12">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600" />
      </div>
    );
  }

  const booking: BookingWithGuests = { ...mapReservationToBooking(data), guests: [] };

  return (
    <BookingDetailPage
      booking={booking}
      isLoading={isLoading}
      onBack={() => navigate('/rentals/bookings')}
      onEdit={() => {}}
      onCancel={() => {}}
      onCheckIn={() => checkIn.mutate()}
      onCheckOut={() => checkOut.mutate()}
      onAddGuest={() => navigate('/rentals/guests')}
      onEditGuest={() => navigate('/rentals/guests')}
      onRegisterGuest={() => navigate('/rentals/guests')}
      onDeleteGuest={() => {}}
    />
  );
}

/** Route wrapper for the calendar view. No calendar endpoint exists yet; events stay empty. */
function RentalsCalendarPageRoute() {
  const navigate = useNavigate();

  return (
    <RentalsCalendarPage
      units={[]}
      events={[] as RentalCalendarEvent[]}
      isLoading={false}
      onUnitChange={() => {}}
      onMonthChange={() => {}}
      onEventClick={() => {}}
      onAddBlock={() => {}}
      onBack={() => navigate('/rentals')}
    />
  );
}

/** Route wrapper for guest registration. */
function GuestRegistrationPageRoute() {
  const navigate = useNavigate();
  const auth = useRentalsAuth();

  const { data, isLoading } = useQuery({
    queryKey: ['rentals', 'guests', auth?.xTenantId],
    queryFn: () =>
      ShortTermRentalsService.rentalsApiListGuests({
        authorization: auth!.authorization,
        xTenantId: auth!.xTenantId,
      }),
    enabled: !!auth,
  });

  // Group registered guests by reservation so the page can list per-booking
  // pending registrations. Without per-booking metadata we surface a single
  // synthetic grouping keyed by reservationId.
  const guests = (data?.data ?? []).map(mapApiGuestToUi);
  const byBooking = new Map<string, RentalGuest[]>();
  for (const g of guests) {
    const list = byBooking.get(g.bookingId) ?? [];
    list.push(g);
    byBooking.set(g.bookingId, list);
  }
  const bookingsWithPendingGuests = Array.from(byBooking.entries()).map(
    ([bookingId, bookingGuests]) => ({
      id: bookingId,
      guestName: bookingGuests[0]
        ? `${bookingGuests[0].firstName} ${bookingGuests[0].lastName}`
        : '',
      unitName: '',
      checkIn: '',
      checkOut: '',
      guests: bookingGuests,
    })
  );

  return (
    <GuestRegistrationPage
      bookingsWithPendingGuests={bookingsWithPendingGuests}
      isLoading={isLoading}
      onRegisterGuest={() => {}}
      onRegisterAllGuests={() => {}}
      onEditGuest={() => {}}
      onAddGuest={() => {}}
      onBack={() => navigate('/rentals')}
    />
  );
}

/** Route wrapper for tax reports. No tax-report endpoint exists yet; returns an empty report. */
function TaxReportPageRoute() {
  const navigate = useNavigate();

  return (
    <TaxReportPage
      buildings={[]}
      isLoading={false}
      onGenerateReport={async (params) =>
        ({
          year: params.year,
          jurisdiction: {
            country: params.jurisdiction,
            name: params.jurisdiction,
            defaultTaxRate: 0,
            requiresGuestRegistration: false,
          },
          reportType: params.reportType,
          generatedAt: new Date().toISOString(),
          currency: 'EUR',
          summary: {
            totalIncome: 0,
            totalBookings: 0,
            totalNightsOccupied: 0,
            averageOccupancyRate: 0,
            totalExpenses: 0,
            netProfit: 0,
            estimatedTax: 0,
            effectiveTaxRate: 0,
          },
          expensesByCategory: [],
          propertiesCovered: [],
        }) satisfies import('./features/rentals/types').TaxReportData
      }
      onExportReport={async () => {}}
      onBack={() => navigate('/rentals')}
    />
  );
}

function Home() {
  const { t } = useTranslation();
  const { isAuthenticated, user } = useAuth();
  const navigate = useNavigate();

  return (
    <div className="space-y-10">
      <section className="text-center py-10">
        <h1 className="text-3xl md:text-4xl font-bold mb-3">{t('home.title')}</h1>
        <p className="text-lg max-w-2xl mx-auto" style={{ color: 'var(--color-text-secondary)' }}>
          {t('home.welcome')}
        </p>
        {!isAuthenticated && (
          <div className="mt-6 flex flex-wrap justify-center gap-3">
            <button
              type="button"
              onClick={() => navigate('/login')}
              className="btn-primary-token px-5 py-2.5 rounded-lg font-medium"
            >
              {t('auth.signIn')}
            </button>
            <button
              type="button"
              onClick={() => navigate('/register')}
              className="btn-outline-token px-5 py-2.5 rounded-lg font-medium"
            >
              {t('auth.register', { defaultValue: 'Register' })}
            </button>
          </div>
        )}
        {isAuthenticated && (
          <div className="mt-6">
            <button
              type="button"
              onClick={() =>
                navigate(user?.role === 'manager' ? '/dashboard/manager' : '/dashboard/resident')
              }
              className="btn-primary-token px-5 py-2.5 rounded-lg font-medium"
            >
              {t('nav.dashboard', { defaultValue: 'Open dashboard' })}
            </button>
          </div>
        )}
      </section>

      <section className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
        {[
          { to: '/documents', titleKey: 'nav.documents' },
          { to: '/news', titleKey: 'nav.news' },
          { to: '/disputes', titleKey: 'nav.disputes' },
          { to: '/outages', titleKey: 'nav.outages' },
          { to: '/emergency', titleKey: 'nav.emergency' },
          { to: '/settings/accessibility', titleKey: 'nav.accessibility' },
        ].map((card) => (
          <Link
            key={card.to}
            to={card.to}
            className="block p-5 rounded-xl"
            style={{
              backgroundColor: 'var(--color-surface)',
              border: '1px solid var(--color-border)',
            }}
          >
            <span className="font-semibold" style={{ color: 'var(--color-text)' }}>
              {t(card.titleKey)}
            </span>
          </Link>
        ))}
      </section>
    </div>
  );
}

// Login component removed - now using LoginPage from ./pages/LoginPage

// ---------------------------------------------------------------------------
// Neighbor Route Wrappers (Epic 6, Story 6.6)
// ---------------------------------------------------------------------------

/**
 * Neighbors listing page — wired to backend GET /api/v1/buildings/{id}/neighbors.
 *
 * Building context: The route uses the first building from `useBuildings()`.
 * Once a building-selector is added to the app-shell, swap for the selected ID.
 */
function NeighborsPageRoute() {
  const navigate = useNavigate();
  const { data: buildingsData, isLoading: isLoadingBuildings } = useBuildings();

  // Pick the first building the user has access to; URL-param selection deferred.
  const buildingId = buildingsData?.items?.[0]?.id ?? '';
  const buildingName = buildingsData?.items?.[0]?.name;

  const { neighbors, isLoading, error } = useNeighbors(buildingId, !isLoadingBuildings);

  return (
    <NeighborsPage
      neighbors={neighbors}
      buildingName={buildingName}
      isLoading={isLoading || isLoadingBuildings}
      error={error}
      onViewProfile={(n) => navigate(`/neighbors/${n.id}`)}
      onContact={(n) => navigate(`/messages/new?recipientId=${n.id}`)}
      onManagePrivacy={() => navigate('/neighbors/privacy')}
    />
  );
}

/**
 * Neighbor detail page — extracts :neighborId from the URL.
 * Fetches the neighbor from the building list and finds by id.
 */
function NeighborDetailRoute() {
  const { neighborId } = useParams<{ neighborId: string }>();
  const navigate = useNavigate();
  const { t } = useTranslation();
  const { data: buildingsData } = useBuildings();

  const buildingId = buildingsData?.items?.[0]?.id ?? '';
  const { neighbors, isLoading } = useNeighbors(buildingId, !!buildingId);
  const neighbor = neighbors.find((n) => n.id === neighborId);

  if (!neighborId) {
    return (
      <div className="error-page">
        <h1>{t('errors.notFound')}</h1>
        <p>{t('errors.neighborNotFound', 'Neighbor not found.')}</p>
      </div>
    );
  }

  if (isLoading) {
    return (
      <div className="loading-page">
        <p>{t('common.loading')}</p>
      </div>
    );
  }

  if (!neighbor) {
    return (
      <div className="error-page">
        <h1>{t('errors.notFound')}</h1>
        <p>{t('errors.neighborNotFound', 'Neighbor not found.')}</p>
      </div>
    );
  }

  return (
    <NeighborDetailPage
      neighbor={neighbor}
      onBack={() => navigate('/neighbors')}
      onMessage={(n) => navigate(`/messages/new?recipientId=${n.id}`)}
    />
  );
}

// Reports Route Wrappers (Epic 81)

/**
 * Reports page route — wires pause/resume/update schedule hooks (Story 81.1).
 *
 * The EditScheduleModal pause/resume buttons call these handlers which in
 * turn invoke the live backend endpoints PUT /api/v1/reports/schedules/{id}/pause
 * and PUT /api/v1/reports/schedules/{id}/resume (from PR #448).
 */
function ReportsPageRoute() {
  const { showToast } = useToast();
  const { t } = useTranslation();
  const { user } = useAuth();
  const organizationId = user?.organizationId ?? '';

  // gap-81-1: cron-based update (cron_expression, recipients, enabled)
  const updateScheduleCronMutation = useUpdateScheduleCron();

  const handleUpdateSchedule = async (
    id: string,
    data: import('@ppt/api-client').CronScheduleUpdateRequest
  ) => {
    try {
      await updateScheduleCronMutation.mutateAsync({ id, data });
      showToast({
        type: 'success',
        title: t('common.success'),
        message: t('reports.schedule.updated', 'Schedule updated successfully.'),
      });
    } catch (e) {
      showToast({
        type: 'error',
        title: t('common.error'),
        message: t('reports.schedule.updateFailed', 'Failed to update schedule.'),
      });
      throw e;
    }
  };

  // --- Story 81.2: Execution history state and hooks ---
  const [activeScheduleId, setActiveScheduleId] = useState<string>('');
  const [executionOffset, setExecutionOffset] = useState(0);
  const EXECUTION_PAGE_SIZE = 20;

  const { data: executionHistoryData, isLoading: executionsLoading } = useReportExecutionHistory(
    { scheduleId: activeScheduleId },
    {
      limit: EXECUTION_PAGE_SIZE,
      offset: executionOffset,
      enabled: !!activeScheduleId,
      refetchInterval: activeScheduleId ? 10_000 : false,
    }
  );

  const downloadReport = useDownloadReport();
  const retryExecution = useRetryReportExecution();

  const executions = executionHistoryData?.executions ?? [];
  const hasMore = executionHistoryData?.hasMore ?? false;

  const handleFetchExecutions = (scheduleId: string) => {
    setActiveScheduleId(scheduleId);
    setExecutionOffset(0);
  };

  const handleLoadMoreExecutions = () => {
    setExecutionOffset((prev) => prev + EXECUTION_PAGE_SIZE);
  };

  const handleDownloadReport = (executionId: string) => {
    downloadReport.mutate(executionId, {
      onError: () => {
        showToast({
          type: 'error',
          title: t('common.error'),
          message: t('reports.execution.downloadFailed', 'Failed to download report.'),
        });
      },
    });
  };

  const handleRetryExecution = async (executionId: string) => {
    try {
      await retryExecution.mutateAsync(executionId);
      showToast({
        type: 'success',
        title: t('common.success'),
        message: t('reports.execution.retryQueued', 'Execution queued for retry.'),
      });
    } catch {
      showToast({
        type: 'error',
        title: t('common.error'),
        message: t('reports.execution.retryFailed', 'Failed to retry execution.'),
      });
      throw new Error('retry failed');
    }
  };

  return (
    <ReportsPage
      organizationId={organizationId}
      dataSources={[]}
      reports={[]}
      schedules={[]}
      kpis={[]}
      buildings={[]}
      trendAnalysis={
        {
          metric: '',
          period: 'monthly',
          current_value: 0,
          previous_value: 0,
          change: 0,
          change_percentage: 0,
          trend: 'stable' as const,
          anomalies: [],
        } as import('@ppt/api-client').TrendAnalysis
      }
      trendLines={[]}
      periodComparison={
        {
          metric: '',
          periods: [],
          difference: 0,
          difference_percentage: 0,
        } as import('@ppt/api-client').PeriodComparison
      }
      onUpdateSchedule={handleUpdateSchedule}
      executions={executions}
      executionsLoading={executionsLoading}
      executionsHasMore={hasMore}
      onFetchExecutions={handleFetchExecutions}
      onLoadMoreExecutions={handleLoadMoreExecutions}
      onDownloadReport={handleDownloadReport}
      onRetryExecution={handleRetryExecution}
    />
  );
}

/**
 * Schedule detail page route — renders execution history table inline (Story 81.2).
 *
 * Route: /reports/schedules/:scheduleId
 * Uses useReportExecutionHistory to fetch paginated executions and renders them
 * via ScheduleDetailPage. Falls back to MSW stub data when api-server is absent.
 */
function ScheduleDetailPageRoute() {
  const { scheduleId = '' } = useParams<{ scheduleId: string }>();
  const navigate = useNavigate();
  const { showToast } = useToast();
  const { t } = useTranslation();

  const EXECUTION_PAGE_SIZE = 20;
  const [executionOffset, setExecutionOffset] = useState(0);

  const {
    data: executionHistoryData,
    isLoading: executionsLoading,
    isError: executionsError,
  } = useReportExecutionHistory(
    { scheduleId },
    {
      limit: EXECUTION_PAGE_SIZE,
      offset: executionOffset,
      enabled: !!scheduleId,
      refetchInterval: 10_000,
    }
  );

  const downloadReport = useDownloadReport();
  const retryExecution = useRetryReportExecution();

  const executions = executionHistoryData?.executions ?? [];
  const hasMore = executionHistoryData?.hasMore ?? false;

  const handleLoadMore = () => setExecutionOffset((prev) => prev + EXECUTION_PAGE_SIZE);

  const handleDownloadReport = (executionId: string) => {
    downloadReport.mutate(executionId, {
      onError: () => {
        showToast({
          type: 'error',
          title: t('common.error'),
          message: t('reports.execution.downloadFailed', 'Failed to download report.'),
        });
      },
    });
  };

  const handleRetryExecution = async (executionId: string) => {
    try {
      await retryExecution.mutateAsync(executionId);
      showToast({
        type: 'success',
        title: t('common.success'),
        message: t('reports.execution.retryQueued', 'Execution queued for retry.'),
      });
    } catch {
      showToast({
        type: 'error',
        title: t('common.error'),
        message: t('reports.execution.retryFailed', 'Failed to retry execution.'),
      });
      throw new Error('retry failed');
    }
  };

  // Stub schedule object — schedule metadata will be enriched once the
  // GET /api/v1/reports/schedules/:id endpoint is wired in a future story.
  const stubSchedule: import('@ppt/api-client').ReportSchedule = {
    id: scheduleId,
    report_id: '',
    organization_id: '',
    name: `Schedule ${scheduleId}`,
    frequency: 'monthly',
    time: '08:00',
    timezone: 'UTC',
    format: 'pdf',
    recipients: [],
    is_active: true,
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
  };

  return (
    <ScheduleDetailPage
      schedule={stubSchedule}
      executions={executions}
      isLoading={executionsLoading}
      isError={executionsError}
      hasMore={hasMore}
      onLoadMore={handleLoadMore}
      onDownload={handleDownloadReport}
      onRetry={handleRetryExecution}
      onBack={() => navigate('/reports')}
    />
  );
}

/**
 * Neighbor privacy settings page — wired to GET/PUT /api/v1/users/me/privacy.
 */
function NeighborsPrivacySettingsRoute() {
  const navigate = useNavigate();
  const { showToast } = useToast();
  const { t } = useTranslation();

  const { settings, isLoading, isSubmitting, error, updateSettings } = usePrivacySettings();
  const [successMessage, setSuccessMessage] = useState<string | null>(null);

  const handleSubmit = async (newSettings: import('./features/neighbors').PrivacySettings) => {
    try {
      await updateSettings(newSettings);
      setSuccessMessage(t('neighbors.privacy.saved', 'Privacy settings saved.'));
      showToast({
        type: 'success',
        title: t('common.success'),
        message: t('neighbors.privacy.saved', 'Privacy settings saved.'),
      });
    } catch {
      showToast({
        type: 'error',
        title: t('common.error'),
        message: t('neighbors.privacy.saveFailed', 'Failed to save privacy settings.'),
      });
    }
  };

  return (
    <NeighborsPrivacySettingsPage
      settings={settings}
      isLoading={isLoading}
      error={error}
      isSubmitting={isSubmitting}
      successMessage={successMessage}
      onSubmit={handleSubmit}
      onBack={() => navigate('/neighbors')}
    />
  );
}

export default App;
