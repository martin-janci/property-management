/**
 * FaultTimeline component - displays the history/timeline of a fault.
 * Epic 4: Fault Reporting & Resolution (UC-03.5, UC-03.7)
 */

import { useTranslation } from 'react-i18next';

export type TimelineAction =
  | 'created'
  | 'triaged'
  | 'assigned'
  | 'status_changed'
  | 'priority_changed'
  | 'work_note'
  | 'comment'
  | 'attachment_added'
  | 'scheduled'
  | 'resolved'
  | 'confirmed'
  | 'reopened'
  | 'rated';

export interface TimelineEntry {
  id: string;
  faultId: string;
  userId: string;
  action: TimelineAction;
  note?: string;
  oldValue?: string;
  newValue?: string;
  isInternal: boolean;
  createdAt: string;
  userName: string;
  userEmail: string;
}

interface FaultTimelineProps {
  entries: TimelineEntry[];
  isLoading?: boolean;
}

const actionIcons: Record<TimelineAction, string> = {
  created: '🆕',
  triaged: '📋',
  assigned: '👤',
  status_changed: '🔄',
  priority_changed: '⚡',
  work_note: '📝',
  comment: '💬',
  attachment_added: '📎',
  scheduled: '📅',
  resolved: '✅',
  confirmed: '✔️',
  reopened: '🔓',
  rated: '⭐',
};

export function FaultTimeline({ entries, isLoading }: FaultTimelineProps) {
  const { t, i18n } = useTranslation();

  if (isLoading) {
    return (
      <div className="flex justify-center py-4">
        <div className="animate-spin rounded-full h-6 w-6 border-b-2 border-blue-600" />
      </div>
    );
  }

  if (entries.length === 0) {
    return <div className="text-center py-4 text-gray-500">{t('faults.timeline.empty')}</div>;
  }

  return (
    <div className="space-y-0">
      <h3 className="text-lg font-semibold text-gray-900 mb-4">{t('faults.timeline.title')}</h3>
      <div className="relative">
        {/* Vertical line */}
        <div className="absolute left-4 top-2 bottom-2 w-0.5 bg-gray-200" />

        {entries.map((entry) => (
          <div key={entry.id} className="relative pl-10 pb-6">
            {/* Icon dot */}
            <div className="absolute left-2 w-5 h-5 bg-white border-2 border-gray-300 rounded-full flex items-center justify-center text-xs">
              {actionIcons[entry.action]}
            </div>

            {/* Content */}
            <div
              className={`p-3 rounded-lg ${
                entry.isInternal ? 'bg-yellow-50 border border-yellow-200' : 'bg-gray-50'
              }`}
            >
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <span className="font-medium text-gray-900">{entry.userName}</span>
                  <span className="text-gray-500">
                    {t(`faults.timeline.action.${entry.action}`)}
                  </span>
                  {entry.isInternal && (
                    <span className="px-2 py-0.5 text-xs bg-yellow-200 text-yellow-800 rounded">
                      {t('faults.timeline.internal')}
                    </span>
                  )}
                </div>
                <span className="text-xs text-gray-400">
                  {new Date(entry.createdAt).toLocaleString(i18n.language)}
                </span>
              </div>

              {/* Status/Priority change */}
              {(entry.action === 'status_changed' || entry.action === 'priority_changed') &&
                entry.oldValue &&
                entry.newValue && (
                  <div className="mt-1 text-sm text-gray-600">
                    <span className="line-through text-gray-400">{entry.oldValue}</span>
                    <span className="mx-2">→</span>
                    <span className="font-medium">{entry.newValue}</span>
                  </div>
                )}

              {/* Note/Comment */}
              {entry.note && <p className="mt-2 text-sm text-gray-700">{entry.note}</p>}

              {/* Attachment filename */}
              {entry.action === 'attachment_added' && entry.newValue && (
                <div className="mt-1 text-sm text-blue-600">📎 {entry.newValue}</div>
              )}

              {/* Rating */}
              {entry.action === 'rated' && entry.newValue && (
                <div className="mt-1 text-sm text-yellow-600">
                  {'⭐'.repeat(Number.parseInt(entry.newValue, 10))}
                </div>
              )}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
