import {
  type MediationSession,
  type ResolveDisputeRequest,
  type ScheduleSessionRequest,
  type UpdateSessionRequest,
  useAssignMediator,
  useCancelSession,
  useDispute,
  useEscalateDispute,
  useMediationSessions,
  useResolveDispute,
  useScheduleSession,
  useUpdateSession,
} from '@ppt/api-client';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Spinner } from '../../../components/Spinner';
import { MediationAssignDialog } from '../components/MediationAssignDialog';
import { MediationChatThread } from '../components/MediationChatThread';
import { MediationEscalateDialog } from '../components/MediationEscalateDialog';
import { MediationResolutionForm } from '../components/MediationResolutionForm';
import { MediationSessionDialog } from '../components/MediationSessionDialog';
import { MediationSessionsPanel } from '../components/MediationSessionsPanel';
import { MediationSubmissionsPanel } from '../components/MediationSubmissionsPanel';
import { MediationTimelineView } from '../components/MediationTimelineView';
import { formatDisputeReference } from '../utils/formatReference';

type Tab = 'timeline' | 'chat' | 'submissions' | 'resolve';

const statusColors: Record<string, string> = {
  filed: 'bg-blue-100 text-blue-800',
  under_review: 'bg-yellow-100 text-yellow-800',
  mediation: 'bg-violet-100 text-violet-800',
  escalated: 'bg-red-100 text-red-800',
  resolved: 'bg-green-100 text-green-800',
  closed: 'bg-gray-100 text-gray-700',
};

const statusLabelKeys: Record<string, string> = {
  filed: 'disputes.statusFiled',
  under_review: 'disputes.statusUnderReview',
  mediation: 'disputes.statusInMediation',
  escalated: 'disputes.statusEscalated',
  resolved: 'disputes.statusResolved',
  closed: 'disputes.statusClosed',
};

export interface MediationWorkspacePageProps {
  disputeId: string;
  currentUserId?: string;
  currentUserName?: string;
  organizationId: string;
  isManager?: boolean;
  onBack: () => void;
  onToastSuccess: (title: string, message?: string) => void;
  onToastError: (title: string, message?: string) => void;
}

