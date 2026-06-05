/**
 * Disputes route group (Epic 77 / Epic 80).
 *
 * Owns the dispute API↔UI type mappers and the route-wrapper components, plus
 * the `<Route>` table fragment. Extracted from App.tsx so dispute work no longer
 * collides with other features on the central aggregator.
 */
import type {
  Dispute as ApiDispute,
  DisputeStatus as ApiDisputeStatus,
  DisputeType as ApiDisputeType,
  TimelineEventType,
} from '@ppt/api-client';
import {
  useDispute,
  useDisputeEvidence,
  useDisputes,
  useDisputeTimeline,
  useUpdateDisputeStatus,
} from '@ppt/api-client';
import { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Link, Route, useNavigate, useParams } from 'react-router-dom';
import { AuthRequiredGate, useToast } from '../../components';
import { useAuth } from '../../contexts';
import type {
  DisputeCategory,
  DisputePriority,
  DisputeSummary,
  DisputeStatus as UiDisputeStatus,
} from '../../features/disputes/components/DisputeCard';
import type { ActivityType } from '../../features/disputes/components/DisputeTimeline';
import type { DisputeDetail as UiDisputeDetail } from '../../features/disputes/pages/DisputeDetailPage';
import { FileDisputePageRoute } from '../../features/disputes/pages/FileDisputePageRoute';
import { DisputeDetailPage, DisputesPage, MediationWorkspacePage } from '../lazyRoutes';
import { isManagerRole } from '../shared';

// ============================================
// Dispute type mappers (API <-> UI)
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
function mapTimelineEventType(eventType: TimelineEventType): ActivityType {
  const mapping: Record<TimelineEventType, ActivityType> = {
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

// ============================================
// Route wrappers
// ============================================

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

  const isManager = isManagerRole(user?.role);

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

  const isManager = isManagerRole(user?.role);

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

/** Dispute Resolution routes (Epic 77). */
export function disputeRoutes() {
  return (
    <>
      <Route path="/disputes" element={<DisputesPageRoute />} />
      <Route path="/disputes/new" element={<FileDisputePageRoute />} />
      <Route path="/disputes/:disputeId" element={<DisputeDetailRoute />} />
      <Route path="/disputes/:disputeId/mediation" element={<DisputeMediationRoute />} />
    </>
  );
}
