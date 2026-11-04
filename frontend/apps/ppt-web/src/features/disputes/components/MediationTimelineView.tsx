import { type TimelineEventType, useDispute, useDisputeTimeline } from '@ppt/api-client';
import { useTranslation } from 'react-i18next';
import { Spinner } from '../../../components/Spinner';
import { formatTime } from '../utils/formatTime';

interface MediationTimelineViewProps {
  disputeId: string;
}

const eventAriaLabelKeys: Record<TimelineEventType, string> = {
  dispute_filed: 'disputes.mediation.timeline.icon.disputeFiled',
  status_changed: 'disputes.mediation.timeline.icon.statusChanged',
  party_added: 'disputes.mediation.timeline.icon.partyAdded',
  evidence_added: 'disputes.mediation.timeline.icon.evidenceAdded',
  session_scheduled: 'disputes.mediation.timeline.icon.meetingScheduled',
  session_completed: 'disputes.mediation.timeline.icon.sessionCompleted',
  resolution_proposed: 'disputes.mediation.timeline.icon.resolutionProposed',
  resolution_voted: 'disputes.mediation.timeline.icon.resolutionVoted',
  resolution_accepted: 'disputes.mediation.timeline.icon.resolutionAccepted',
  action_created: 'disputes.mediation.timeline.icon.actionCreated',
  action_completed: 'disputes.mediation.timeline.icon.actionCompleted',
  comment_added: 'disputes.mediation.timeline.icon.noteAdded',
  escalated: 'disputes.mediation.timeline.icon.escalated',
  closed: 'disputes.mediation.timeline.icon.closed',
};

const eventIconMap: Record<TimelineEventType, string> = {
  dispute_filed: '📋',
  status_changed: '🔄',
  party_added: '👤',
  evidence_added: '📎',
  session_scheduled: '📅',
  session_completed: '🤝',
  resolution_proposed: '💡',
  resolution_voted: '🗳️',
  resolution_accepted: '✅',
  action_created: '📌',
  action_completed: '☑️',
  comment_added: '📝',
  escalated: '⚠️',
  closed: '🔒',
};

const eventColorMap: Record<TimelineEventType, string> = {
  dispute_filed: 'bg-blue-100',
  status_changed: 'bg-yellow-100',
  party_added: 'bg-violet-100',
  evidence_added: 'bg-indigo-100',
  session_scheduled: 'bg-teal-100',
  session_completed: 'bg-teal-200',
  resolution_proposed: 'bg-orange-100',
  resolution_voted: 'bg-orange-50',
  resolution_accepted: 'bg-green-100',
  action_created: 'bg-sky-100',
  action_completed: 'bg-green-50',
  comment_added: 'bg-gray-100',
  escalated: 'bg-red-100',
  closed: 'bg-gray-200',
};

export function MediationTimelineView({ disputeId }: MediationTimelineViewProps) {
  const { t, i18n } = useTranslation();
  const { data: events, isLoading } = useDisputeTimeline(disputeId);
  // The activity log carries only actor ids; resolve names from the parties
  // already loaded (and cached) by the workspace page's useDispute call.
  const { data: dispute } = useDispute(disputeId);
  const actorNames = new Map((dispute?.parties ?? []).map((p) => [p.userId, p.userName]));

  if (isLoading) {
    return (
      <div className="flex justify-center py-8">
        <Spinner size="md" />
      </div>
    );
  }

  if (!events || events.length === 0) {
    return (
      <p className="text-sm text-gray-500 text-center py-8">
        {t('disputes.mediation.noTimelineEvents')}
      </p>
    );
  }

  return (
    <div className="relative">
      <div className="absolute left-4 top-0 bottom-0 w-px bg-gray-200" aria-hidden="true" />
      <ul className="space-y-4">
        {events.map((event) => {
          const ariaLabel = t(
            eventAriaLabelKeys[event.eventType] ?? 'disputes.mediation.timeline.icon.statusChanged'
          );
          const icon = eventIconMap[event.eventType] ?? '•';
          const colorClass = eventColorMap[event.eventType] ?? 'bg-gray-100';

          return (
            <li key={event.id} className="relative pl-10">
              <div
                aria-label={ariaLabel}
                className={`absolute left-0 w-8 h-8 rounded-full flex items-center justify-center text-sm ${colorClass}`}
              >
                {icon}
              </div>
              <div className="bg-white rounded-lg border border-gray-100 p-3 shadow-sm">
                <div className="flex items-start justify-between gap-2">
                  <div>
                    <p className="text-sm font-medium text-gray-900">{event.description}</p>
                    {actorNames.has(event.actorId) && (
                      <p className="text-xs text-gray-500 mt-0.5">
                        {actorNames.get(event.actorId)}
                      </p>
                    )}
                  </div>
                  <time dateTime={event.createdAt} className="text-xs text-gray-400 shrink-0">
                    {formatTime(event.createdAt, i18n.language)}
                  </time>
                </div>
                {event.metadata && Object.keys(event.metadata).length > 0 && (
                  <dl className="mt-2 grid grid-cols-2 gap-1">
                    {Object.entries(event.metadata).map(([key, value]) => (
                      <div key={key} className="text-xs">
                        <dt className="text-gray-400 inline">{key}: </dt>
                        <dd className="text-gray-600 inline">{String(value)}</dd>
                      </div>
                    ))}
                  </dl>
                )}
              </div>
            </li>
          );
        })}
      </ul>
    </div>
  );
}
