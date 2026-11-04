/**
 * Disputes route group (Epic 77 / Epic 80).
 *
 * Owns the dispute API↔UI type mappers and the route-wrapper components, plus
 * the `<Route>` table fragment. Extracted from App.tsx so dispute work no longer
 * collides with other features on the central aggregator.
 */
import {
  useAcceptResolution,
  useCompleteActionItem,
  useCreateActionItem,
  useDeleteEvidence,
  useDispute,
  useDisputeActionItems,
  useDisputeEvidence,
  useDisputeResolutions,
  useDisputes,
  useDisputeTimeline,
  useEscalateDispute,
  useImplementResolution,
  useProposeResolution,
  useSendActionReminder,
  useUpdateDisputeStatus,
  useVoteOnResolution,
} from '@ppt/api-client';
import { useEffect, useState } from 'react';
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
import type { DisputeDetail as UiDisputeDetail } from '../../features/disputes/pages/DisputeDetailPage';
import { FileDisputePageRoute } from '../../features/disputes/pages/FileDisputePageRoute';
import { mapDisputeDetailForUi } from '../../features/disputes/utils/mapDisputeDetail';
import { DisputeDetailPage, DisputesPage, MediationWorkspacePage } from '../lazyRoutes';
import { isManagerRole } from '../shared';

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

  // Filter state for API query. The UI status/category/priority values are the
  // same strings the backend `GET /api/v1/disputes` list filters on.
  const [filters, setFilters] = useState<{
    status?: UiDisputeStatus;
    priority?: DisputePriority;
    category?: DisputeCategory;
    search?: string;
    page: number;
    pageSize: number;
  }>({ page: 1, pageSize: 10 });

  const apiQuery = {
    status: filters.status,
    category: filters.category,
    priority: filters.priority,
    search: filters.search,
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
  const disputes: DisputeSummary[] = data?.data ?? [];
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
 * Wires the detail page to the real api-server routes:
 *   - GET  /disputes/{id}                 dispute + parties (names/emails)
 *   - GET  /disputes/{id}/activities      timeline (actor names from parties)
 *   - GET  /disputes/{id}/evidence        evidence (uploader names from parties)
 *   - GET  /disputes/{id}/resolutions[/{rid}]  resolutions with votes
 *   - GET  /disputes/{id}/actions         action items
 *   - PATCH /disputes/{id}                status change (manager only)
 *   - DELETE evidence, propose/vote/accept/implement resolution,
 *     create/complete/remind action items, escalate an action item.
 *
 * Not offered because the backend has no route for it: uploading evidence
 * files (the evidence route only records metadata of an already-stored file)
 * and ticking off individual resolution terms.
 */
function DisputeDetailRoute() {
  const { t } = useTranslation();
  const { disputeId = '' } = useParams<{ disputeId: string }>();
  const navigate = useNavigate();
  const { user } = useAuth();
  const { showToast } = useToast();

  const { data: dispute, isLoading, error } = useDispute(disputeId);
  const { data: timelineData, isLoading: timelineLoading } = useDisputeTimeline(disputeId);
  const { data: evidenceData, isLoading: evidenceLoading } = useDisputeEvidence(disputeId);
  const { data: resolutionData } = useDisputeResolutions(disputeId);
  const { data: actionData } = useDisputeActionItems(disputeId);

  const updateStatus = useUpdateDisputeStatus(user?.organizationId ?? '');
  const deleteEvidence = useDeleteEvidence(disputeId);
  const proposeResolution = useProposeResolution(disputeId);
  const voteOnResolution = useVoteOnResolution(disputeId);
  const acceptResolution = useAcceptResolution(disputeId);
  const implementResolution = useImplementResolution(disputeId);
  const createAction = useCreateActionItem(disputeId);
  const completeAction = useCompleteActionItem(disputeId);
  const sendReminder = useSendActionReminder(disputeId);
  const escalate = useEscalateDispute(user?.organizationId ?? '');

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

  const ui = mapDisputeDetailForUi({
    dispute,
    timeline: timelineData,
    evidence: evidenceData,
    resolutions: resolutionData,
    actions: actionData,
    currentUserId: user?.id,
    unknownName: t('disputes.detail.unknownUser'),
  });

  /** Run a mutation and toast its outcome. */
  const run = async (action: () => Promise<unknown>, successTitle: string) => {
    try {
      await action();
      showToast({ type: 'success', title: successTitle, message: '' });
    } catch (err) {
      showToast({
        type: 'error',
        title: t('disputes.detail.actionFailed'),
        message: err instanceof Error ? err.message : t('auth.unexpectedError'),
      });
    }
  };

  return (
    <DisputeDetailPage
      // The page shows a spinner until the dispute has loaded.
      dispute={ui.dispute as UiDisputeDetail}
      parties={ui.parties}
      evidence={ui.evidence}
      timeline={ui.timeline}
      resolutions={ui.resolutions}
      actionItems={ui.actionItems}
      isManager={isManagerRole(user?.role)}
      isParty={ui.isParty}
      isMediator={ui.isMediator}
      isLoading={!ui.dispute || isLoading || timelineLoading || evidenceLoading}
      currentUserId={user?.id}
      onBack={() => navigate('/disputes')}
      onUpdateStatus={(status, reason) =>
        run(
          () => updateStatus.mutateAsync({ disputeId, data: { status, reason } }),
          t('disputes.statusUpdated')
        )
      }
      onDeleteEvidence={(id) =>
        run(() => deleteEvidence.mutateAsync(id), t('disputes.detail.evidenceDeleted'))
      }
      onProposeResolution={(resolutionText, terms) =>
        run(
          () => proposeResolution.mutateAsync({ resolutionText, terms }),
          t('disputes.detail.resolutionProposed')
        )
      }
      onVoteResolution={(resolutionId, accepted, comments) =>
        run(
          () => voteOnResolution.mutateAsync({ resolutionId, data: { accepted, comments } }),
          t('disputes.detail.voteRecorded')
        )
      }
      onAcceptResolution={(id) =>
        run(() => acceptResolution.mutateAsync(id), t('disputes.detail.resolutionAccepted'))
      }
      onImplementResolution={(id) =>
        run(() => implementResolution.mutateAsync(id), t('disputes.detail.resolutionImplemented'))
      }
      onCreateAction={(data) =>
        run(() => createAction.mutateAsync(data), t('disputes.detail.actionCreated'))
      }
      onCompleteAction={(actionId, notes) =>
        run(
          () => completeAction.mutateAsync({ actionId, notes }),
          t('disputes.detail.actionCompleted')
        )
      }
      onSendReminder={(actionId) =>
        run(() => sendReminder.mutateAsync(actionId), t('disputes.detail.reminderSent'))
      }
      onEscalate={(actionId) =>
        run(
          () =>
            escalate.mutateAsync({
              disputeId,
              data: {
                reason: t('disputes.detail.actionEscalationReason'),
                severity: 'formal_notice',
                actionItemId: actionId,
              },
            }),
          t('disputes.detail.actionEscalated')
        )
      }
      onNavigateToMediation={() => navigate(`/disputes/${disputeId}/mediation`)}
    />
  );
}

/**
 * Route wrapper for dispute mediation workspace (Epic 80, Story 80-3).
 *
 * Route: /disputes/:disputeId/mediation
 *
 * Renders the full MediationWorkspacePage, which threads the complete
 * sessions/submissions/resolution workflow to the backend:
 *   - Dispute timeline wired to useDisputeTimeline (real API)
 *   - Manager/tenant chat thread via useMediationNotes + useAddMediationNote
 *   - Mediation sessions (schedule / reschedule / cancel) via useMediationSessions
 *     + useScheduleSession + useUpdateSession + useCancelSession
 *   - Party submissions via the MediationSubmissionsPanel (real API)
 *   - Resolution form using useResolveDispute
 *   - Escalate dialog using useEscalateDispute
 *   - Assign mediator dialog using useAssignMediator
 *
 * The former standalone MediationPage (a prop-driven sessions/submissions stub
 * that was never routed) has been removed now that the workspace page threads
 * the whole flow end-to-end.
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