export function MediationWorkspacePage({
  disputeId,
  currentUserId,
  currentUserName,
  organizationId,
  isManager = false,
  onBack,
  onToastSuccess,
  onToastError,
}: MediationWorkspacePageProps) {
  const { t } = useTranslation();
  const [activeTab, setActiveTab] = useState<Tab>('timeline');
  const [showEscalateDialog, setShowEscalateDialog] = useState(false);
  const [showAssignDialog, setShowAssignDialog] = useState(false);
  // Session dialog: undefined = closed; null = schedule; session = reschedule.
  const [sessionDialog, setSessionDialog] = useState<MediationSession | null | undefined>(
    undefined
  );

  const {
    data: dispute,
    isLoading: disputeLoading,
    isError: disputeError,
    refetch: refetchDispute,
    isFetching: disputeFetching,
  } = useDispute(disputeId);
  const { data: sessions = [], isLoading: sessionsLoading } = useMediationSessions(disputeId);

  const resolveDispute = useResolveDispute(organizationId);
  const escalateDispute = useEscalateDispute(organizationId);
  const assignMediator = useAssignMediator();
  const scheduleSession = useScheduleSession(disputeId);
  const updateSession = useUpdateSession(disputeId);
  const cancelSession = useCancelSession(disputeId);

  const isMediator = !!dispute && dispute.assignedMediatorId === currentUserId;
  const isParty =
    !!dispute && (dispute.filedBy === currentUserId || dispute.respondentId === currentUserId);
  const canManage = isManager || isMediator;
  const canChat = canManage || isParty;
  const isActive = !!dispute && dispute.status !== 'resolved' && dispute.status !== 'closed';

  const handleResolve = async (data: ResolveDisputeRequest) => {
    try {
      await resolveDispute.mutateAsync({ disputeId, data });
      onToastSuccess(t('disputes.mediation.resolvedSuccess'), t('disputes.mediation.resolvedMsg'));
      setActiveTab('timeline');
    } catch (err) {
      onToastError(
        t('disputes.mediation.resolveError'),
        err instanceof Error ? err.message : undefined
      );
    }
  };

  const handleEscalate = async (reason: string) => {
    try {
      await escalateDispute.mutateAsync({ disputeId, data: { reason } });
      onToastSuccess(
        t('disputes.mediation.escalatedSuccess'),
        t('disputes.mediation.escalatedMsg')
      );
      setShowEscalateDialog(false);
    } catch (err) {
      onToastError(
        t('disputes.mediation.escalateError'),
        err instanceof Error ? err.message : undefined
      );
    }
  };

  const handleAssignMediator = async (mediatorId: string) => {
    try {
      await assignMediator.mutateAsync({ disputeId, data: { mediatorId } });
      onToastSuccess(t('disputes.mediation.assignedSuccess'), t('disputes.mediation.assignedMsg'));
      setShowAssignDialog(false);
    } catch (err) {
      onToastError(
        t('disputes.mediation.assignError'),
        err instanceof Error ? err.message : undefined
      );
    }
  };

  const handleScheduleSession = async (data: ScheduleSessionRequest) => {
    try {
      await scheduleSession.mutateAsync(data);
      onToastSuccess(
        t('disputes.mediation.sessions.scheduledSuccess'),
        t('disputes.mediation.sessions.scheduledMsg')
      );
      setSessionDialog(undefined);
    } catch (err) {
      onToastError(
        t('disputes.mediation.sessions.scheduleError'),
        err instanceof Error ? err.message : undefined
      );
    }
  };

  const handleRescheduleSession = async (sessionId: string, data: UpdateSessionRequest) => {
    try {
      await updateSession.mutateAsync({ sessionId, data });
      onToastSuccess(
        t('disputes.mediation.sessions.rescheduledSuccess'),
        t('disputes.mediation.sessions.rescheduledMsg')
      );
      setSessionDialog(undefined);
    } catch (err) {
      onToastError(
        t('disputes.mediation.sessions.rescheduleError'),
        err instanceof Error ? err.message : undefined
      );
    }
  };

  const handleCancelSession = async (session: MediationSession) => {
    if (!window.confirm(t('disputes.mediation.sessions.cancelConfirm'))) {
      return;
    }
    try {
      await cancelSession.mutateAsync(session.id);
      onToastSuccess(
        t('disputes.mediation.sessions.cancelledSuccess'),
        t('disputes.mediation.sessions.cancelledMsg')
      );
    } catch (err) {
      onToastError(
        t('disputes.mediation.sessions.cancelError'),
        err instanceof Error ? err.message : undefined
      );
    }
  };

  if (disputeLoading && !dispute) {
    return (
      <div className="flex justify-center py-16">
        <Spinner size="lg" />
      </div>
    );
  }

  // Fetch failed and we have no dispute to fall back on: show an explicit
  // error banner with a retry affordance instead of rendering the page with
  // 'unknown' status and empty placeholders.
  if (disputeError && !dispute) {
    return (
      <div className="max-w-5xl mx-auto px-4 py-8">
        <button
          type="button"
          onClick={onBack}
          className="text-sm text-violet-600 hover:text-violet-800 mb-4 flex items-center gap-1"
        >
          {t('disputes.mediation.backToDispute')}
        </button>
        <div role="alert" className="bg-red-50 border border-red-200 rounded-xl p-8 text-center">
          <svg
            className="w-12 h-12 text-red-400 mx-auto mb-4"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
            aria-hidden="true"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M12 9v2m0 4h.01M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z"
            />
          </svg>
          <p className="text-gray-900 font-medium">{t('disputes.mediation.loadError.title')}</p>
          <p className="text-gray-500 text-sm mt-1">{t('disputes.mediation.loadError.message')}</p>
          <button
            type="button"
            onClick={() => refetchDispute()}
            disabled={disputeFetching}
            className="mt-5 inline-flex items-center px-4 py-2 text-sm bg-violet-600 text-white rounded-lg hover:bg-violet-700 font-medium disabled:opacity-60 disabled:cursor-not-allowed"
          >
            {disputeFetching
              ? t('disputes.mediation.loadError.retrying')
              : t('disputes.mediation.loadError.retry')}
          </button>
        </div>
      </div>
    );
  }

  const disputeStatus = dispute?.status ?? 'unknown';
  const referenceNumber = dispute ? formatDisputeReference(dispute.id) : disputeId;
  const disputeTitle = dispute?.subject ?? t('disputes.title');
  const statusLabel = statusLabelKeys[disputeStatus]
    ? t(statusLabelKeys[disputeStatus])
    : disputeStatus;

  const tabs: { id: Tab; label: string }[] = [
    { id: 'timeline', label: t('disputes.mediation.tabTimeline') },
    { id: 'chat', label: t('disputes.mediation.tabDiscussion') },
    { id: 'submissions', label: t('disputes.mediation.tabSubmissions') },
    { id: 'resolve', label: t('disputes.mediation.tabResolution') },
  ];

  return (
    <div className="max-w-5xl mx-auto px-4 py-8">
      <div className="mb-6">
        <button
          type="button"
          onClick={onBack}
          className="text-sm text-violet-600 hover:text-violet-800 mb-4 flex items-center gap-1"
        >
          {t('disputes.mediation.backToDispute')}
        </button>
        <div className="flex items-start justify-between flex-wrap gap-4">
          <div>
            <div className="flex items-center gap-2 mb-1">
              <span className="text-sm font-mono text-gray-500">{referenceNumber}</span>
              <span
                className={`px-2 py-0.5 text-xs font-medium rounded-full ${
                  statusColors[disputeStatus] ?? 'bg-gray-100 text-gray-700'
                }`}
              >
                {statusLabel}
              </span>
            </div>
            <h1 className="text-2xl font-bold text-gray-900">{t('disputes.mediation.title')}</h1>
            <p className="text-gray-500 mt-0.5 text-sm">{disputeTitle}</p>
          </div>

          {isActive && canManage && (
            <div className="flex items-center gap-2">
              <button
                type="button"
                onClick={() => setShowAssignDialog(true)}
                className="px-4 py-2 text-sm border border-violet-300 text-violet-700 rounded-lg hover:bg-violet-50"
              >
                {t('disputes.mediation.assignMediator')}
              </button>
              <button
                type="button"
                onClick={() => setShowEscalateDialog(true)}
                className="px-4 py-2 text-sm border border-red-300 text-red-600 rounded-lg hover:bg-red-50"
              >
                {t('disputes.mediation.escalate')}
              </button>
              <button
                type="button"
                onClick={() => setActiveTab('resolve')}
                className="px-4 py-2 text-sm bg-green-600 text-white rounded-lg hover:bg-green-700 font-medium"
              >
                {t('disputes.mediation.resolveDispute')}
              </button>
            </div>
          )}
        </div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-4 gap-6">
        <div className="lg:col-span-1 space-y-4">
          <div className="bg-white rounded-xl shadow-sm border border-gray-100 p-4">
            <h3 className="text-sm font-semibold text-gray-700 mb-3">
              {t('disputes.mediation.disputePanel')}
            </h3>
            <dl className="space-y-2 text-sm">
              <div>
                <dt className="text-xs text-gray-400 uppercase tracking-wide">
                  {t('disputes.mediation.filedBy')}
                </dt>
                <dd className="font-medium text-gray-900">
                  {dispute?.filerDetails?.name ?? dispute?.filedBy ?? '—'}
                </dd>
              </div>
              {dispute?.respondent && (
                <div>
                  <dt className="text-xs text-gray-400 uppercase tracking-wide">
                    {t('disputes.mediation.respondent')}
                  </dt>
                  <dd className="font-medium text-gray-900">{dispute.respondent}</dd>
                </div>
              )}
              {dispute?.assignedMediator && (
                <div>
                  <dt className="text-xs text-gray-400 uppercase tracking-wide">
                    {t('disputes.mediation.mediator')}
                  </dt>
                  <dd className="font-medium text-violet-700">{dispute.assignedMediator}</dd>
                </div>
              )}
              <div>
                <dt className="text-xs text-gray-400 uppercase tracking-wide">
                  {t('disputes.mediation.filed')}
                </dt>
                <dd className="text-gray-600">
                  {dispute ? new Date(dispute.filedAt).toLocaleDateString() : '—'}
                </dd>
              </div>
              {dispute?.resolutionDeadline && (
                <div>
                  <dt className="text-xs text-gray-400 uppercase tracking-wide">
                    {t('disputes.mediation.deadline')}
                  </dt>
                  <dd className="text-orange-600 font-medium">
                    {new Date(dispute.resolutionDeadline).toLocaleDateString()}
                  </dd>
                </div>
              )}
            </dl>
          </div>

          <div className="bg-white rounded-xl shadow-sm border border-gray-100 p-4">
            <h3 className="text-sm font-semibold text-gray-700 mb-3">
              {t('disputes.mediation.parties')}
            </h3>
            <div className="space-y-2">
              {dispute?.filerDetails && (
                <div className="p-2.5 bg-blue-50 rounded-lg">
                  <p className="text-sm font-medium text-gray-900">{dispute.filerDetails.name}</p>
                  <p className="text-xs text-blue-600 mt-0.5">{t('disputes.mediation.filer')}</p>
                  {dispute.filerDetails.email && (
                    <p className="text-xs text-gray-500">{dispute.filerDetails.email}</p>
                  )}
                </div>
              )}
              {dispute?.respondentDetails && (
                <div className="p-2.5 bg-gray-50 rounded-lg">
                  <p className="text-sm font-medium text-gray-900">
                    {dispute.respondentDetails.name}
                  </p>
                  <p className="text-xs text-gray-500 mt-0.5">
                    {t('disputes.mediation.respondent')}
                  </p>
                  {dispute.respondentDetails.email && (
                    <p className="text-xs text-gray-500">{dispute.respondentDetails.email}</p>
                  )}
                </div>
              )}
              {dispute?.mediatorDetails && (
                <div className="p-2.5 bg-violet-50 rounded-lg">
                  <p className="text-sm font-medium text-gray-900">
                    {dispute.mediatorDetails.name}
                  </p>
                  <p className="text-xs text-violet-600 mt-0.5">
                    {t('disputes.mediation.mediator')}
                  </p>
                </div>
              )}
              {!dispute?.filerDetails && !dispute?.respondentDetails && (
                <p className="text-xs text-gray-400">
                  {t('disputes.mediation.partyDetailsUnavailable')}
                </p>
              )}
            </div>
          </div>

          <MediationSessionsPanel
            sessions={sessions}
            isLoading={sessionsLoading}
            canManage={isActive && canManage}
            onSchedule={() => setSessionDialog(null)}
            onReschedule={(session) => setSessionDialog(session)}
            onCancel={handleCancelSession}
          />

          <div className="bg-violet-50 rounded-xl border border-violet-100 p-4">
            <h3 className="text-sm font-semibold text-violet-900 mb-2">
              {t('disputes.mediation.guidelines')}
            </h3>
            <ul className="text-xs text-violet-800 space-y-1.5 list-disc list-inside">
              <li>{t('disputes.mediation.guideline1')}</li>
              <li>{t('disputes.mediation.guideline2')}</li>
              <li>{t('disputes.mediation.guideline3')}</li>
              <li>{t('disputes.mediation.guideline4')}</li>
            </ul>
          </div>
        </div>

        <div className="lg:col-span-3">
          <div className="bg-white rounded-xl shadow-sm border border-gray-100">
            <div className="border-b border-gray-200 px-6">
              <nav className="flex gap-6" aria-label={t('aria.workspaceTabs')}>
                {tabs.map((tab) => (
                  <button
                    key={tab.id}
                    type="button"
                    onClick={() => setActiveTab(tab.id)}
                    className={[
                      'py-4 text-sm font-medium border-b-2 transition-colors',
                      activeTab === tab.id
                        ? 'border-violet-600 text-violet-700'
                        : 'border-transparent text-gray-500 hover:text-gray-700',
                    ].join(' ')}
                  >
                    {tab.label}
                  </button>
                ))}
              </nav>
            </div>

            <div className="p-6">
              {activeTab === 'timeline' && <MediationTimelineView disputeId={disputeId} />}

              {activeTab === 'chat' && (
                <>
                  {canChat ? (
                    <MediationChatThread
                      disputeId={disputeId}
                      currentUserId={currentUserId}
                      currentUserName={currentUserName}
                      canSendPrivate={canManage}
                      onError={onToastError}
                    />
                  ) : (
                    <p className="text-sm text-gray-500 text-center py-8">
                      {t('disputes.mediation.mustBeParty')}
                    </p>
                  )}
                </>
              )}

              {activeTab === 'submissions' && (
                <MediationSubmissionsPanel
                  disputeId={disputeId}
                  canSubmit={isParty}
                  onToastSuccess={onToastSuccess}
                  onToastError={onToastError}
                />
              )}

              {activeTab === 'resolve' && (
                <>
                  {!isActive ? (
                    <div className="text-center py-8">
                      <span
                        className={`px-3 py-1.5 text-sm font-medium rounded-full ${
                          statusColors[disputeStatus] ?? 'bg-gray-100 text-gray-700'
                        }`}
                      >
                        {statusLabel}
                      </span>
                      <p className="text-gray-500 text-sm mt-3">
                        {t('disputes.mediation.noLongerActive')}
                      </p>
                    </div>
                  ) : canManage ? (
                    <>
                      <h2 className="text-base font-semibold text-gray-900 mb-4">
                        {t('disputes.mediation.recordResolution')}
                      </h2>
                      <MediationResolutionForm
                        isSubmitting={resolveDispute.isPending}
                        onResolve={handleResolve}
                        onCancel={() => setActiveTab('timeline')}
                      />
                    </>
                  ) : (
                    <p className="text-sm text-gray-500 text-center py-8">
                      {t('disputes.mediation.onlyManagersCanResolve')}
                    </p>
                  )}
                </>
              )}
            </div>
          </div>
        </div>
      </div>

      {showEscalateDialog && (
        <MediationEscalateDialog
          onConfirm={handleEscalate}
          onCancel={() => setShowEscalateDialog(false)}
          isSubmitting={escalateDispute.isPending}
        />
      )}

      {showAssignDialog && (
        <MediationAssignDialog
          onConfirm={handleAssignMediator}
          onCancel={() => setShowAssignDialog(false)}
          isSubmitting={assignMediator.isPending}
        />
      )}

      {sessionDialog !== undefined && (
        <MediationSessionDialog
          session={sessionDialog ?? undefined}
          onScheduleConfirm={handleScheduleSession}
          onRescheduleConfirm={handleRescheduleSession}
          onCancel={() => setSessionDialog(undefined)}
          isSubmitting={scheduleSession.isPending || updateSession.isPending}
        />
      )}
    </div>
  );
}
