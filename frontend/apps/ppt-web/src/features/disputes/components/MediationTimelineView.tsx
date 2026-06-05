import { type TimelineEventType, useDisputeTimeline } from '@ppt/api-client';
import { useTranslation } from 'react-i18next';
import { Spinner } from '../../../components/Spinner';
import { formatTime } from '../utils/formatTime';

interface MediationTimelineViewProps {
  disputeId: string;
}

const eventAriaLabelKeys: Record<TimelineEventType, string> = {
  dispute_filed: 'disputes.mediation.timeline.icon.disputeFiled',
  status_changed: 'disputes.mediation.timeline.icon.statusChanged',
  mediator_assigned: 'disputes.mediation.timeline.icon.mediatorAssigned',
  evidence_added: 'disputes.mediation.timeline.icon.evidenceAdded',
  note_added: 'disputes.mediation.timeline.icon.noteAdded',
  meeting_scheduled: 'disputes.mediation.timeline.icon.meetingScheduled',
  resolution_proposed: 'disputes.mediation.timeline.icon.resolutionProposed',
  resolution_accepted: 'disputes.mediation.timeline.icon.resolutionAccepted',
  escalated: 'disputes.mediation.timeline.icon.escalated',
  closed: 'disputes.mediation.timeline.icon.closed',
};

const eventIconMap: Record<TimelineEventType, string> = {
  dispute_filed: '📋',
  status_changed: '🔄',
  mediator_assigned: '👤',
  evidence_added: '📎',
  note_added: '📝',
  meeting_scheduled: '📅',
  resolution_proposed: '💡',
  resolution_accepted: '✅',
  escalated: '⚠️',
  closed: '🔒',
};

const eventColorMap: Record<TimelineEventType, string> = {
  dispute_filed: 'bg-blue-100',
  status_changed: 'bg-yellow-100',
  mediator_assigned: 'bg-violet-100',
  evidence_added: 'bg-indigo-100',
  note_added: 'bg-gray-100',
  meeting_scheduled: 'bg-teal-100',
  resolution_proposed: 'bg-orange-100',
  resolution_accepted: 'bg-green-100',
  escalated: 'bg-red-100',
  closed: 'bg-gray-200',
};

export function MediationTimelineView({ disputeId }: MediationTimelineViewProps) {
  const { t, i18n } = useTranslation();
  const { data: events, isLoading } = useDisputeTimeline(disputeId);

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
                    <p className="text-xs text-gray-500 mt-0.5">{event.actorName}</p>
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
