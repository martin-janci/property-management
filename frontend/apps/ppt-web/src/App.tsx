import {
  createBuildingsApi,
  getToken,
  useCancelOutage,
  useCreateDispute,
  useCreateOutage,
  useDispute,
  useDisputes,
  useOutage,
  useOutages,
  useResolveOutage,
  useStartOutage,
  useUpdateOutage,
} from '@ppt/api-client';
import type {
  Dispute as ApiDispute,
  DisputeStatus as ApiDisputeStatus,
  DisputeType as ApiDisputeType,
  OutageCommodity as ApiOutageCommodity,
  OutageSeverity as ApiOutageSeverity,
  OutageStatus as ApiOutageStatus,
  OutageSummary as ApiOutageSummary,
  OutageListQuery,
} from '@ppt/api-client';
import { AccessibilityProvider, SkipNavigation } from '@ppt/ui-kit';
import { useQuery } from '@tanstack/react-query';
import { type ReactNode, useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { BrowserRouter, Link, Route, Routes, useNavigate, useParams } from 'react-router-dom';
import {
  AnnouncerProvider,
  ConnectionStatus,
  LanguageSwitcher,
  OfflineIndicator,
  ToastProvider,
  useToast,
} from './components';
import './styles/accessibility.css';
import './features/settings/styles/accessibility.css';
import { AuthProvider, WebSocketProvider, useAuth } from './contexts';
import { CommandPaletteDialog, useNavigationCommands } from './features/command-palette';
import { ManagerDashboardPage, ResidentDashboardPage } from './features/dashboard';
import type {
  DisputeCategory,
  DisputePriority,
  DisputeSummary,
  DisputeStatus as UiDisputeStatus,
} from './features/disputes/components/DisputeCard';
import type { ListOutagesParams, OutageDetail } from './features/outages';
// Lazy-loaded route components for code splitting (Epic 130)
import {
  AccessibilitySettingsPage,
  ArticleDetailPage,
  CreateOutagePage,
  DisputesPage,
  DocumentDetailPage,
  DocumentUploadPage,
  DocumentsPage,
  EditOutagePage,
  EmergencyContactDirectoryPage,
  FileDisputePage,
  LoginPage,
  NewsListPage,
  OutagesPage,
  PrivacySettingsPage,
  ViewOutagePage,
} from './routes';

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
  return (
    <nav className="app-nav" aria-label="Main navigation">
      <Link to="/">{t('nav.home')}</Link>
      <Link to="/documents">{t('nav.documents')}</Link>
      <Link to="/news">{t('nav.news')}</Link>
      <Link to="/emergency">{t('nav.emergency')}</Link>
      <Link to="/disputes">{t('nav.disputes')}</Link>
      <Link to="/outages">{t('nav.outages')}</Link>
      <Link to="/settings/accessibility">{t('nav.accessibility')}</Link>
      <Link to="/settings/privacy">{t('nav.privacy')}</Link>
      <ConnectionStatus />
      <LanguageSwitcher />
    </nav>
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
        <ToastProvider>
          <AnnouncerProvider>
            <WebSocketWrapper>
              <BrowserRouter>
                <CommandPaletteWrapper>
                  <SkipNavigation mainContentId="main-content" />
                  <OfflineIndicator />
                  <div className="app">
                    <AppNavigation />
                    <main id="main-content">
                      <Routes>
                        <Route path="/" element={<Home />} />
                        <Route path="/login" element={<LoginPage />} />
                        {/* Dashboard routes (Epic 124) */}
                        <Route path="/dashboard/manager" element={<ManagerDashboardPage />} />
                        <Route path="/dashboard/resident" element={<ResidentDashboardPage />} />
                        {/* Document Intelligence routes (Epic 39) */}
                        <Route path="/documents" element={<DocumentsPageRoute />} />
                        <Route path="/documents/upload" element={<DocumentUploadPage />} />
                        <Route path="/documents/:documentId" element={<DocumentDetailRoute />} />
                        {/* News routes (Epic 59) */}
                        <Route path="/news" element={<NewsListPage />} />
                        <Route path="/news/:articleId" element={<ArticleDetailRoute />} />
                        {/* Emergency contacts route (Epic 62) */}
                        <Route path="/emergency" element={<EmergencyContactDirectoryPage />} />
                        {/* Accessibility settings route (Epic 60) */}
                        <Route
                          path="/settings/accessibility"
                          element={<AccessibilitySettingsPage />}
                        />
                        {/* Privacy settings route (Epic 63) */}
                        <Route path="/settings/privacy" element={<PrivacySettingsPage />} />
                        {/* Dispute Resolution routes (Epic 77) */}
                        <Route path="/disputes" element={<DisputesPageRoute />} />
                        <Route path="/disputes/new" element={<FileDisputePageRoute />} />
                        <Route path="/disputes/:disputeId" element={<DisputeDetailRoute />} />
                        {/* Outages routes (UC-12) */}
                        <Route path="/outages" element={<OutagesPageRoute />} />
                        <Route path="/outages/new" element={<CreateOutagePageRoute />} />
                        <Route path="/outages/:outageId" element={<ViewOutagePageRoute />} />
                        <Route path="/outages/:outageId/edit" element={<EditOutagePageRoute />} />
                      </Routes>
                    </main>
                  </div>
                </CommandPaletteWrapper>
              </BrowserRouter>
            </WebSocketWrapper>
          </AnnouncerProvider>
        </ToastProvider>
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
    return (
      <div className="error-page">
        <h1>{t('errors.authenticationRequired')}</h1>
        <p>{t('errors.missingOrgContext')}</p>
        <Link to="/login">{t('auth.signIn')}</Link>
      </div>
    );
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
 * Route wrapper for file dispute page (Epic 77, Story 80.2).
 *
 * Uses useCreateDispute mutation from @ppt/api-client for creating disputes.
 * Implements real API integration with toast notifications.
 * Transforms UI form data to API CreateDisputeRequest format.
 */
function FileDisputePageRoute() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { user } = useAuth();
  const { showToast } = useToast();

  // Require organization context for filing disputes
  if (!user?.organizationId) {
    return (
      <div className="error-page">
        <h1>{t('errors.authenticationRequired')}</h1>
        <p>{t('errors.missingOrgContext')}</p>
        <Link to="/login">{t('auth.signIn')}</Link>
      </div>
    );
  }

  const organizationId = user.organizationId;

  const createDispute = useCreateDispute(organizationId);

  // Handle form submission - transform UI data to API format
  const handleSubmit = async (formData: {
    category: DisputeCategory;
    title: string;
    description: string;
    desiredResolution?: string;
    respondentIds: string[];
    buildingId?: string;
    unitId?: string;
  }) => {
    // Validate unitId is provided before submission
    if (!formData.unitId) {
      showToast({
        type: 'error',
        title: t('disputes.unitRequired'),
        message: t('disputes.selectUnit'),
      });
      return;
    }

    // Warn if multiple respondents selected (API only supports one)
    if (formData.respondentIds.length > 1) {
      showToast({
        type: 'warning',
        title: t('disputes.multipleRespondents'),
        message: t('disputes.multipleRespondentsMsg'),
      });
    }

    try {
      // Transform UI form data to API CreateDisputeRequest
      const apiRequest = {
        type: mapCategoryToType(formData.category),
        subject: formData.title,
        // Combine description and desired resolution with clear delimiters
        description: formData.desiredResolution
          ? `Description:\n${formData.description}\n\n---\nDesired Resolution:\n${formData.desiredResolution}`
          : formData.description,
        unitId: formData.unitId,
        respondentId: formData.respondentIds[0],
      };

      await createDispute.mutateAsync(apiRequest);
      showToast({
        type: 'success',
        title: t('disputes.filedSuccessfully'),
        message: t('disputes.submittedMsg'),
      });
      navigate('/disputes');
    } catch (error) {
      showToast({
        type: 'error',
        title: t('disputes.failedToFile'),
        message: error instanceof Error ? error.message : t('auth.unexpectedError'),
      });
    }
  };

  const handleCancel = () => {
    navigate('/disputes');
  };

  return (
    <FileDisputePage
      onSubmit={handleSubmit}
      onCancel={handleCancel}
      isSubmitting={createDispute.isPending}
    />
  );
}

/**
 * Route wrapper for dispute detail page (Epic 77, Story 80.1).
 *
 * Uses useDispute hook from @ppt/api-client for data fetching.
 * Implements real API integration with loading/error states.
 * Maps API DisputeWithDetails to UI-friendly display format.
 */
function DisputeDetailRoute() {
  const { t } = useTranslation();
  const { disputeId } = useParams<{ disputeId: string }>();
  const { showToast } = useToast();

  const { data: dispute, isLoading, error, refetch } = useDispute(disputeId ?? '');

  // Use useEffect for error toast to prevent spam on re-renders
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

  if (isLoading) {
    return (
      <div className="loading-page">
        <h1>{t('common.loadingDispute')}</h1>
        <p>{t('common.pleaseWait')}</p>
      </div>
    );
  }

  // Add retry button for error states
  if (error) {
    return (
      <div className="error-page">
        <h1>{t('errors.errorLoadingDispute')}</h1>
        <p>{t('errors.disputeLoadError')}</p>
        <div className="error-actions">
          <button
            type="button"
            onClick={() => refetch()}
            className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 mr-2"
          >
            {t('common.tryAgain')}
          </button>
          <Link to="/disputes" className="px-4 py-2 border border-gray-300 rounded-lg">
            {t('common.backToDisputes')}
          </Link>
        </div>
      </div>
    );
  }

  if (!dispute) {
    return (
      <div className="error-page">
        <h1>{t('errors.disputeNotFound')}</h1>
        <p>{t('errors.disputeNotExist')}</p>
        <Link to="/disputes">{t('common.backToDisputes')}</Link>
      </div>
    );
  }

  // Map API type to UI category for display
  const category = mapTypeToCategory(dispute.type);
  const status = mapApiStatusToUiStatus(dispute.status);

  return (
    <div className="dispute-detail-page">
      <h1>Dispute: {dispute.subject}</h1>
      <div className="dispute-meta">
        <span className={`status status--${status}`}>{status.split('_').join(' ')}</span>
        <span className="type">{category.split('_').join(' ')}</span>
      </div>
      <div className="dispute-description">
        <h2>Description</h2>
        <p>{dispute.description}</p>
      </div>
      <div className="dispute-timeline">
        <h2>Timeline</h2>
        {dispute.timeline && dispute.timeline.length > 0 ? (
          <ul>
            {dispute.timeline.map((event) => (
              <li key={event.id}>
                <strong>{event.eventType.split('_').join(' ')}</strong>: {event.description}
                <span className="text-gray-500 ml-2">
                  ({new Date(event.createdAt).toLocaleDateString()})
                </span>
              </li>
            ))}
          </ul>
        ) : (
          <p>
            <em>No timeline events yet.</em>
          </p>
        )}
      </div>
      <Link to="/disputes">{t('common.backToDisputes')}</Link>
    </div>
  );
}

/**
 * Route wrapper for outages list page (UC-12).
 * Manages filter state and navigation callbacks.
 */
function OutagesPageRoute() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { user } = useAuth();
  const [queryParams, setQueryParams] = useState<OutageListQuery>({ limit: 10, offset: 0 });

  const { data, isLoading } = useOutages(queryParams);

  if (!user?.organizationId) {
    return (
      <div className="error-page">
        <h1>{t('errors.authenticationRequired')}</h1>
        <p>{t('errors.missingOrgContext')}</p>
        <Link to="/login">{t('auth.signIn')}</Link>
      </div>
    );
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

  if (!user?.organizationId) {
    return (
      <div className="error-page">
        <h1>{t('errors.authenticationRequired')}</h1>
        <p>{t('errors.missingOrgContext')}</p>
        <Link to="/login">{t('auth.signIn')}</Link>
      </div>
    );
  }

  const buildingsApi = useMemo(
    () =>
      createBuildingsApi({
        baseUrl: '',
        accessToken: getToken() ?? undefined,
      }),
    []
  );

  const { data: buildingsData } = useQuery({
    queryKey: ['buildings'],
    queryFn: () => buildingsApi.list(),
  });

  const buildings = useMemo(
    () =>
      (buildingsData?.items ?? []).map((b) => ({
        id: b.id,
        name: b.name,
        address: `${b.address.street}, ${b.address.city}`,
      })),
    [buildingsData]
  );

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
      isLoading={createOutage.isPending}
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
    createdByName: outageData.createdBy,
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

  if (!outageId) {
    return (
      <div className="error-page">
        <h1>{t('errors.outageNotFound')}</h1>
        <p>{t('errors.outageNotFoundDesc')}</p>
        <Link to="/outages">{t('common.backToOutages')}</Link>
      </div>
    );
  }

  const buildingsApi = useMemo(
    () =>
      createBuildingsApi({
        baseUrl: '',
        accessToken: getToken() ?? undefined,
      }),
    []
  );

  const { data: buildingsData } = useQuery({
    queryKey: ['buildings'],
    queryFn: () => buildingsApi.list(),
  });

  const buildings = useMemo(
    () =>
      (buildingsData?.items ?? []).map((b) => ({
        id: b.id,
        name: b.name,
        address: `${b.address.street}, ${b.address.city}`,
      })),
    [buildingsData]
  );

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

  if (isLoadingOutage || !outageData) {
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
    createdByName: outageData.createdBy,
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

function Home() {
  const { t } = useTranslation();
  return (
    <div>
      <h1>{t('home.title')}</h1>
      <p>{t('home.welcome')}</p>
    </div>
  );
}

// Login component removed - now using LoginPage from ./pages/LoginPage

export default App;
