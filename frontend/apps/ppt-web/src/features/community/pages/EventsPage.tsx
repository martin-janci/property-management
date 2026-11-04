/**
 * EventsPage — `/community/events`.
 *
 * Upcoming scheduled events of the selected building
 * (`GET /community/buildings/{id}/events`, ordered by start time) with RSVP
 * (`POST /community/events/{id}/rsvp`, an upsert). The list does not carry the
 * caller's RSVP, so the confirmed status is kept per event for this session.
 *
 * Not offered (no server endpoint): event detail page, editing / deleting /
 * cancelling events, attendee list, calendar export.
 */

import {
  type CommunityEventType,
  type CommunityRsvpStatus,
  useEvents,
  useRsvpEvent,
} from '@ppt/api-client';
import { useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { useToast } from '../../../components';
import { getErrorMessage } from '../../../lib/api';
import { CommunityLayout } from '../components/CommunityLayout';
import { EventCard } from '../components/EventCard';
import { EmptyState, ErrorState, LoadingState } from '../components/StateViews';
import { useCommunityBuilding, withBuilding } from '../hooks/useCommunityBuilding';

export const EVENT_TYPES: CommunityEventType[] = [
  'social',
  'meeting',
  'workshop',
  'volunteer',
  'sports',
  'cultural',
  'other',
];

export function EventsPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const building = useCommunityBuilding();

  return (
    <CommunityLayout
      active="events"
      title={t('community.events.title')}
      subtitle={t('community.events.subtitle')}
      building={building}
      actions={
        <button
          type="button"
          onClick={() => navigate(withBuilding('/community/events/new', building.buildingId))}
          className="px-4 py-2 text-sm font-medium bg-blue-600 text-white rounded-md hover:bg-blue-700"
        >
          {t('community.events.create')}
        </button>
      }
    >
      {(buildingId) => <EventsContent buildingId={buildingId} />}
    </CommunityLayout>
  );
}

function EventsContent({ buildingId }: { buildingId: string }) {
  const { t } = useTranslation();
  const { showToast } = useToast();
  const [type, setType] = useState<CommunityEventType | ''>('');
  const [myRsvps, setMyRsvps] = useState<Record<string, CommunityRsvpStatus>>({});

  const eventsQuery = useEvents(buildingId);
  const rsvp = useRsvpEvent();

  const events = useMemo(
    () => (eventsQuery.data ?? []).filter((e) => !type || e.eventType === type),
    [eventsQuery.data, type]
  );

  const handleRsvp = (eventId: string, status: CommunityRsvpStatus) =>
    rsvp.mutate(
      { eventId, data: { status } },
      {
        onSuccess: (result) => {
          setMyRsvps((prev) => ({ ...prev, [eventId]: result.status }));
          showToast({
            type: 'success',
            title: t('common.success'),
            message: t('community.hub.rsvpSaved'),
          });
        },
        onError: (err) =>
          showToast({
            type: 'error',
            title: t('community.hub.rsvpFailed'),
            message: getErrorMessage(err),
          }),
      }
    );

  if (eventsQuery.isLoading) return <LoadingState />;
  if (eventsQuery.error) {
    return (
      <ErrorState message={t('community.hub.loadError')} onRetry={() => eventsQuery.refetch()} />
    );
  }

  return (
    <div className="space-y-6">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <select
          value={type}
          onChange={(e) => setType(e.target.value as CommunityEventType | '')}
          aria-label={t('community.hub.eventType')}
          className="px-3 py-2 text-sm border border-gray-300 rounded-md"
        >
          <option value="">{t('community.hub.allEventTypes')}</option>
          {EVENT_TYPES.map((et) => (
            <option key={et} value={et}>
              {t(`community.eventTypes.${et}`)}
            </option>
          ))}
        </select>
        <span className="text-sm text-gray-500">
          {events.length}{' '}
          {events.length === 1 ? t('community.events.event') : t('community.events.events')}
        </span>
      </div>

      {events.length === 0 ? (
        <EmptyState
          title={t('community.events.emptyTitle')}
          body={
            type ? t('community.events.emptyBodyDefault') : t('community.events.emptyBodyUpcoming')
          }
        />
      ) : (
        <div className="grid gap-4 md:grid-cols-2">
          {events.map((event) => (
            <EventCard
              key={event.id}
              event={event}
              myRsvp={myRsvps[event.id]}
              pendingStatus={
                rsvp.isPending && rsvp.variables?.eventId === event.id
                  ? rsvp.variables.data.status
                  : undefined
              }
              onRsvp={handleRsvp}
            />
          ))}
        </div>
      )}
    </div>
  );
}
