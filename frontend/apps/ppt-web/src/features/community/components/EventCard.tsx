/**
 * EventCard — an upcoming community event with RSVP buttons.
 *
 * The event list does not include the caller's own RSVP, so `myRsvp` is the
 * status confirmed by the server in this session (from the RSVP response).
 */

import type { CommunityEvent, CommunityRsvpStatus } from '@ppt/api-client';
import { useTranslation } from 'react-i18next';
import { formatDateTime, formatMoney } from '../utils/format';

const RSVP_OPTIONS: { status: CommunityRsvpStatus; key: string }[] = [
  { status: 'going', key: 'community.eventCard.btnGoing' },
  { status: 'maybe', key: 'community.eventCard.btnMaybe' },
  { status: 'not_going', key: 'community.eventCard.btnCantGo' },
];

const RSVP_CONFIRMATION: Partial<Record<CommunityRsvpStatus, string>> = {
  going: 'community.eventCard.rsvpGoing',
  maybe: 'community.eventCard.rsvpMaybe',
  not_going: 'community.eventCard.rsvpNotGoing',
};

interface EventCardProps {
  event: CommunityEvent;
  myRsvp?: CommunityRsvpStatus;
  pendingStatus?: CommunityRsvpStatus;
  onRsvp: (eventId: string, status: CommunityRsvpStatus) => void;
}

export function EventCard({ event, myRsvp, pendingStatus, onRsvp }: EventCardProps) {
  const { t, i18n } = useTranslation();
  const locale = i18n.language;
  const cost = formatMoney(event.costPerPerson, event.costCurrency, locale);
  const isOpen = event.status === 'scheduled';
  const confirmation = myRsvp ? RSVP_CONFIRMATION[myRsvp] : undefined;

  return (
    <article className="bg-white rounded-lg shadow p-4 flex flex-col">
      <div className="flex items-start justify-between gap-2">
        <h3 className="text-lg font-semibold text-gray-900">{event.title}</h3>
        <span className="text-xs px-2 py-0.5 rounded-full bg-blue-50 text-blue-700 shrink-0">
          {t(`community.eventTypes.${event.eventType}`)}
        </span>
      </div>
      <dl className="mt-2 space-y-1 text-sm text-gray-600">
        <div>
          <dt className="sr-only">{t('community.eventForm.dateTime')}</dt>
          <dd>
            {formatDateTime(event.startTime, locale, event.allDay)} –{' '}
            {formatDateTime(event.endTime, locale, event.allDay)}
          </dd>
        </div>
        {(event.location || event.isVirtual) && (
          <div>
            <dt className="sr-only">{t('community.eventForm.locationLabel')}</dt>
            <dd>
              {event.location}
              {event.locationDetails && ` (${event.locationDetails})`}
              {event.isVirtual && event.virtualLink && (
                <>
                  {event.location ? ' · ' : ''}
                  <a
                    href={event.virtualLink}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-blue-600 hover:text-blue-800"
                  >
                    {t('community.hub.joinOnline')}
                  </a>
                </>
              )}
            </dd>
          </div>
        )}
        <div>
          <dt className="sr-only">{t('community.hub.attendees')}</dt>
          <dd>
            {t('community.eventCard.going', { count: event.attendeeCount })}
            {event.maxAttendees != null &&
              ` · ${t('community.eventCard.max', { count: event.maxAttendees })}`}
            {cost && ` · ${cost}`}
          </dd>
        </div>
      </dl>
      {event.description && (
        <p className="mt-3 text-sm text-gray-700 whitespace-pre-wrap line-clamp-4">
          {event.description}
        </p>
      )}

      <div className="mt-auto pt-4">
        {confirmation && (
          <p className="mb-2 text-sm font-medium text-green-700">{t(confirmation)}</p>
        )}
        {isOpen ? (
          <fieldset className="flex flex-wrap gap-2">
            <legend className="sr-only">{t('community.hub.rsvp')}</legend>
            {RSVP_OPTIONS.map((opt) => (
              <button
                key={opt.status}
                type="button"
                onClick={() => onRsvp(event.id, opt.status)}
                disabled={!!pendingStatus}
                aria-pressed={myRsvp === opt.status}
                className={`px-3 py-1 text-sm rounded-md border disabled:opacity-50 ${
                  myRsvp === opt.status
                    ? 'bg-blue-600 border-blue-600 text-white'
                    : 'border-gray-300 text-gray-700 hover:bg-gray-50'
                }`}
              >
                {t(opt.key)}
              </button>
            ))}
          </fieldset>
        ) : (
          <p className="text-sm text-gray-500">{t('community.eventCard.rsvpNotAvailable')}</p>
        )}
      </div>
    </article>
  );
}
