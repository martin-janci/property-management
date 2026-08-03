/**
 * DisputeDetailPage - displays full dispute details with timeline and actions.
 * Epic 77: Dispute Resolution (Story 77.1, 77.3, 77.4)
 */

import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Spinner } from '../../../components/Spinner';
import { type ActionItem, ActionItemCard } from '../components/ActionItemCard';
import type { DisputeCategory, DisputePriority, DisputeStatus } from '../components/DisputeCard';
import { categoryLabels, priorityLabels, statusLabels } from '../components/DisputeCard';
import { DisputeTimeline, type TimelineEntry } from '../components/DisputeTimeline';
import { type Resolution, ResolutionCard, type ResolutionVote } from '../components/ResolutionCard';

export interface DisputeParty {
  id: string;
  userId: string;
  userName: string;
  userEmail: string;
  role: 'complainant' | 'respondent' | 'witness' | 'mediator';
  notifiedAt?: string;
  respondedAt?: string;
}

export interface DisputeEvidence {
  id: string;
  uploadedBy: string;
  uploaderName: string;
  filename: string;
  originalFilename: string;
  contentType: string;
  sizeBytes: number;
  storageUrl: string;
  description?: string;
  createdAt: string;
}

export interface DisputeDetail {
  id: string;
  organizationId: string;
  buildingId?: string;
  unitId?: string;
  referenceNumber: string;
  category: DisputeCategory;
  title: string;
  description: string;
  desiredResolution?: string;
  status: DisputeStatus;
  priority: DisputePriority;
  filedBy: string;
  filedByName: string;
  assignedTo?: string;
  assignedToName?: string;
  createdAt: string;
  updatedAt: string;
  // Joined fields
  buildingName?: string;
  unitDesignation?: string;
}

interface DisputeDetailPageProps {
  dispute: DisputeDetail;
  parties: DisputeParty[];
  evidence: DisputeEvidence[];
  timeline: TimelineEntry[];
  resolutions: Array<Resolution & { votes: ResolutionVote[]; acceptanceRate: number }>;
  actionItems: ActionItem[];
  isManager?: boolean;
  isParty?: boolean;
  isMediator?: boolean;
  currentUserId?: string;
  isLoading?: boolean;
  onBack: () => void;
  onUpdateStatus: (status: DisputeStatus, reason?: string) => void;
  onAddEvidence: (file: File, description?: string) => void;
  onDeleteEvidence: (id: string) => void;
  onProposeResolution: (
    text: string,
    terms: Array<{ description: string; deadline?: string; responsiblePartyId?: string }>
  ) => void;
  onVoteResolution: (resolutionId: string, accepted: boolean, comments?: string) => void;
  onAcceptResolution: (resolutionId: string) => void;
  onImplementResolution: (resolutionId: string) => void;
  onCompleteResolutionTerm: (resolutionId: string, termId: string) => void;
  onCreateAction: (data: {
    title: string;
    description: string;
    assignedTo: string;
    dueDate: string;
  }) => void;
  onCompleteAction: (actionId: string, notes?: string) => void;
  onSendReminder: (actionId: string) => void;
  onEscalate: (actionId: string) => void;
  onNavigateToMediation: () => void;
}

const statusColors: Record<DisputeStatus, string> = {
  filed: 'bg-blue-100 text-blue-800',
  under_review: 'bg-yellow-100 text-yellow-800',
  mediation: 'bg-purple-100 text-purple-800',
  awaiting_response: 'bg-orange-100 text-orange-800',
  resolved: 'bg-green-100 text-green-800',
  escalated: 'bg-red-100 text-red-800',
  withdrawn: 'bg-gray-100 text-gray-600',
  closed: 'bg-gray-100 text-gray-800',
};

const priorityColors: Record<DisputePriority, string> = {
  low: 'text-gray-500',
  medium: 'text-blue-500',
  high: 'text-orange-500',
  urgent: 'text-red-600 font-bold',
};

