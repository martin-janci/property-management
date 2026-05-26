/**
 * MediationWorkspacePage — mediation workspace for dispute resolution.
 * Epic 80: Dispute Mediation (Story 80.3)
 *
 * Three-panel layout:
 *   Left sidebar  — dispute summary card + parties + mediator chip
 *   Main column   — tabbed: "Timeline" | "Chat" | "Resolve"
 *   Right rail    — status + quick-action buttons (Escalate, Assign Mediator)
 *
 * All data comes from @ppt/api-client hooks; no prop drilling of raw data.
 */

import {
  type ResolveDisputeRequest,
  useAssignMediator,
  useDispute,
  useDisputeTimeline,
  useEscalateDispute,
  useResolveDispute,
} from '@ppt/api-client';
import { useState } from 'react';
import { MediationChatThread } from '../components/MediationChatThread';
import { MediationResolutionForm } from '../components/MediationResolutionForm';

// ============================================
// Inline sub-components
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

const statusLabels: Record<string, string> = {
  filed: 'Filed',
  under_review: 'Under Review',
  mediation: 'In Mediation',
  escalated: 'Escalated',
  resolved: 'Resolved',
  closed: 'Closed',
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

function formatTime(dateString: string): string {
  const date = new Date(dateString);
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffMins = Math.floor(diffMs / 60_000);
  const diffHours = Math.floor(diffMins / 60);
  const diffDays = Math.floor(diffHours / 24);

  if (diffMins < 1) return 'Just now';
  if (diffMins < 60) return `${diffMins}m ago`;
  if (diffHours < 24) return `${diffHours}h ago`;
  if (diffDays < 7) return `${diffDays}d ago`;
  return date.toLocaleDateString();
}

// ============================================
// DisputeTimeline sub-view
// ============================================

interface TimelineViewProps {
  disputeId: string;
}

function TimelineView({ disputeId }: TimelineViewProps) {
  const { data: events = [], isLoading } = useDisputeTimeline(disputeId);

  if (isLoading) {
    return (
      <div className="flex justify-center py-8">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-violet-600" />
      </div>
    );
  }

  if (events.length === 0) {
    return <p className="text-sm text-gray-500 py-4">No timeline events yet.</p>;
  }

  return (
    <div className="relative">
      {/* Vertical connector line */}
      <div className="absolute left-4 top-0 bottom-0 w-px bg-gray-200" />

      <div className="space-y-4">
        {events.map((event) => {
          const eventType = event.eventType as TimelineEventType;
          const icon = eventIcons[eventType] ?? '·';
          const color = eventColors[eventType] ?? 'bg-gray-100 text-gray-600';

          return (
            <div key={event.id} className="relative flex items-start gap-4 pl-10">
              {/* Icon dot */}
              <div
                className={`absolute left-0 w-8 h-8 rounded-full flex items-center justify-center text-xs font-bold ${color}`}
              >
                {icon}
              </div>

              {/* Content card */}
              <div className="flex-1 bg-white rounded-lg border border-gray-200 px-4 py-3">
                <div className="flex items-center justify-between">
                  <span className="text-sm font-medium text-gray-900">{event.actorName}</span>
                  <span className="text-xs text-gray-400">{formatTime(event.createdAt)}</span>
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
// Escalate dialog
// ============================================

interface EscalateDialogProps {
  onConfirm: (reason: string) => void;
  onCancel: () => void;
  isSubmitting: boolean;
}

function EscalateDialog({ onConfirm, onCancel, isSubmitting }: EscalateDialogProps) {
  const [reason, setReason] = useState('');

  return (
    <div className="fixed inset-0 z-50 overflow-y-auto">
      <button
        type="button"
        className="fixed inset-0 bg-black bg-opacity-50 cursor-default"
        onClick={onCancel}
        onKeyDown={(e) => e.key === 'Escape' && onCancel()}
        aria-label="Close dialog"
      />
      <div className="flex min-h-full items-center justify-center p-4">
        <div className="relative w-full max-w-md bg-white rounded-xl shadow-xl p-6">
          <h2 className="text-lg font-semibold text-gray-900 mb-4">Escalate Dispute</h2>
          <p className="text-sm text-gray-600 mb-4">
            Escalation moves this dispute to a higher authority. Please provide a reason.
          </p>
          <label className="block text-sm font-medium text-gray-700 mb-1">
            Reason for escalation <span className="text-red-500">*</span>
          </label>
          <textarea
            value={reason}
            onChange={(e) => setReason(e.target.value)}
            rows={4}
            placeholder="Describe why this dispute needs to be escalated…"
            className="w-full rounded-lg border border-gray-300 px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-red-500"
          />
          <div className="flex justify-end gap-3 mt-5">
            <button
              type="button"
              onClick={onCancel}
              className="px-4 py-2 text-sm border border-gray-300 rounded-lg hover:bg-gray-50"
            >
              Cancel
            </button>
            <button
              type="button"
              onClick={() => onConfirm(reason)}
              disabled={!reason.trim() || isSubmitting}
              className="px-4 py-2 text-sm bg-red-600 text-white rounded-lg hover:bg-red-700 disabled:opacity-50 font-medium"
            >
              {isSubmitting ? 'Escalating…' : 'Escalate'}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

// ============================================
// Assign mediator dialog
// ============================================

interface AssignMediatorDialogProps {
  onConfirm: (mediatorId: string) => void;
  onCancel: () => void;
  isSubmitting: boolean;
}

function AssignMediatorDialog({ onConfirm, onCancel, isSubmitting }: AssignMediatorDialogProps) {
  const [mediatorId, setMediatorId] = useState('');

  return (
    <div className="fixed inset-0 z-50 overflow-y-auto">
      <button
        type="button"
        className="fixed inset-0 bg-black bg-opacity-50 cursor-default"
        onClick={onCancel}
        onKeyDown={(e) => e.key === 'Escape' && onCancel()}
        aria-label="Close dialog"
      />
      <div className="flex min-h-full items-center justify-center p-4">
        <div className="relative w-full max-w-md bg-white rounded-xl shadow-xl p-6">
          <h2 className="text-lg font-semibold text-gray-900 mb-4">Assign Mediator</h2>
          <p className="text-sm text-gray-600 mb-4">
            Enter the user ID of the staff member to assign as mediator.
          </p>
          <label className="block text-sm font-medium text-gray-700 mb-1">
            Mediator user ID <span className="text-red-500">*</span>
          </label>
          <input
            type="text"
            value={mediatorId}
            onChange={(e) => setMediatorId(e.target.value)}
            placeholder="user_xxxxxxxxxxxx"
            className="w-full rounded-lg border border-gray-300 px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-violet-500"
          />
          <p className="mt-1 text-xs text-gray-400">
            A user-picker will be wired once the staff directory endpoint is available.
          </p>
          <div className="flex justify-end gap-3 mt-5">
            <button
              type="button"
              onClick={onCancel}
              className="px-4 py-2 text-sm border border-gray-300 rounded-lg hover:bg-gray-50"
            >
              Cancel
            </button>
            <button
              type="button"
              onClick={() => onConfirm(mediatorId)}
              disabled={!mediatorId.trim() || isSubmitting}
              className="px-4 py-2 text-sm bg-violet-600 text-white rounded-lg hover:bg-violet-700 disabled:opacity-50 font-medium"
            >
              {isSubmitting ? 'Assigning…' : 'Assign'}
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
  const [activeTab, setActiveTab] = useState<Tab>('timeline');
  const [showEscalateDialog, setShowEscalateDialog] = useState(false);
  const [showAssignDialog, setShowAssignDialog] = useState(false);

  // Data queries
  const { data: dispute, isLoading: disputeLoading } = useDispute(disputeId);

  // Mutations
  const resolveDispute = useResolveDispute(organizationId);
  const escalateDispute = useEscalateDispute(organizationId);
  const assignMediator = useAssignMediator();
  // Note: useMediationNotes + useAddMediationNote are consumed inside MediationChatThread

  const isMediator = !!dispute && dispute.assignedMediatorId === currentUserId;
  const isParty =
    !!dispute && (dispute.filedBy === currentUserId || dispute.respondentId === currentUserId);
  const canManage = isManager || isMediator;
  const canChat = canManage || isParty;
  const isActive = dispute?.status !== 'resolved' && dispute?.status !== 'closed';

  const handleResolve = async (data: ResolveDisputeRequest) => {
    try {
      await resolveDispute.mutateAsync({ disputeId, data });
      onToastSuccess('Dispute resolved', 'The dispute has been marked as resolved.');
      setActiveTab('timeline');
    } catch (err) {
      onToastError(
        'Failed to resolve dispute',
        err instanceof Error ? err.message : 'Unexpected error'
      );
    }
  };

  const handleEscalate = async (reason: string) => {
    try {
      await escalateDispute.mutateAsync({ disputeId, data: { reason } });
      onToastSuccess('Dispute escalated', 'The dispute has been escalated.');
      setShowEscalateDialog(false);
    } catch (err) {
      onToastError(
        'Failed to escalate dispute',
        err instanceof Error ? err.message : 'Unexpected error'
      );
    }
  };

  const handleAssignMediator = async (mediatorId: string) => {
    try {
      await assignMediator.mutateAsync({ disputeId, data: { mediatorId } });
      onToastSuccess('Mediator assigned', 'The mediator has been assigned to this dispute.');
      setShowAssignDialog(false);
    } catch (err) {
      onToastError(
        'Failed to assign mediator',
        err instanceof Error ? err.message : 'Unexpected error'
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
  const disputeTitle = dispute?.subject ?? 'Dispute';

  return (
    <div className="max-w-5xl mx-auto px-4 py-8">
      {/* Header */}
      <div className="mb-6">
        <button
          type="button"
          onClick={onBack}
          className="text-sm text-violet-600 hover:text-violet-800 mb-4 flex items-center gap-1"
        >
          ← Back to Dispute
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
                {statusLabels[disputeStatus] ?? disputeStatus}
              </span>
            </div>
            <h1 className="text-2xl font-bold text-gray-900">Mediation Workspace</h1>
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
                Assign Mediator
              </button>
              <button
                type="button"
                onClick={() => setShowEscalateDialog(true)}
                className="px-4 py-2 text-sm border border-red-300 text-red-600 rounded-lg hover:bg-red-50"
              >
                Escalate
              </button>
              <button
                type="button"
                onClick={() => setActiveTab('resolve')}
                className="px-4 py-2 text-sm bg-green-600 text-white rounded-lg hover:bg-green-700 font-medium"
              >
                Resolve Dispute
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
            <h3 className="text-sm font-semibold text-gray-700 mb-3">Dispute</h3>
            <dl className="space-y-2 text-sm">
              <div>
                <dt className="text-xs text-gray-400 uppercase tracking-wide">Filed by</dt>
                <dd className="font-medium text-gray-900">
                  {dispute?.filerDetails?.name ?? dispute?.filedBy ?? '—'}
                </dd>
              </div>
              {dispute?.respondent && (
                <div>
                  <dt className="text-xs text-gray-400 uppercase tracking-wide">Respondent</dt>
                  <dd className="font-medium text-gray-900">{dispute.respondent}</dd>
                </div>
              )}
              {dispute?.assignedMediator && (
                <div>
                  <dt className="text-xs text-gray-400 uppercase tracking-wide">Mediator</dt>
                  <dd className="font-medium text-violet-700">{dispute.assignedMediator}</dd>
                </div>
              )}
              <div>
                <dt className="text-xs text-gray-400 uppercase tracking-wide">Filed</dt>
                <dd className="text-gray-600">
                  {dispute ? new Date(dispute.filedAt).toLocaleDateString() : '—'}
                </dd>
              </div>
              {dispute?.resolutionDeadline && (
                <div>
                  <dt className="text-xs text-gray-400 uppercase tracking-wide">Deadline</dt>
                  <dd className="text-orange-600 font-medium">
                    {new Date(dispute.resolutionDeadline).toLocaleDateString()}
                  </dd>
                </div>
              )}
            </dl>
          </div>

          {/* Parties */}
          <div className="bg-white rounded-xl shadow-sm border border-gray-100 p-4">
            <h3 className="text-sm font-semibold text-gray-700 mb-3">Parties</h3>
            <div className="space-y-2">
              {dispute?.filerDetails && (
                <div className="p-2.5 bg-blue-50 rounded-lg">
                  <p className="text-sm font-medium text-gray-900">{dispute.filerDetails.name}</p>
                  <p className="text-xs text-blue-600 mt-0.5">Filer</p>
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
                  <p className="text-xs text-gray-500 mt-0.5">Respondent</p>
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
                  <p className="text-xs text-violet-600 mt-0.5">Mediator</p>
                </div>
              )}
              {!dispute?.filerDetails && !dispute?.respondentDetails && (
                <p className="text-xs text-gray-400">Party details unavailable.</p>
              )}
            </div>
          </div>

          {/* Mediation guidelines */}
          <div className="bg-violet-50 rounded-xl border border-violet-100 p-4">
            <h3 className="text-sm font-semibold text-violet-900 mb-2">Guidelines</h3>
            <ul className="text-xs text-violet-800 space-y-1.5 list-disc list-inside">
              <li>Be respectful and constructive</li>
              <li>Focus on interests, not positions</li>
              <li>Discussions are confidential</li>
              <li>Work towards a shared solution</li>
            </ul>
          </div>
        </div>

        {/* Main content */}
        <div className="lg:col-span-3">
          <div className="bg-white rounded-xl shadow-sm border border-gray-100">
            {/* Tabs */}
            <div className="border-b border-gray-200 px-6">
              <nav className="flex gap-6" aria-label="Workspace tabs">
                {(
                  [
                    { id: 'timeline', label: 'Timeline' },
                    { id: 'chat', label: 'Discussion' },
                    { id: 'resolve', label: 'Resolution' },
                  ] as { id: Tab; label: string }[]
                ).map((tab) => (
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
                    />
                  ) : (
                    <p className="text-sm text-gray-500 text-center py-8">
                      You must be a party or manager to participate in this discussion.
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
                        {statusLabels[disputeStatus] ?? disputeStatus}
                      </span>
                      <p className="text-gray-500 text-sm mt-3">
                        This dispute is no longer active.
                      </p>
                    </div>
                  ) : canManage ? (
                    <>
                      <h2 className="text-base font-semibold text-gray-900 mb-4">
                        Record Resolution
                      </h2>
                      <MediationResolutionForm
                        disputeId={disputeId}
                        isSubmitting={resolveDispute.isPending}
                        onResolve={handleResolve}
                        onCancel={() => setActiveTab('timeline')}
                      />
                    </>
                  ) : (
                    <p className="text-sm text-gray-500 text-center py-8">
                      Only managers and mediators can record a resolution.
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
