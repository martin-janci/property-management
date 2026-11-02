/**
 * EventCard Component
 *
 * Displays a community event card with RSVP actions.
 * Part of Story 42.3: Community Events.
 */

import type { CommunityEvent, EventStatus, RsvpStatus } from '@ppt/api-client';
import type { TFunction } from 'i18next';
import { useTranslation } from 'react-i18next';

interface EventCardProps {
  event: CommunityEvent;
  onView?: (id: string) => void;
  onRsvp?: (id: string, status: RsvpStatus) => void;
  onEdit?: (id: string) => void;
  onDelete?: (id: string) => void;
  onExportCalendar?: (id: string) => void;
  isCurrentUserOrganizer?: boolean;
  isRsvping?: boolean;
}

const statusColors: Record<EventStatus, string> = {
  draft: 'bg-gray-100 text-gray-800',
  published: 'bg-green-100 text-green-800',
  cancelled: 'bg-red-100 text-red-800',
  completed: 'bg-blue-100 text-blue-800',
};

const rsvpColors: Record<RsvpStatus, string> = {
  going: 'bg-green-600 text-white',
  maybe: 'bg-yellow-500 text-white',
  not_going: 'bg-gray-400 text-white',
};

function formatEventDate(startDate: string, endDate: string, allDay: boolean): string {
  const start = new Date(startDate);
  const end = new Date(endDate);
  const sameDay = start.toDateString() === end.toDateString();

  const dateOptions: Intl.DateTimeFormatOptions = {
    weekday: 'short',
    month: 'short',
    day: 'numeric',
  };

  const timeOptions: Intl.DateTimeFormatOptions = {
    hour: 'numeric',
    minute: '2-digit',
  };

  if (allDay) {
    if (sameDay) {
      return start.toLocaleDateString(undefined, dateOptions);
    }
    return `${start.toLocaleDateString(undefined, dateOptions)} - ${end.toLocaleDateString(undefined, dateOptions)}`;
  }

  if (sameDay) {
    return `${start.toLocaleDateString(undefined, dateOptions)} · ${start.toLocaleTimeString(undefined, timeOptions)} - ${end.toLocaleTimeString(undefined, timeOptions)}`;
  }

  return `${start.toLocaleDateString(undefined, dateOptions)} ${start.toLocaleTimeString(undefined, timeOptions)} - ${end.toLocaleDateString(undefined, dateOptions)} ${end.toLocaleTimeString(undefined, timeOptions)}`;
}

function getRelativeTime(dateString: string, t: TFunction): string {
  const date = new Date(dateString);
  const now = new Date();
  const diff = date.getTime() - now.getTime();
  const days = Math.floor(diff / (1000 * 60 * 60 * 24));

  if (days < 0) return t('community.eventCard.relative.past');
  if (days === 0) return t('community.eventCard.relative.today');
  if (days === 1) return t('community.eventCard.relative.tomorrow');
  if (days < 7) return t('community.eventCard.relative.inDays', { count: days });
  if (days < 30) return t('community.eventCard.relative.inWeeks', { count: Math.floor(days / 7) });
  return t('community.eventCard.relative.inMonths', { count: Math.floor(days / 30) });
}

