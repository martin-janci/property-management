import {
  type ResolveDisputeRequest,
  useAssignMediator,
  useDispute,
  useDisputeTimeline,
  useEscalateDispute,
  useResolveDispute,
} from '@ppt/api-client';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { MediationChatThread } from '../components/MediationChatThread';
import { MediationResolutionForm } from '../components/MediationResolutionForm';
import { formatTime } from '../utils/formatTime';

// ============================================
// Types
// ============================================

type Tab = 'timeline' | 'chat' | 'resolve';

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

type TimelineEventType =
  | 'dispute_filed'
  | 'status_changed'
  | 'mediator_assigned'
  | 'evidence_added'
  | 'note_added'
  | 'meeting_scheduled'
  | 'resolution_proposed'
  | 'resolution_accepted'
  | 'escalated'
  | 'closed';

const eventIcons: Record<TimelineEventType, string> = {
  dispute_filed: '+',
  status_changed: '~',
  mediator_assigned: 'M',
  evidence_added: 'E',
  note_added: 'N',
  meeting_scheduled: 'S',
  resolution_proposed: 'P',
  resolution_accepted: 'A',
  escalated: '!',
  closed: 'X',
};

const eventColors: Record<TimelineEventType, string> = {
  dispute_filed: 'bg-blue-100 text-blue-700',
  status_changed: 'bg-yellow-100 text-yellow-700',
  mediator_assigned: 'bg-violet-100 text-violet-700',
  evidence_added: 'bg-indigo-100 text-indigo-700',
  note_added: 'bg-gray-100 text-gray-600',
  meeting_scheduled: 'bg-orange-100 text-orange-700',
  resolution_proposed: 'bg-cyan-100 text-cyan-700',
  resolution_accepted: 'bg-emerald-100 text-emerald-700',
  escalated: 'bg-red-100 text-red-700',
  closed: 'bg-gray-100 text-gray-700',
};

// ============================================
// TimelineView
// ============================================

