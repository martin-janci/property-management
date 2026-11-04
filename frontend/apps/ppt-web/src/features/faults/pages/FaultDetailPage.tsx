/**
 * FaultDetailPage - displays full fault details with timeline.
 * Epic 4: Fault Reporting & Resolution (UC-03.5, UC-03.6, UC-03.7, UC-03.10, UC-03.11)
 */

import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { FaultCategory, FaultPriority, FaultStatus } from '../components/FaultCard';
import { FaultTimeline, type TimelineEntry } from '../components/FaultTimeline';
import { type TriageData, TriageFaultDialog } from '../components/TriageFaultDialog';

export interface FaultDetail {
  id: string;
  organizationId: string;
  buildingId: string;
  unitId?: string;
  reporterId: string;
  title: string;
  description: string;
  locationDescription?: string;
  category: FaultCategory;
  priority: FaultPriority;
  status: FaultStatus;
  aiCategory?: string;
  aiPriority?: string;
  aiConfidence?: number;
  assignedTo?: string;
  assignedAt?: string;
  triagedBy?: string;
  triagedAt?: string;
  resolvedAt?: string;
  resolvedBy?: string;
  resolutionNotes?: string;
  confirmedAt?: string;
  confirmedBy?: string;
  rating?: number;
  feedback?: string;
  scheduledDate?: string;
  estimatedCompletion?: string;
  createdAt: string;
  updatedAt: string;
  // Joined fields
  reporterName: string;
  reporterEmail: string;
  buildingName: string;
  buildingAddress: string;
  unitDesignation?: string;
  assignedToName?: string;
  attachmentCount: number;
  commentCount: number;
}

export interface FaultAttachment {
  id: string;
  faultId: string;
  filename: string;
  originalFilename: string;
  contentType: string;
  sizeBytes: number;
  storageUrl: string;
  thumbnailUrl?: string;
  description?: string;
  createdAt: string;
}

interface FaultDetailPageProps {
  fault: FaultDetail;
  timeline: TimelineEntry[];
  attachments: FaultAttachment[];
  technicians?: Array<{ id: string; name: string }>;
  isManager?: boolean;
  isLoading?: boolean;
  onBack: () => void;
  onEdit: () => void;
  onTriage: (data: TriageData) => void;
  onResolve: (notes: string) => void;
  onConfirm: (rating?: number, feedback?: string) => void;
  onReopen: (reason: string) => void;
  onAddComment: (note: string, isInternal: boolean) => void;
  /** Omit to hide the upload control (no client-side storage upload flow yet). */
  onAddAttachment?: (file: File) => void;
  onDeleteAttachment: (id: string) => void;
}

const statusColors: Record<FaultStatus, string> = {
  new: 'bg-red-100 text-red-800',
  triaged: 'bg-blue-100 text-blue-800',
  in_progress: 'bg-yellow-100 text-yellow-800',
  waiting_parts: 'bg-orange-100 text-orange-800',
  scheduled: 'bg-purple-100 text-purple-800',
  resolved: 'bg-green-100 text-green-800',
  closed: 'bg-gray-100 text-gray-800',
  reopened: 'bg-red-100 text-red-800',
};

const priorityColors: Record<FaultPriority, string> = {
  low: 'text-gray-500',
  medium: 'text-blue-500',
  high: 'text-orange-500',
  urgent: 'text-red-600 font-bold',
};

