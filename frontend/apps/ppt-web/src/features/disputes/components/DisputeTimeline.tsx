/**
 * DisputeTimeline - displays activity timeline for a dispute.
 * Epic 77: Dispute Resolution (Story 77.3)
 */

import { useTranslation } from 'react-i18next';
import { formatTime } from '../utils/formatTime';

export type ActivityType =
  | 'dispute_filed'
  | 'status_changed'
  | 'party_added'
  | 'evidence_added'
  | 'session_scheduled'
  | 'session_completed'
  | 'resolution_proposed'
  | 'resolution_voted'
  | 'resolution_accepted'
  | 'action_created'
  | 'action_completed'
  | 'comment_added'
  | 'escalated'
  | 'closed';

export interface TimelineEntry {
  id: string;
  actorId: string;
  actorName: string;
  activityType: ActivityType;
  description: string;
  metadata?: Record<string, unknown>;
  createdAt: string;
}

// Icons for timeline entries - exported for use in other components
export const activityIcons: Record<ActivityType, string> = {
  dispute_filed: 'document-plus',
  status_changed: 'arrow-path',
  party_added: 'user-plus',
  evidence_added: 'photo',
  session_scheduled: 'calendar',
  session_completed: 'check-circle',
  resolution_proposed: 'light-bulb',
  resolution_voted: 'hand-raised',
  resolution_accepted: 'check-badge',
  action_created: 'clipboard-list',
  action_completed: 'clipboard-check',
  comment_added: 'chat-bubble-left',
  escalated: 'arrow-trending-up',
  closed: 'lock-closed',
};

const activityColors: Record<ActivityType, string> = {
  dispute_filed: 'bg-blue-100 text-blue-600',
  status_changed: 'bg-yellow-100 text-yellow-600',
  party_added: 'bg-purple-100 text-purple-600',
  evidence_added: 'bg-indigo-100 text-indigo-600',
  session_scheduled: 'bg-orange-100 text-orange-600',
  session_completed: 'bg-green-100 text-green-600',
  resolution_proposed: 'bg-cyan-100 text-cyan-600',
  resolution_voted: 'bg-pink-100 text-pink-600',
  resolution_accepted: 'bg-emerald-100 text-emerald-600',
  action_created: 'bg-gray-100 text-gray-600',
  action_completed: 'bg-green-100 text-green-600',
  comment_added: 'bg-blue-100 text-blue-600',
  escalated: 'bg-red-100 text-red-600',
  closed: 'bg-gray-100 text-gray-600',
};

interface DisputeTimelineProps {
  entries: TimelineEntry[];
  maxEntries?: number;
  onShowMore?: () => void;
}

export function DisputeTimeline({ entries, maxEntries, onShowMore }: DisputeTimelineProps) {
  const displayedEntries = maxEntries ? entries.slice(0, maxEntries) : entries;
  const hasMore = maxEntries ? entries.length > maxEntries : false;
  const { t, i18n } = useTranslation();

  return (
    <div className="space-y-4">
      <h3 className="text-lg font-semibold text-gray-900">{t('disputes.timeline.title')}</h3>

      {entries.length === 0 ? (
        <p className="text-gray-500">{t('disputes.timeline.empty')}</p>
      ) : (
        <div className="relative">
          {/* Timeline line */}
          <div className="absolute left-4 top-0 bottom-0 w-0.5 bg-gray-200" />

          <div className="space-y-4">
            {displayedEntries.map((entry) => (
              <div key={entry.id} className="relative flex items-start gap-4 pl-10">
                {/* Icon */}
                <div
                  className={`absolute left-0 w-8 h-8 rounded-full flex items-center justify-center ${activityColors[entry.activityType]}`}
                >
                  <span className="text-sm">
                    {entry.activityType === 'dispute_filed' && '+'}
                    {entry.activityType === 'status_changed' && '~'}
                    {entry.activityType === 'party_added' && '+'}
                    {entry.activityType === 'evidence_added' && 'E'}
                    {entry.activityType === 'session_scheduled' && 'S'}
                    {entry.activityType === 'session_completed' && 'V'}
                    {entry.activityType === 'resolution_proposed' && 'P'}
                    {entry.activityType === 'resolution_voted' && 'V'}
                    {entry.activityType === 'resolution_accepted' && 'A'}
                    {entry.activityType === 'action_created' && 'T'}
                    {entry.activityType === 'action_completed' && 'D'}
                    {entry.activityType === 'comment_added' && 'C'}
                    {entry.activityType === 'escalated' && '!'}
                    {entry.activityType === 'closed' && 'X'}
                  </span>
                </div>

                {/* Content */}
                <div className="flex-1 bg-white rounded-lg border border-gray-200 p-3">
                  <div className="flex items-center justify-between">
                    <span className="font-medium text-gray-900">{entry.actorName}</span>
                    <span className="text-sm text-gray-500">
                      {formatTime(entry.createdAt, i18n.language)}
                    </span>
                  </div>
                  <p className="mt-1 text-gray-700">{entry.description}</p>
                  {entry.metadata && Object.keys(entry.metadata).length > 0 && (
                    <div className="mt-2 text-sm text-gray-500">
                      {'oldStatus' in entry.metadata && 'newStatus' in entry.metadata && (
                        <span>
                          {t('disputes.timeline.statusChange', {
                            from: String(entry.metadata.oldStatus),
                            to: String(entry.metadata.newStatus),
                          })}
                        </span>
                      )}
                    </div>
                  )}
                </div>
              </div>
            ))}
          </div>

          {hasMore && onShowMore && (
            <button
              type="button"
              onClick={onShowMore}
              className="mt-4 ml-10 text-blue-600 hover:text-blue-800 text-sm"
            >
              {t('disputes.timeline.showMore')}
            </button>
          )}
        </div>
      )}
    </div>
  );
}