function TimelineView({ disputeId }: { disputeId: string }) {
  const { t, i18n } = useTranslation();
  const { data: events = [], isLoading } = useDisputeTimeline(disputeId);

  if (isLoading) {
    return (
      <div className="flex justify-center py-8">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-violet-600" />
      </div>
    );
  }

  if (events.length === 0) {
    return <p className="text-sm text-gray-500 py-4">{t('disputes.mediation.noTimelineEvents')}</p>;
  }

  return (
    <div className="relative">
      <div className="absolute left-4 top-0 bottom-0 w-px bg-gray-200" />
      <div className="space-y-4">
        {events.map((event) => {
          const eventType = event.eventType as TimelineEventType;
          const icon = eventIcons[eventType] ?? '·';
          const color = eventColors[eventType] ?? 'bg-gray-100 text-gray-600';

          return (
            <div key={event.id} className="relative flex items-start gap-4 pl-10">
              <div
                className={`absolute left-0 w-8 h-8 rounded-full flex items-center justify-center text-xs font-bold ${color}`}
              >
                {icon}
              </div>
              <div className="flex-1 bg-white rounded-lg border border-gray-200 px-4 py-3">
                <div className="flex items-center justify-between">
                  <span className="text-sm font-medium text-gray-900">{event.actorName}</span>
                  <span className="text-xs text-gray-400">
                    {formatTime(event.createdAt, i18n.language)}
                  </span>
                </div>
                <p className="mt-0.5 text-sm text-gray-700">{event.description}</p>
                {event.metadata &&
                  'oldStatus' in event.metadata &&
                  'newStatus' in event.metadata && (
                    <p className="mt-1 text-xs text-gray-500">
                      Status: {String(event.metadata.oldStatus)} →{' '}
                      {String(event.metadata.newStatus)}
                    </p>
                  )}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}

// ============================================
// EscalateDialog
// ============================================

interface EscalateDialogProps {
  onConfirm: (reason: string) => void;
  onCancel: () => void;
  isSubmitting: boolean;
}

function EscalateDialog({ onConfirm, onCancel, isSubmitting }: EscalateDialogProps) {
  const { t } = useTranslation();
  const [reason, setReason] = useState('');

  return (
    <div className="fixed inset-0 z-50 overflow-y-auto" role="dialog" aria-modal="true">
      <button
        type="button"
        className="fixed inset-0 bg-black bg-opacity-50 cursor-default"
        onClick={onCancel}
        onKeyDown={(e) => e.key === 'Escape' && onCancel()}
        aria-label={t('disputes.mediation.cancel')}
      />
      <div className="flex min-h-full items-center justify-center p-4">
        <div className="relative w-full max-w-md bg-white rounded-xl shadow-xl p-6">
          <h2 className="text-lg font-semibold text-gray-900 mb-4">
            {t('disputes.mediation.escalateTitle')}
          </h2>
          <p className="text-sm text-gray-600 mb-4">
            {t('disputes.mediation.escalateDescription')}
          </p>
          <label className="block text-sm font-medium text-gray-700 mb-1">
            {t('disputes.mediation.reasonForEscalation')} <span className="text-red-500">*</span>
          </label>
          <textarea
            value={reason}
            onChange={(e) => setReason(e.target.value)}
            rows={4}
            placeholder={t('disputes.mediation.reasonPlaceholder')}
            className="w-full rounded-lg border border-gray-300 px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-red-500"
          />
          <div className="flex justify-end gap-3 mt-5">
            <button
              type="button"
              onClick={onCancel}
              className="px-4 py-2 text-sm border border-gray-300 rounded-lg hover:bg-gray-50"
            >
              {t('disputes.mediation.cancel')}
            </button>
            <button
              type="button"
              onClick={() => onConfirm(reason)}
              disabled={!reason.trim() || isSubmitting}
              className="px-4 py-2 text-sm bg-red-600 text-white rounded-lg hover:bg-red-700 disabled:opacity-50 font-medium"
            >
              {isSubmitting
                ? t('disputes.mediation.escalatingBtn')
                : t('disputes.mediation.escalateBtn')}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

// ============================================
// AssignMediatorDialog
// ============================================

interface AssignMediatorDialogProps {
  onConfirm: (mediatorId: string) => void;
  onCancel: () => void;
  isSubmitting: boolean;
}

function AssignMediatorDialog({ onConfirm, onCancel, isSubmitting }: AssignMediatorDialogProps) {
  const { t } = useTranslation();
  const [mediatorId, setMediatorId] = useState('');

  return (
    <div className="fixed inset-0 z-50 overflow-y-auto" role="dialog" aria-modal="true">
      <button
        type="button"
        className="fixed inset-0 bg-black bg-opacity-50 cursor-default"
        onClick={onCancel}
        onKeyDown={(e) => e.key === 'Escape' && onCancel()}
        aria-label={t('disputes.mediation.cancel')}
      />
      <div className="flex min-h-full items-center justify-center p-4">
        <div className="relative w-full max-w-md bg-white rounded-xl shadow-xl p-6">
          <h2 className="text-lg font-semibold text-gray-900 mb-4">
            {t('disputes.mediation.assignMediatorTitle')}
          </h2>
          <p className="text-sm text-gray-600 mb-4">
            {t('disputes.mediation.assignMediatorDescription')}
          </p>
          <label className="block text-sm font-medium text-gray-700 mb-1">
            {t('disputes.mediation.mediatorUserId')} <span className="text-red-500">*</span>
          </label>
          <input
            type="text"
            value={mediatorId}
            onChange={(e) => setMediatorId(e.target.value)}
            placeholder="user_xxxxxxxxxxxx"
            className="w-full rounded-lg border border-gray-300 px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-violet-500"
          />
          <p className="mt-1 text-xs text-gray-400">{t('disputes.mediation.userPickerNote')}</p>
          <div className="flex justify-end gap-3 mt-5">
            <button
              type="button"
              onClick={onCancel}
              className="px-4 py-2 text-sm border border-gray-300 rounded-lg hover:bg-gray-50"
            >
              {t('disputes.mediation.cancel')}
            </button>
            <button
              type="button"
              onClick={() => onConfirm(mediatorId)}
              disabled={!mediatorId.trim() || isSubmitting}
              className="px-4 py-2 text-sm bg-violet-600 text-white rounded-lg hover:bg-violet-700 disabled:opacity-50 font-medium"
            >
              {isSubmitting ? t('disputes.mediation.assigning') : t('disputes.mediation.assign')}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

// ============================================
// Main workspace page
// ============================================

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

  const { data: dispute, isLoading: disputeLoading } = useDispute(disputeId);

  const resolveDispute = useResolveDispute(organizationId);
  const escalateDispute = useEscalateDispute(organizationId);
  const assignMediator = useAssignMediator();

  const isMediator = !!dispute && dispute.assignedMediatorId === currentUserId;
  const isParty =
    !!dispute && (dispute.filedBy === currentUserId || dispute.respondentId === currentUserId);
  const canManage = isManager || isMediator;
  const canChat = canManage || isParty;
  // Fix: false when dispute is undefined (error/loading path)
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

  if (disputeLoading && !dispute) {
    return (
      <div className="flex justify-center py-16">
        <div className="animate-spin rounded-full h-10 w-10 border-b-2 border-violet-600" />
      </div>
    );
  }

  const disputeStatus = dispute?.status ?? 'unknown';
  const referenceNumber = dispute ? `DSP-${dispute.id.toUpperCase()}` : disputeId;
  const disputeTitle = dispute?.subject ?? t('disputes.title');
  const statusLabel = statusLabelKeys[disputeStatus]
    ? t(statusLabelKeys[disputeStatus])
    : disputeStatus;

  const tabs: { id: Tab; label: string }[] = [
    { id: 'timeline', label: t('disputes.mediation.tabTimeline') },
    { id: 'chat', label: t('disputes.mediation.tabDiscussion') },
    { id: 'resolve', label: t('disputes.mediation.tabResolution') },
  ];

  return (
    <div className="max-w-5xl mx-auto px-4 py-8">
      {/* Header */}
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

          {/* Action buttons */}
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
        {/* Left sidebar */}
        <div className="lg:col-span-1 space-y-4">
          {/* Dispute summary */}
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

          {/* Parties */}
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

          {/* Mediation guidelines */}
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

        {/* Main content */}
        <div className="lg:col-span-3">
          <div className="bg-white rounded-xl shadow-sm border border-gray-100">
            {/* Tabs */}
            <div className="border-b border-gray-200 px-6">
              <nav className="flex gap-6" aria-label="Workspace tabs">
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

            {/* Tab panels */}
            <div className="p-6">
              {activeTab === 'timeline' && <TimelineView disputeId={disputeId} />}

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

      {/* Dialogs */}
      {showEscalateDialog && (
        <EscalateDialog
          onConfirm={handleEscalate}
          onCancel={() => setShowEscalateDialog(false)}
          isSubmitting={escalateDispute.isPending}
        />
      )}

      {showAssignDialog && (
        <AssignMediatorDialog
          onConfirm={handleAssignMediator}
          onCancel={() => setShowAssignDialog(false)}
          isSubmitting={assignMediator.isPending}
        />
      )}
    </div>
  );
}