const roleLabels: Record<string, string> = {
  complainant: 'Complainant',
  respondent: 'Respondent',
  witness: 'Witness',
  mediator: 'Mediator',
};

export function DisputeDetailPage({
  dispute,
  parties,
  evidence,
  timeline,
  resolutions,
  actionItems,
  isManager = false,
  isParty = false,
  isMediator = false,
  currentUserId,
  isLoading,
  onBack,
  onUpdateStatus,
  onAddEvidence,
  onDeleteEvidence,
  onProposeResolution,
  onVoteResolution,
  onAcceptResolution,
  onImplementResolution,
  onCompleteResolutionTerm,
  onCreateAction,
  onCompleteAction,
  onSendReminder,
  onEscalate,
  onNavigateToMediation,
}: DisputeDetailPageProps) {
  const { t } = useTranslation();
  const [showProposeDialog, setShowProposeDialog] = useState(false);
  const [showActionDialog, setShowActionDialog] = useState(false);
  const [proposalText, setProposalText] = useState('');
  const [newActionTitle, setNewActionTitle] = useState('');
  const [newActionDescription, setNewActionDescription] = useState('');
  const [newActionAssignee, setNewActionAssignee] = useState('');
  const [newActionDueDate, setNewActionDueDate] = useState('');

  const canManage = isManager || isMediator;
  const canPropose = canManage || isParty;
  const activeStatus = ['filed', 'under_review', 'mediation', 'awaiting_response'].includes(
    dispute.status
  );
  const pendingActions = actionItems.filter((a) => a.status !== 'completed');
  const overdueActions = actionItems.filter((a) => a.status === 'overdue');

  const handleFileUpload = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (file) {
      onAddEvidence(file);
    }
  };

  const handleProposeResolution = () => {
    if (proposalText.trim()) {
      onProposeResolution(proposalText, []);
      setProposalText('');
      setShowProposeDialog(false);
    }
  };

  const handleCreateAction = () => {
    if (newActionTitle.trim() && newActionAssignee && newActionDueDate) {
      onCreateAction({
        title: newActionTitle,
        description: newActionDescription,
        assignedTo: newActionAssignee,
        dueDate: newActionDueDate,
      });
      setNewActionTitle('');
      setNewActionDescription('');
      setNewActionAssignee('');
      setNewActionDueDate('');
      setShowActionDialog(false);
    }
  };

  if (isLoading) {
    return (
      <div className="flex justify-center py-12">
        <Spinner size="lg" color="blue" />
      </div>
    );
  }

  return (
    <div className="max-w-4xl mx-auto px-4 py-8">
      {/* Header */}
      <div className="mb-6">
        <button
          type="button"
          onClick={onBack}
          className="text-sm text-blue-600 hover:text-blue-800 flex items-center gap-1 mb-4"
        >
          Back to Disputes
        </button>
        <div className="flex items-start justify-between">
          <div>
            <div className="flex items-center gap-2 mb-2">
              <span className="text-sm font-mono text-gray-500">{dispute.referenceNumber}</span>
              <span
                className={`px-2 py-1 text-sm font-medium rounded ${statusColors[dispute.status]}`}
              >
                {statusLabels[dispute.status]}
              </span>
            </div>
            <h1 className="text-2xl font-bold text-gray-900">{dispute.title}</h1>
            <div className="mt-2 flex items-center gap-4 text-sm">
              <span className="text-gray-500">{categoryLabels[dispute.category]}</span>
              <span className={priorityColors[dispute.priority]}>
                {priorityLabels[dispute.priority]} Priority
              </span>
            </div>
          </div>
          <div className="flex gap-2">
            {activeStatus && canManage && (
              <>
                <button
                  type="button"
                  onClick={onNavigateToMediation}
                  className="px-3 py-1.5 text-sm border border-gray-300 rounded-lg hover:bg-gray-50"
                >
                  Mediation
                </button>
                {dispute.status !== 'resolved' && (
                  <button
                    type="button"
                    onClick={() => onUpdateStatus('resolved')}
                    className="px-3 py-1.5 text-sm bg-green-600 text-white rounded-lg hover:bg-green-700"
                  >
                    Resolve
                  </button>
                )}
              </>
            )}
          </div>
        </div>
      </div>

      {/* Stats Bar */}
      {pendingActions.length > 0 && (
        <div
          className={`mb-6 p-4 rounded-lg ${overdueActions.length > 0 ? 'bg-red-50 border border-red-200' : 'bg-yellow-50 border border-yellow-200'}`}
        >
          <p className={overdueActions.length > 0 ? 'text-red-800' : 'text-yellow-800'}>
            {overdueActions.length > 0 ? (
              <span className="font-medium">{overdueActions.length} overdue action(s)</span>
            ) : (
              <span>{pendingActions.length} pending action(s)</span>
            )}
          </p>
        </div>
      )}

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Main Content */}
        <div className="lg:col-span-2 space-y-6">
          {/* Description */}
          <div className="bg-white rounded-lg shadow p-6">
            <h2 className="text-lg font-semibold text-gray-900 mb-3">Description</h2>
            <p className="text-gray-700 whitespace-pre-wrap">{dispute.description}</p>
            {dispute.desiredResolution && (
              <div className="mt-4 pt-4 border-t">
                <h3 className="text-sm font-medium text-gray-700 mb-2">Desired Resolution</h3>
                <p className="text-gray-600">{dispute.desiredResolution}</p>
              </div>
            )}
          </div>

          {/* Evidence */}
          <div className="bg-white rounded-lg shadow p-6">
            <div className="flex items-center justify-between mb-4">
              <h2 className="text-lg font-semibold text-gray-900">Evidence ({evidence.length})</h2>
              {(isParty || canManage) && (
                <label className="px-3 py-1.5 text-sm border border-gray-300 rounded-lg hover:bg-gray-50 cursor-pointer">
                  Add Evidence
                  <input type="file" className="hidden" onChange={handleFileUpload} />
                </label>
              )}
            </div>
            {evidence.length === 0 ? (
              <p className="text-gray-500">No evidence submitted.</p>
            ) : (
              <div className="space-y-3">
                {evidence.map((item) => (
                  <div
                    key={item.id}
                    className="flex items-center justify-between p-3 bg-gray-50 rounded-lg"
                  >
                    <div>
                      <p className="font-medium">{item.originalFilename}</p>
                      <p className="text-sm text-gray-500">
                        Uploaded by {item.uploaderName} -{' '}
                        {new Date(item.createdAt).toLocaleDateString()}
                      </p>
                      {item.description && (
                        <p className="text-sm text-gray-600 mt-1">{item.description}</p>
                      )}
                    </div>
                    <div className="flex gap-2">
                      <a
                        href={item.storageUrl}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="text-blue-600 hover:text-blue-800 text-sm"
                      >
                        View
                      </a>
                      {canManage && (
                        <button
                          type="button"
                          onClick={() => onDeleteEvidence(item.id)}
                          className="text-red-600 hover:text-red-800 text-sm"
                        >
                          Delete
                        </button>
                      )}
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>

          {/* Resolutions */}
          <div className="bg-white rounded-lg shadow p-6">
            <div className="flex items-center justify-between mb-4">
              <h2 className="text-lg font-semibold text-gray-900">Resolutions</h2>
              {canPropose && activeStatus && (
                <button
                  type="button"
                  onClick={() => setShowProposeDialog(true)}
                  className="px-3 py-1.5 text-sm bg-blue-600 text-white rounded-lg hover:bg-blue-700"
                >
                  Propose Resolution
                </button>
              )}
            </div>
            {resolutions.length === 0 ? (
              <p className="text-gray-500">No resolutions proposed yet.</p>
            ) : (
              <div className="space-y-4">
                {resolutions.map((resolution) => (
                  <ResolutionCard
                    key={resolution.id}
                    resolution={resolution}
                    votes={resolution.votes}
                    acceptanceRate={resolution.acceptanceRate}
                    canVote={isParty}
                    canAccept={canManage}
                    canImplement={canManage}
                    onVote={onVoteResolution}
                    onAccept={onAcceptResolution}
                    onImplement={onImplementResolution}
                    onCompleteItem={onCompleteResolutionTerm}
                  />
                ))}
              </div>
            )}
          </div>

          {/* Action Items */}
          <div className="bg-white rounded-lg shadow p-6">
            <div className="flex items-center justify-between mb-4">
              <h2 className="text-lg font-semibold text-gray-900">Action Items</h2>
              {canManage && (
                <button
                  type="button"
                  onClick={() => setShowActionDialog(true)}
                  className="px-3 py-1.5 text-sm bg-blue-600 text-white rounded-lg hover:bg-blue-700"
                >
                  Create Action
                </button>
              )}
            </div>
            {actionItems.length === 0 ? (
              <p className="text-gray-500">No action items.</p>
            ) : (
              <div className="space-y-4">
                {actionItems.map((action) => (
                  <ActionItemCard
                    key={action.id}
                    action={action}
                    isAssignee={action.assignedTo === currentUserId}
                    isManager={canManage}
                    onComplete={onCompleteAction}
                    onSendReminder={onSendReminder}
                    onEscalate={onEscalate}
                  />
                ))}
              </div>
            )}
          </div>

          {/* Timeline */}
          <div className="bg-white rounded-lg shadow p-6">
            <DisputeTimeline entries={timeline} maxEntries={10} />
          </div>
        </div>

        {/* Sidebar */}
        <div className="space-y-6">
          {/* Details */}
          <div className="bg-white rounded-lg shadow p-6">
            <h3 className="text-lg font-semibold text-gray-900 mb-4">Details</h3>
            <dl className="space-y-3 text-sm">
              {dispute.buildingName && (
                <div>
                  <dt className="text-gray-500">Building</dt>
                  <dd className="font-medium">{dispute.buildingName}</dd>
                </div>
              )}
              {dispute.unitDesignation && (
                <div>
                  <dt className="text-gray-500">Unit</dt>
                  <dd className="font-medium">{dispute.unitDesignation}</dd>
                </div>
              )}
              <div>
                <dt className="text-gray-500">Filed by</dt>
                <dd className="font-medium">{dispute.filedByName}</dd>
              </div>
              <div>
                <dt className="text-gray-500">Filed on</dt>
                <dd className="font-medium">{new Date(dispute.createdAt).toLocaleDateString()}</dd>
              </div>
              {dispute.assignedToName && (
                <div>
                  <dt className="text-gray-500">Assigned to</dt>
                  <dd className="font-medium">{dispute.assignedToName}</dd>
                </div>
              )}
            </dl>
          </div>

          {/* Parties */}
          <div className="bg-white rounded-lg shadow p-6">
            <h3 className="text-lg font-semibold text-gray-900 mb-4">Parties</h3>
            <div className="space-y-3">
              {parties.map((party) => (
                <div key={party.id} className="p-3 bg-gray-50 rounded-lg">
                  <div className="flex items-center justify-between">
                    <span className="font-medium">{party.userName}</span>
                    <span className="text-xs px-2 py-0.5 bg-gray-200 rounded">
                      {roleLabels[party.role]}
                    </span>
                  </div>
                  <p className="text-sm text-gray-500">{party.userEmail}</p>
                </div>
              ))}
            </div>
          </div>
        </div>
      </div>

      {/* Propose Resolution Dialog */}
      {showProposeDialog && (
        <div className="fixed inset-0 z-50 overflow-y-auto">
          <button
            type="button"
            className="fixed inset-0 bg-black bg-opacity-50 cursor-default"
            onClick={() => setShowProposeDialog(false)}
            onKeyDown={(e) => e.key === 'Escape' && setShowProposeDialog(false)}
            aria-label={t('aria.closeDialog')}
          />
          <div className="flex min-h-full items-center justify-center p-4">
            <div className="relative w-full max-w-md bg-white rounded-lg shadow-xl p-6">
              <h2 className="text-lg font-semibold mb-4">Propose Resolution</h2>
              <textarea
                value={proposalText}
                onChange={(e) => setProposalText(e.target.value)}
                rows={6}
                placeholder="Describe your proposed resolution..."
                className="w-full rounded-md border border-gray-300 px-3 py-2"
              />
              <div className="flex justify-end gap-3 mt-4">
                <button
                  type="button"
                  onClick={() => setShowProposeDialog(false)}
                  className="px-4 py-2 border rounded-lg"
                >
                  Cancel
                </button>
                <button
                  type="button"
                  onClick={handleProposeResolution}
                  disabled={!proposalText.trim()}
                  className="px-4 py-2 bg-blue-600 text-white rounded-lg disabled:opacity-50"
                >
                  Propose
                </button>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Create Action Dialog */}
      {showActionDialog && (
        <div className="fixed inset-0 z-50 overflow-y-auto">
          <button
            type="button"
            className="fixed inset-0 bg-black bg-opacity-50 cursor-default"
            onClick={() => setShowActionDialog(false)}
            onKeyDown={(e) => e.key === 'Escape' && setShowActionDialog(false)}
            aria-label={t('aria.closeDialog')}
          />
          <div className="flex min-h-full items-center justify-center p-4">
            <div className="relative w-full max-w-md bg-white rounded-lg shadow-xl p-6">
              <h2 className="text-lg font-semibold mb-4">Create Action Item</h2>
              <div className="space-y-4">
                <div>
                  <label className="block text-sm font-medium mb-1">Title</label>
                  <input
                    type="text"
                    value={newActionTitle}
                    onChange={(e) => setNewActionTitle(e.target.value)}
                    className="w-full rounded-md border border-gray-300 px-3 py-2"
                  />
                </div>
                <div>
                  <label className="block text-sm font-medium mb-1">Description</label>
                  <textarea
                    value={newActionDescription}
                    onChange={(e) => setNewActionDescription(e.target.value)}
                    rows={3}
                    className="w-full rounded-md border border-gray-300 px-3 py-2"
                  />
                </div>
                <div>
                  <label className="block text-sm font-medium mb-1">Assign to</label>
                  <select
                    value={newActionAssignee}
                    onChange={(e) => setNewActionAssignee(e.target.value)}
                    className="w-full rounded-md border border-gray-300 px-3 py-2"
                  >
                    <option value="">Select party...</option>
                    {parties.map((party) => (
                      <option key={party.id} value={party.userId}>
                        {party.userName}
                      </option>
                    ))}
                  </select>
                </div>
                <div>
                  <label className="block text-sm font-medium mb-1">Due Date</label>
                  <input
                    type="date"
                    value={newActionDueDate}
                    onChange={(e) => setNewActionDueDate(e.target.value)}
                    className="w-full rounded-md border border-gray-300 px-3 py-2"
                  />
                </div>
              </div>
              <div className="flex justify-end gap-3 mt-4">
                <button
                  type="button"
                  onClick={() => setShowActionDialog(false)}
                  className="px-4 py-2 border rounded-lg"
                >
                  Cancel
                </button>
                <button
                  type="button"
                  onClick={handleCreateAction}
                  disabled={!newActionTitle.trim() || !newActionAssignee || !newActionDueDate}
                  className="px-4 py-2 bg-blue-600 text-white rounded-lg disabled:opacity-50"
                >
                  Create
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