export function EventCard({
  event,
  onView,
  onRsvp,
  onEdit,
  onDelete,
  onExportCalendar,
  isCurrentUserOrganizer,
  isRsvping,
}: EventCardProps) {
  const { t } = useTranslation();
  const isPast = new Date(event.endDate) < new Date();
  const isCancelled = event.status === 'cancelled';
  const canRsvp = !isPast && !isCancelled && event.status === 'published';

  return (
    <div className="bg-white rounded-lg shadow overflow-hidden hover:shadow-md transition-shadow">
      {/* Cover Image */}
      <div className="h-36 bg-gradient-to-r from-purple-500 to-pink-500 relative">
        {event.coverImageUrl && (
          <img src={event.coverImageUrl} alt={event.title} className="w-full h-full object-cover" />
        )}
        {/* Date Badge */}
        <div className="absolute top-3 left-3 bg-white rounded-lg shadow px-3 py-2 text-center">
          <div className="text-xs font-medium text-gray-500 uppercase">
            {new Date(event.startDate).toLocaleDateString(undefined, { month: 'short' })}
          </div>
          <div className="text-2xl font-bold text-gray-900">
            {new Date(event.startDate).getDate()}
          </div>
        </div>
        {/* Status Badge */}
        {event.status !== 'published' && (
          <span
            className={`absolute top-3 right-3 px-2 py-1 text-xs font-medium rounded ${statusColors[event.status]}`}
          >
            {t(`community.eventCard.status.${event.status}`)}
          </span>
        )}
      </div>

      {/* Content */}
      <div className="p-4">
        <div className="flex items-start justify-between">
          <div className="flex-1 min-w-0">
            <span className="text-xs font-medium text-purple-600">
              {getRelativeTime(event.startDate, t)}
            </span>
            <button
              type="button"
              className="mt-1 text-lg font-semibold text-gray-900 truncate cursor-pointer hover:text-blue-600 text-left w-full"
              onClick={() => onView?.(event.id)}
            >
              {event.title}
            </button>
          </div>
        </div>

        {/* Date & Location */}
        <div className="mt-2 space-y-1 text-sm text-gray-500">
          <div className="flex items-center gap-2">
            <svg
              className="w-4 h-4 flex-shrink-0"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
              aria-hidden="true"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"
              />
            </svg>
            <span className="truncate">
              {formatEventDate(event.startDate, event.endDate, event.allDay)}
            </span>
          </div>
          <div className="flex items-center gap-2">
            <svg
              className="w-4 h-4 flex-shrink-0"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
              aria-hidden="true"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M17.657 16.657L13.414 20.9a1.998 1.998 0 01-2.827 0l-4.244-4.243a8 8 0 1111.314 0z"
              />
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M15 11a3 3 0 11-6 0 3 3 0 016 0z"
              />
            </svg>
            <span className="truncate">{event.location}</span>
          </div>
        </div>

        {/* Attendance Stats */}
        <div className="mt-3 flex items-center gap-4 text-xs text-gray-500">
          <span className="flex items-center gap-1">
            <svg
              className="w-4 h-4 text-green-500"
              fill="currentColor"
              viewBox="0 0 20 20"
              aria-hidden="true"
            >
              <path
                fillRule="evenodd"
                d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z"
                clipRule="evenodd"
              />
            </svg>
            {t('community.eventCard.going', { count: event.goingCount })}
          </span>
          <span className="flex items-center gap-1">
            <svg
              className="w-4 h-4 text-yellow-500"
              fill="currentColor"
              viewBox="0 0 20 20"
              aria-hidden="true"
            >
              <path
                fillRule="evenodd"
                d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-8-3a1 1 0 00-.867.5 1 1 0 11-1.731-1A3 3 0 0113 8a3.001 3.001 0 01-2 2.83V11a1 1 0 11-2 0v-1a1 1 0 011-1 1 1 0 100-2zm0 8a1 1 0 100-2 1 1 0 000 2z"
                clipRule="evenodd"
              />
            </svg>
            {t('community.eventCard.maybe', { count: event.maybeCount })}
          </span>
          {event.maxAttendees && (
            <span className="text-gray-400">
              {t('community.eventCard.max', { count: event.maxAttendees })}
            </span>
          )}
        </div>

        {/* User RSVP Status */}
        {event.userRsvp && (
          <div className="mt-3">
            <span
              className={`inline-flex items-center px-2 py-1 rounded-full text-xs font-medium ${rsvpColors[event.userRsvp]}`}
            >
              {event.userRsvp === 'going' && t('community.eventCard.rsvpGoing')}
              {event.userRsvp === 'maybe' && t('community.eventCard.rsvpMaybe')}
              {event.userRsvp === 'not_going' && t('community.eventCard.rsvpNotGoing')}
            </span>
          </div>
        )}
      </div>

      {/* Actions */}
      <div className="px-4 py-3 bg-gray-50 border-t flex items-center justify-between">
        {canRsvp ? (
          <div className="flex items-center gap-2">
            <button
              type="button"
              onClick={() => onRsvp?.(event.id, 'going')}
              disabled={isRsvping}
              className={`px-3 py-1 text-sm rounded-md transition-colors ${
                event.userRsvp === 'going'
                  ? 'bg-green-600 text-white'
                  : 'bg-green-100 text-green-700 hover:bg-green-200'
              }`}
            >
              {t('community.eventCard.btnGoing')}
            </button>
            <button
              type="button"
              onClick={() => onRsvp?.(event.id, 'maybe')}
              disabled={isRsvping}
              className={`px-3 py-1 text-sm rounded-md transition-colors ${
                event.userRsvp === 'maybe'
                  ? 'bg-yellow-500 text-white'
                  : 'bg-yellow-100 text-yellow-700 hover:bg-yellow-200'
              }`}
            >
              {t('community.eventCard.btnMaybe')}
            </button>
            <button
              type="button"
              onClick={() => onRsvp?.(event.id, 'not_going')}
              disabled={isRsvping}
              className={`px-3 py-1 text-sm rounded-md transition-colors ${
                event.userRsvp === 'not_going'
                  ? 'bg-gray-500 text-white'
                  : 'bg-gray-100 text-gray-700 hover:bg-gray-200'
              }`}
            >
              {t('community.eventCard.btnCantGo')}
            </button>
          </div>
        ) : (
          <span className="text-sm text-gray-500">
            {isPast
              ? t('community.eventCard.ended')
              : isCancelled
                ? t('community.eventCard.eventCancelled')
                : t('community.eventCard.rsvpNotAvailable')}
          </span>
        )}

        <div className="flex items-center gap-2">
          <button
            type="button"
            onClick={() => onExportCalendar?.(event.id)}
            className="p-2 text-gray-400 hover:text-gray-600 rounded"
            title={t('community.eventCard.addToCalendar')}
          >
            <svg
              className="w-5 h-5"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
              aria-hidden="true"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M8 7V3m8 4V3m-9 8h10M5 21h14a2 2 0 002-2V7a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z"
              />
            </svg>
          </button>
          {isCurrentUserOrganizer && (
            <>
              <button
                type="button"
                onClick={() => onEdit?.(event.id)}
                className="p-2 text-gray-400 hover:text-gray-600 rounded"
                title={t('community.eventCard.editEvent')}
              >
                <svg
                  className="w-5 h-5"
                  fill="none"
                  viewBox="0 0 24 24"
                  stroke="currentColor"
                  aria-hidden="true"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"
                  />
                </svg>
              </button>
              <button
                type="button"
                onClick={() => onDelete?.(event.id)}
                className="p-2 text-gray-400 hover:text-red-600 rounded"
                title={t('community.eventCard.deleteEvent')}
              >
                <svg
                  className="w-5 h-5"
                  fill="none"
                  viewBox="0 0 24 24"
                  stroke="currentColor"
                  aria-hidden="true"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"
                  />
                </svg>
              </button>
            </>
          )}
        </div>
      </div>
    </div>
  );
}
