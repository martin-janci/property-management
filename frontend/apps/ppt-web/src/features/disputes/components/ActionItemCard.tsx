/**
 * ActionItemCard - displays an action item for resolution enforcement.
 * Epic 77: Dispute Resolution (Story 77.4)
 */

import { useTranslation } from 'react-i18next';

export type ActionStatus = 'pending' | 'in_progress' | 'completed' | 'overdue' | 'escalated';

export interface ActionItem {
  id: string;
  disputeId: string;
  resolutionId?: string;
  resolutionTermId?: string;
  assignedTo: string;
  assignedToName: string;
  title: string;
  description: string;
  dueDate: string;
  status: ActionStatus;
  completedAt?: string;
  completionNotes?: string;
  reminderSentAt?: string;
  escalatedAt?: string;
}

const statusColors: Record<ActionStatus, string> = {
  pending: 'bg-yellow-100 text-yellow-800',
  in_progress: 'bg-blue-100 text-blue-800',
  completed: 'bg-green-100 text-green-800',
  overdue: 'bg-red-100 text-red-800',
  escalated: 'bg-red-200 text-red-900',
};

/** i18n keys for action item status labels. */
const statusLabelKeys: Record<ActionStatus, string> = {
  pending: 'disputes.action.status.pending',
  in_progress: 'disputes.action.status.in_progress',
  completed: 'disputes.action.status.completed',
  overdue: 'disputes.action.status.overdue',
  escalated: 'disputes.action.status.escalated',
};

interface ActionItemCardProps {
  action: ActionItem;
  isAssignee?: boolean;
  isManager?: boolean;
  onStart?: (id: string) => void;
  onComplete?: (id: string, notes?: string) => void;
  onSendReminder?: (id: string) => void;
  onEscalate?: (id: string) => void;
  onView?: (id: string) => void;
}

export function ActionItemCard({
  action,
  isAssignee,
  isManager,
  onStart,
  onComplete,
  onSendReminder,
  onEscalate,
  onView,
}: ActionItemCardProps) {
  const { t, i18n } = useTranslation();
  const dueDate = new Date(action.dueDate);
  const now = new Date();
  const isOverdue = dueDate < now && action.status !== 'completed';
  const daysUntilDue = Math.ceil((dueDate.getTime() - now.getTime()) / (1000 * 60 * 60 * 24));

  const formatDate = (dateString: string) => {
    return new Date(dateString).toLocaleDateString(i18n.language);
  };

  const getDueDateDisplay = () => {
    if (action.status === 'completed') {
      return <span className="text-green-600">{t('disputes.action.status.completed')}</span>;
    }
    if (isOverdue) {
      return (
        <span className="text-red-600 font-medium">
          {t('disputes.action.overdueBy', { count: Math.abs(daysUntilDue) })}
        </span>
      );
    }
    if (daysUntilDue === 0) {
      return <span className="text-orange-600 font-medium">{t('disputes.action.dueToday')}</span>;
    }
    if (daysUntilDue === 1) {
      return <span className="text-orange-600">{t('disputes.action.dueTomorrow')}</span>;
    }
    if (daysUntilDue <= 3) {
      return (
        <span className="text-yellow-600">
          {t('disputes.action.dueInDays', { count: daysUntilDue })}
        </span>
      );
    }
    return (
      <span className="text-gray-600">
        {t('disputes.action.dueOn', { date: formatDate(action.dueDate) })}
      </span>
    );
  };

  return (
    <div
      className={`bg-white rounded-lg shadow border p-4 ${
        isOverdue ? 'border-red-300' : 'border-gray-200'
      }`}
    >
      {/* Header */}
      <div className="flex items-start justify-between mb-2">
        <div className="flex-1">
          <div className="flex items-center gap-2 mb-1">
            <span
              className={`px-2 py-0.5 text-xs font-medium rounded ${statusColors[action.status]}`}
            >
              {t(statusLabelKeys[action.status])}
            </span>
            {getDueDateDisplay()}
          </div>
          <h4 className="font-medium text-gray-900">{action.title}</h4>
        </div>
      </div>

      {/* Description */}
      <p className="text-sm text-gray-700 mb-3">{action.description}</p>

      {/* Assignee */}
      <div className="text-sm text-gray-500 mb-3">
        <span>{t('disputes.action.assignedTo')} </span>
        <span className="font-medium text-gray-700">{action.assignedToName}</span>
      </div>

      {/* Completion Notes */}
      {action.completedAt && action.completionNotes && (
        <div className="mb-3 p-2 bg-green-50 border border-green-100 rounded">
          <p className="text-sm text-green-800">
            <span className="font-medium">{t('disputes.action.completedLabel')}</span>{' '}
            {action.completionNotes}
          </p>
          <p className="text-xs text-green-600 mt-1">{formatDate(action.completedAt)}</p>
        </div>
      )}

      {/* Reminder/Escalation Info */}
      {action.reminderSentAt && (
        <div className="text-xs text-gray-500 mb-2">
          {t('disputes.action.reminderSent', { date: formatDate(action.reminderSentAt) })}
        </div>
      )}
      {action.escalatedAt && (
        <div className="text-xs text-red-600 mb-2">
          {t('disputes.action.escalatedOn', { date: formatDate(action.escalatedAt) })}
        </div>
      )}

      {/* Actions */}
      <div className="flex justify-end gap-2 pt-3 border-t">
        {onView && (
          <button
            type="button"
            onClick={() => onView(action.id)}
            className="px-3 py-1.5 text-sm border border-gray-300 rounded-lg hover:bg-gray-50"
          >
            {t('disputes.view')}
          </button>
        )}

        {/* Assignee actions */}
        {isAssignee && action.status === 'pending' && onStart && (
          <button
            type="button"
            onClick={() => onStart(action.id)}
            className="px-3 py-1.5 text-sm bg-blue-600 text-white rounded-lg hover:bg-blue-700"
          >
            {t('disputes.action.start')}
          </button>
        )}
        {isAssignee &&
          ['pending', 'in_progress', 'overdue'].includes(action.status) &&
          onComplete && (
            <button
              type="button"
              onClick={() => onComplete(action.id)}
              className="px-3 py-1.5 text-sm bg-green-600 text-white rounded-lg hover:bg-green-700"
            >
              {t('disputes.action.complete')}
            </button>
          )}

        {/* Manager actions */}
        {isManager && ['pending', 'in_progress', 'overdue'].includes(action.status) && (
          <>
            {onSendReminder && (
              <button
                type="button"
                onClick={() => onSendReminder(action.id)}
                className="px-3 py-1.5 text-sm border border-orange-300 text-orange-600 rounded-lg hover:bg-orange-50"
              >
                {t('disputes.action.sendReminder')}
              </button>
            )}
            {onEscalate && action.status !== 'escalated' && (
              <button
                type="button"
                onClick={() => onEscalate(action.id)}
                className="px-3 py-1.5 text-sm border border-red-300 text-red-600 rounded-lg hover:bg-red-50"
              >
                {t('disputes.action.escalate')}
              </button>
            )}
          </>
        )}
      </div>
    </div>
  );
}

export { statusLabelKeys };