export function FaultDetailPage({
  fault,
  timeline,
  attachments,
  technicians,
  isManager = false,
  isLoading,
  onBack,
  onEdit,
  onTriage,
  onResolve,
  onConfirm,
  onReopen,
  onAddComment,
  onAddAttachment,
  onDeleteAttachment,
}: FaultDetailPageProps) {
  const { t } = useTranslation();
  const [showTriageDialog, setShowTriageDialog] = useState(false);
  const [showResolveDialog, setShowResolveDialog] = useState(false);
  const [showConfirmDialog, setShowConfirmDialog] = useState(false);
  const [showReopenDialog, setShowReopenDialog] = useState(false);
  const [comment, setComment] = useState('');
  const [isInternalComment, setIsInternalComment] = useState(false);
  const [resolutionNotes, setResolutionNotes] = useState('');
  const [confirmRating, setConfirmRating] = useState<number | undefined>();
  const [confirmFeedback, setConfirmFeedback] = useState('');
  const [reopenReason, setReopenReason] = useState('');

  const statusLabels: Record<FaultStatus, string> = {
    new: t('faults.statusNew'),
    triaged: t('faults.statusTriaged'),
    in_progress: t('faults.statusInProgress'),
    waiting_parts: t('faults.statusWaitingParts'),
    scheduled: t('faults.statusScheduled'),
    resolved: t('faults.statusResolved'),
    closed: t('faults.statusClosed'),
    reopened: t('faults.statusReopened'),
  };

  const priorityLabels: Record<FaultPriority, string> = {
    low: t('faults.priorityLow'),
    medium: t('faults.priorityMedium'),
    high: t('faults.priorityHigh'),
    urgent: t('faults.priorityUrgent'),
  };

  const categoryLabels: Record<FaultCategory, string> = {
    plumbing: t('faults.categoryPlumbing'),
    electrical: t('faults.categoryElectrical'),
    heating: t('faults.categoryHeating'),
    structural: t('faults.categoryStructural'),
    exterior: t('faults.categoryExterior'),
    elevator: t('faults.categoryElevator'),
    common_area: t('faults.categoryCommonArea'),
    security: t('faults.categorySecurity'),
    cleaning: t('faults.categoryCleaning'),
    other: t('faults.categoryOther'),
  };

  const canEdit = fault.status === 'new';
  const canTriage = fault.status === 'new' && isManager;
  const canResolve =
    isManager &&
    ['triaged', 'in_progress', 'waiting_parts', 'scheduled', 'reopened'].includes(fault.status);
  const canConfirm = fault.status === 'resolved';
  const canReopen = fault.status === 'closed';

  const handleAddComment = (e: React.FormEvent) => {
    e.preventDefault();
    if (comment.trim()) {
      onAddComment(comment, isInternalComment);
      setComment('');
      setIsInternalComment(false);
    }
  };

  const handleResolve = () => {
    onResolve(resolutionNotes);
    setShowResolveDialog(false);
    setResolutionNotes('');
  };

  const handleConfirm = () => {
    onConfirm(confirmRating, confirmFeedback || undefined);
    setShowConfirmDialog(false);
    setConfirmRating(undefined);
    setConfirmFeedback('');
  };

  const handleReopen = () => {
    onReopen(reopenReason);
    setShowReopenDialog(false);
    setReopenReason('');
  };

  const handleFileUpload = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (file) {
      onAddAttachment?.(file);
    }
  };

  if (isLoading) {
    return (
      <div className="flex justify-center py-12">
        <div className="animate-spin rounded-full h-10 w-10 border-b-2 border-blue-600" />
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
          {t('common.backToFaults')}
        </button>
        <div className="flex items-start justify-between">
          <div>
            <h1 className="text-2xl font-bold text-gray-900">{fault.title}</h1>
            <div className="mt-2 flex items-center gap-3">
              <span
                className={`px-2 py-1 text-sm font-medium rounded ${statusColors[fault.status]}`}
              >
                {statusLabels[fault.status]}
              </span>
              <span className={`text-sm font-medium ${priorityColors[fault.priority]}`}>
                {priorityLabels[fault.priority]} {t('faults.priority')}
              </span>
              <span className="text-sm text-gray-500">{categoryLabels[fault.category]}</span>
            </div>
          </div>
          <div className="flex gap-2">
            {canEdit && (
              <button
                type="button"
                onClick={onEdit}
                className="px-3 py-1.5 text-sm border border-gray-300 rounded-lg hover:bg-gray-50"
              >
                {t('common.edit')}
              </button>
            )}
            {canTriage && (
              <button
                type="button"
                onClick={() => setShowTriageDialog(true)}
                className="px-3 py-1.5 text-sm bg-blue-600 text-white rounded-lg hover:bg-blue-700"
              >
                {t('faults.triage')}
              </button>
            )}
            {canResolve && (
              <button
                type="button"
                onClick={() => setShowResolveDialog(true)}
                className="px-3 py-1.5 text-sm bg-green-600 text-white rounded-lg hover:bg-green-700"
              >
                {t('faults.resolve')}
              </button>
            )}
            {canConfirm && (
              <button
                type="button"
                onClick={() => setShowConfirmDialog(true)}
                className="px-3 py-1.5 text-sm bg-green-600 text-white rounded-lg hover:bg-green-700"
              >
                {t('common.confirm')}
              </button>
            )}
            {canReopen && (
              <button
                type="button"
                onClick={() => setShowReopenDialog(true)}
                className="px-3 py-1.5 text-sm bg-orange-600 text-white rounded-lg hover:bg-orange-700"
              >
                {t('faults.reopen')}
              </button>
            )}
          </div>
        </div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Main Content */}
        <div className="lg:col-span-2 space-y-6">
          {/* Description */}
          <div className="bg-white rounded-lg shadow p-6">
            <h2 className="text-lg font-semibold text-gray-900 mb-3">{t('faults.description')}</h2>
            <p className="text-gray-700 whitespace-pre-wrap">{fault.description}</p>
            {fault.locationDescription && (
              <p className="mt-3 text-sm text-gray-500">
                <strong>{t('faults.location')}:</strong> {fault.locationDescription}
              </p>
            )}
          </div>

          {/* Attachments */}
          <div className="bg-white rounded-lg shadow p-6">
            <div className="flex items-center justify-between mb-4">
              <h2 className="text-lg font-semibold text-gray-900">
                {t('faults.attachments')} ({attachments.length})
              </h2>
              {onAddAttachment && (
                <label className="px-3 py-1.5 text-sm border border-gray-300 rounded-lg hover:bg-gray-50 cursor-pointer">
                  {t('faults.addPhoto')}
                  <input
                    type="file"
                    accept="image/*"
                    className="hidden"
                    onChange={handleFileUpload}
                  />
                </label>
              )}
            </div>
            {attachments.length === 0 ? (
              <p className="text-gray-500">{t('faults.noAttachments')}</p>
            ) : (
              <div className="grid grid-cols-2 md:grid-cols-3 gap-4">
                {attachments.map((att) => (
                  <div key={att.id} className="relative group">
                    {att.contentType.startsWith('image/') ? (
                      <img
                        src={att.thumbnailUrl || att.storageUrl}
                        alt={att.originalFilename}
                        className="w-full h-32 object-cover rounded-lg"
                      />
                    ) : (
                      <div className="w-full h-32 bg-gray-100 rounded-lg flex items-center justify-center">
                        {att.originalFilename}
                      </div>
                    )}
                    <button
                      type="button"
                      onClick={() => onDeleteAttachment(att.id)}
                      className="absolute top-1 right-1 p-1 bg-red-500 text-white rounded-full opacity-0 group-hover:opacity-100 transition-opacity"
                      title={t('common.delete')}
                    >
                      X
                    </button>
                  </div>
                ))}
              </div>
            )}
          </div>

          {/* Timeline */}
          <div className="bg-white rounded-lg shadow p-6">
            <FaultTimeline entries={timeline} />
          </div>

          {/* Add Comment */}
          <div className="bg-white rounded-lg shadow p-6">
            <h3 className="text-lg font-semibold text-gray-900 mb-4">{t('faults.addComment')}</h3>
            <form onSubmit={handleAddComment}>
              <textarea
                value={comment}
                onChange={(e) => setComment(e.target.value)}
                rows={3}
                placeholder={t('faults.commentPlaceholder')}
                className="w-full rounded-md border border-gray-300 px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500"
              />
              {isManager && (
                <label className="flex items-center gap-2 mt-2">
                  <input
                    type="checkbox"
                    checked={isInternalComment}
                    onChange={(e) => setIsInternalComment(e.target.checked)}
                  />
                  <span className="text-sm text-gray-600">{t('faults.internalNote')}</span>
                </label>
              )}
              <button
                type="submit"
                disabled={!comment.trim()}
                className="mt-3 px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50"
              >
                {t('faults.addComment')}
              </button>
            </form>
          </div>
        </div>

        {/* Sidebar */}
        <div className="space-y-6">
          {/* Details */}
          <div className="bg-white rounded-lg shadow p-6">
            <h3 className="text-lg font-semibold text-gray-900 mb-4">{t('faults.details')}</h3>
            <dl className="space-y-3 text-sm">
              <div>
                <dt className="text-gray-500">{t('buildings.title')}</dt>
                <dd className="font-medium">{fault.buildingName}</dd>
                <dd className="text-gray-500">{fault.buildingAddress}</dd>
              </div>
              {fault.unitDesignation && (
                <div>
                  <dt className="text-gray-500">{t('faults.unit')}</dt>
                  <dd className="font-medium">{fault.unitDesignation}</dd>
                </div>
              )}
              <div>
                <dt className="text-gray-500">{t('faults.reportedBy')}</dt>
                <dd className="font-medium">{fault.reporterName}</dd>
                <dd className="text-gray-500">{fault.reporterEmail}</dd>
              </div>
              <div>
                <dt className="text-gray-500">{t('faults.reportedOn')}</dt>
                <dd className="font-medium">{new Date(fault.createdAt).toLocaleString()}</dd>
              </div>
              {fault.assignedToName && (
                <div>
                  <dt className="text-gray-500">{t('faults.assignedTo')}</dt>
                  <dd className="font-medium">{fault.assignedToName}</dd>
                </div>
              )}
              {fault.scheduledDate && (
                <div>
                  <dt className="text-gray-500">{t('faults.scheduled')}</dt>
                  <dd className="font-medium">
                    {new Date(fault.scheduledDate).toLocaleDateString()}
                  </dd>
                </div>
              )}
              {fault.resolvedAt && (
                <div>
                  <dt className="text-gray-500">{t('faults.resolvedOn')}</dt>
                  <dd className="font-medium">{new Date(fault.resolvedAt).toLocaleString()}</dd>
                </div>
              )}
              {fault.rating && (
                <div>
                  <dt className="text-gray-500">{t('faults.rating')}</dt>
                  <dd className="font-medium">{'*'.repeat(fault.rating)}</dd>
                </div>
              )}
            </dl>
          </div>

          {/* AI Suggestion (if available) */}
          {fault.aiCategory && fault.aiConfidence && (
            <div className="bg-blue-50 rounded-lg p-6 border border-blue-100">
              <h3 className="text-lg font-semibold text-blue-900 mb-2">{t('faults.aiAnalysis')}</h3>
              <p className="text-sm text-blue-800">
                {t('faults.suggestedCategory')}: <strong>{fault.aiCategory}</strong>
              </p>
              {fault.aiPriority && (
                <p className="text-sm text-blue-800">
                  {t('faults.suggestedPriority')}: <strong>{fault.aiPriority}</strong>
                </p>
              )}
              <p className="text-sm text-blue-600 mt-1">
                {Math.round(fault.aiConfidence * 100)}% {t('faults.confidence')}
              </p>
            </div>
          )}
        </div>
      </div>

      {/* Triage Dialog */}
      <TriageFaultDialog
        isOpen={showTriageDialog}
        faultTitle={fault.title}
        currentCategory={fault.category}
        aiSuggestion={
          fault.aiCategory && fault.aiConfidence
            ? {
                category: fault.aiCategory,
                priority: fault.aiPriority,
                confidence: fault.aiConfidence,
              }
            : undefined
        }
        technicians={technicians}
        onSubmit={(data) => {
          onTriage(data);
          setShowTriageDialog(false);
        }}
        onClose={() => setShowTriageDialog(false)}
      />

      {/* Resolve Dialog */}
      {showResolveDialog && (
        <div className="fixed inset-0 z-50 overflow-y-auto">
          <button
            type="button"
            className="fixed inset-0 bg-black bg-opacity-50 cursor-default"
            onClick={() => setShowResolveDialog(false)}
            onKeyDown={(e) => e.key === 'Escape' && setShowResolveDialog(false)}
            aria-label={t('common.close')}
          />
          <div className="flex min-h-full items-center justify-center p-4">
            <div className="relative w-full max-w-md bg-white rounded-lg shadow-xl p-6">
              <h2 className="text-lg font-semibold mb-4">{t('faults.resolveFault')}</h2>
              <textarea
                value={resolutionNotes}
                onChange={(e) => setResolutionNotes(e.target.value)}
                rows={4}
                placeholder={t('faults.resolutionPlaceholder')}
                className="w-full rounded-md border border-gray-300 px-3 py-2"
              />
              <div className="flex justify-end gap-3 mt-4">
                <button
                  type="button"
                  onClick={() => setShowResolveDialog(false)}
                  className="px-4 py-2 border rounded-lg"
                >
                  {t('common.cancel')}
                </button>
                <button
                  type="button"
                  onClick={handleResolve}
                  className="px-4 py-2 bg-green-600 text-white rounded-lg"
                >
                  {t('faults.resolve')}
                </button>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Confirm Dialog */}
      {showConfirmDialog && (
        <div className="fixed inset-0 z-50 overflow-y-auto">
          <button
            type="button"
            className="fixed inset-0 bg-black bg-opacity-50 cursor-default"
            onClick={() => setShowConfirmDialog(false)}
            onKeyDown={(e) => e.key === 'Escape' && setShowConfirmDialog(false)}
            aria-label={t('common.close')}
          />
          <div className="flex min-h-full items-center justify-center p-4">
            <div className="relative w-full max-w-md bg-white rounded-lg shadow-xl p-6">
              <h2 className="text-lg font-semibold mb-4">{t('faults.confirmResolution')}</h2>
              <fieldset className="mb-4">
                <legend className="block text-sm font-medium text-gray-700 mb-2">
                  {t('faults.rateResolution')}
                </legend>
                <div className="flex gap-2">
                  {[1, 2, 3, 4, 5].map((star) => (
                    <button
                      key={star}
                      type="button"
                      onClick={() => setConfirmRating(star)}
                      className={`text-2xl ${confirmRating && confirmRating >= star ? 'text-yellow-400' : 'text-gray-300'}`}
                    >
                      *
                    </button>
                  ))}
                </div>
              </fieldset>
              <textarea
                value={confirmFeedback}
                onChange={(e) => setConfirmFeedback(e.target.value)}
                rows={3}
                placeholder={t('faults.feedbackPlaceholder')}
                className="w-full rounded-md border border-gray-300 px-3 py-2"
              />
              <div className="flex justify-end gap-3 mt-4">
                <button
                  type="button"
                  onClick={() => setShowConfirmDialog(false)}
                  className="px-4 py-2 border rounded-lg"
                >
                  {t('common.cancel')}
                </button>
                <button
                  type="button"
                  onClick={handleConfirm}
                  className="px-4 py-2 bg-green-600 text-white rounded-lg"
                >
                  {t('common.confirm')}
                </button>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Reopen Dialog */}
      {showReopenDialog && (
        <div className="fixed inset-0 z-50 overflow-y-auto">
          <button
            type="button"
            className="fixed inset-0 bg-black bg-opacity-50 cursor-default"
            onClick={() => setShowReopenDialog(false)}
            onKeyDown={(e) => e.key === 'Escape' && setShowReopenDialog(false)}
            aria-label={t('common.close')}
          />
          <div className="flex min-h-full items-center justify-center p-4">
            <div className="relative w-full max-w-md bg-white rounded-lg shadow-xl p-6">
              <h2 className="text-lg font-semibold mb-4">{t('faults.reopenFault')}</h2>
              <textarea
                value={reopenReason}
                onChange={(e) => setReopenReason(e.target.value)}
                rows={3}
                placeholder={t('faults.reopenPlaceholder')}
                className="w-full rounded-md border border-gray-300 px-3 py-2"
              />
              <div className="flex justify-end gap-3 mt-4">
                <button
                  type="button"
                  onClick={() => setShowReopenDialog(false)}
                  className="px-4 py-2 border rounded-lg"
                >
                  {t('common.cancel')}
                </button>
                <button
                  type="button"
                  onClick={handleReopen}
                  disabled={!reopenReason.trim()}
                  className="px-4 py-2 bg-orange-600 text-white rounded-lg disabled:opacity-50"
                >
                  {t('faults.reopen')}
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
