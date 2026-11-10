/**
 * CreateEventPage — `/community/events/new`.
 *
 * Creates an event in the selected building
 * (`POST /community/buildings/{id}/events`). Local date/time inputs are sent
 * as ISO-8601 instants; all-day events span the chosen days in local time.
 */

import { type CommunityEventType, useCreateEvent } from '@ppt/api-client';
import { type FormEvent, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { useToast } from '../../../components';
import { getErrorMessage } from '../../../lib/api';
import { CommunityLayout } from '../components/CommunityLayout';
import { useCommunityBuilding, withBuilding } from '../hooks/useCommunityBuilding';
import { EVENT_TYPES } from './EventsPage';

export function CreateEventPage() {
  const { t } = useTranslation();
  const building = useCommunityBuilding();

  return (
    <CommunityLayout active="events" title={t('community.eventForm.create')} building={building}>
      {(buildingId) => <CreateEventForm buildingId={buildingId} />}
    </CommunityLayout>
  );
}

/** Parse a `<input type="date|datetime-local">` value as local time; null if invalid. */
function toInstant(value: string, allDay: boolean, endOfDay: boolean): Date | null {
  if (!value) return null;
  const local = allDay ? `${value}T${endOfDay ? '23:59:59' : '00:00:00'}` : value;
  const d = new Date(local);
  return Number.isNaN(d.getTime()) ? null : d;
}

function CreateEventForm({ buildingId }: { buildingId: string }) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { showToast } = useToast();
  const createEvent = useCreateEvent();

  const [title, setTitle] = useState('');
  const [description, setDescription] = useState('');
  const [eventType, setEventType] = useState<CommunityEventType>('social');
  const [location, setLocation] = useState('');
  const [locationDetails, setLocationDetails] = useState('');
  const [isVirtual, setIsVirtual] = useState(false);
  const [virtualLink, setVirtualLink] = useState('');
  const [allDay, setAllDay] = useState(false);
  const [start, setStart] = useState('');
  const [end, setEnd] = useState('');
  const [maxAttendees, setMaxAttendees] = useState('');
  const [cost, setCost] = useState('');
  const [touched, setTouched] = useState(false);
  const [submitError, setSubmitError] = useState<string | null>(null);

  const startAt = toInstant(start, allDay, false);
  const endAt = toInstant(end, allDay, true);
  const max = maxAttendees.trim() ? Number(maxAttendees) : undefined;
  const costValue = cost.trim() ? Number(cost.replace(',', '.')) : undefined;

  const errors: Record<string, string | null> = {
    title:
      title.trim().length === 0
        ? t('community.eventForm.errors.titleRequired')
        : title.trim().length < 3
          ? t('community.eventForm.errors.titleMin')
          : null,
    start: startAt ? null : t('community.eventForm.errors.startRequired'),
    end: !endAt
      ? t('community.eventForm.errors.endRequired')
      : startAt && endAt <= startAt
        ? t('community.eventForm.errors.endAfterStart')
        : null,
    virtualLink:
      isVirtual && virtualLink.trim() && !/^https?:\/\/\S+$/i.test(virtualLink.trim())
        ? t('community.eventForm.errors.invalidUrl')
        : null,
    maxAttendees:
      max !== undefined && (!Number.isInteger(max) || max < 1)
        ? t('community.eventForm.errors.invalidNumber')
        : null,
    cost:
      costValue !== undefined && (!Number.isFinite(costValue) || costValue < 0)
        ? t('community.eventForm.errors.invalidNumber')
        : null,
  };
  const hasErrors = Object.values(errors).some(Boolean);

  const handleSubmit = (e: FormEvent) => {
    e.preventDefault();
    setTouched(true);
    if (hasErrors || !startAt || !endAt) return;
    setSubmitError(null);
    createEvent.mutate(
      {
        buildingId,
        data: {
          title: title.trim(),
          description: description.trim() || undefined,
          eventType,
          location: location.trim() || undefined,
          locationDetails: locationDetails.trim() || undefined,
          isVirtual: isVirtual || undefined,
          virtualLink: isVirtual ? virtualLink.trim() || undefined : undefined,
          startTime: startAt.toISOString(),
          endTime: endAt.toISOString(),
          allDay: allDay || undefined,
          maxAttendees: max,
          costPerPerson: costValue !== undefined ? costValue.toFixed(2) : undefined,
        },
      },
      {
        onSuccess: () => {
          showToast({
            type: 'success',
            title: t('common.success'),
            message: t('community.hub.eventCreated'),
          });
          navigate(withBuilding('/community/events', buildingId));
        },
        onError: (err) => {
          const message = getErrorMessage(err);
          setSubmitError(message);
          showToast({ type: 'error', title: t('community.hub.createEventFailed'), message });
        },
      }
    );
  };

  const inputClass =
    'mt-1 w-full px-3 py-2 text-sm border border-gray-300 rounded-md focus:ring-blue-500 focus:border-blue-500';
  const err = (key: string) =>
    touched && errors[key] ? <p className="mt-1 text-sm text-red-600">{errors[key]}</p> : null;

  return (
    <form
      onSubmit={handleSubmit}
      noValidate
      className="max-w-2xl bg-white rounded-lg shadow p-6 space-y-5"
    >
      {submitError && (
        <div
          role="alert"
          className="rounded-md bg-red-50 border border-red-200 p-3 text-sm text-red-700"
        >
          {t('community.hub.createEventFailed')}: {submitError}
        </div>
      )}
      <div>
        <label htmlFor="event-title" className="block text-sm font-medium text-gray-700">
          {t('community.eventForm.titleLabel')} *
        </label>
        <input
          id="event-title"
          value={title}
          onChange={(e) => setTitle(e.target.value)}
          placeholder={t('community.eventForm.titlePlaceholder')}
          className={inputClass}
        />
        {err('title')}
      </div>
      <div>
        <label htmlFor="event-type" className="block text-sm font-medium text-gray-700">
          {t('community.hub.eventType')}
        </label>
        <select
          id="event-type"
          value={eventType}
          onChange={(e) => setEventType(e.target.value as CommunityEventType)}
          className={inputClass}
        >
          {EVENT_TYPES.map((et) => (
            <option key={et} value={et}>
              {t(`community.eventTypes.${et}`)}
            </option>
          ))}
        </select>
      </div>
      <div>
        <label htmlFor="event-description" className="block text-sm font-medium text-gray-700">
          {t('community.eventForm.descriptionLabel')}
        </label>
        <textarea
          id="event-description"
          value={description}
          onChange={(e) => setDescription(e.target.value)}
          rows={4}
          placeholder={t('community.eventForm.descriptionPlaceholder')}
          className={inputClass}
        />
      </div>

      <fieldset className="space-y-3">
        <legend className="text-sm font-medium text-gray-700">
          {t('community.eventForm.dateTime')}
        </legend>
        <label className="flex items-center gap-2 text-sm text-gray-600">
          <input
            type="checkbox"
            checked={allDay}
            onChange={(e) => {
              setAllDay(e.target.checked);
              setStart('');
              setEnd('');
            }}
            className="rounded border-gray-300"
          />
          {t('community.eventForm.allDay')}
        </label>
        <div className="grid gap-3 sm:grid-cols-2">
          <div>
            <label htmlFor="event-start" className="block text-sm text-gray-700">
              {t('community.eventForm.start')} *
            </label>
            <input
              id="event-start"
              type={allDay ? 'date' : 'datetime-local'}
              value={start}
              onChange={(e) => setStart(e.target.value)}
              className={inputClass}
            />
            {err('start')}
          </div>
          <div>
            <label htmlFor="event-end" className="block text-sm text-gray-700">
              {t('community.eventForm.end')} *
            </label>
            <input
              id="event-end"
              type={allDay ? 'date' : 'datetime-local'}
              value={end}
              onChange={(e) => setEnd(e.target.value)}
              className={inputClass}
            />
            {err('end')}
          </div>
        </div>
      </fieldset>

      <div className="grid gap-3 sm:grid-cols-2">
        <div>
          <label htmlFor="event-location" className="block text-sm font-medium text-gray-700">
            {t('community.eventForm.locationLabel')}
          </label>
          <input
            id="event-location"
            value={location}
            onChange={(e) => setLocation(e.target.value)}
            placeholder={t('community.eventForm.locationPlaceholder')}
            className={inputClass}
          />
        </div>
        <div>
          <label
            htmlFor="event-location-details"
            className="block text-sm font-medium text-gray-700"
          >
            {t('community.eventForm.locationDetailsLabel')}
          </label>
          <input
            id="event-location-details"
            value={locationDetails}
            onChange={(e) => setLocationDetails(e.target.value)}
            placeholder={t('community.eventForm.locationDetailsPlaceholder')}
            className={inputClass}
          />
        </div>
      </div>

      <div className="space-y-2">
        <label className="flex items-center gap-2 text-sm text-gray-600">
          <input
            type="checkbox"
            checked={isVirtual}
            onChange={(e) => setIsVirtual(e.target.checked)}
            className="rounded border-gray-300"
          />
          {t('community.hub.isVirtual')}
        </label>
        {isVirtual && (
          <div>
            <label htmlFor="event-link" className="block text-sm text-gray-700">
              {t('community.hub.virtualLink')}
            </label>
            <input
              id="event-link"
              type="url"
              value={virtualLink}
              onChange={(e) => setVirtualLink(e.target.value)}
              placeholder="https://"
              className={inputClass}
            />
            {err('virtualLink')}
          </div>
        )}
      </div>

      <div className="grid gap-3 sm:grid-cols-2">
        <div>
          <label htmlFor="event-max" className="block text-sm font-medium text-gray-700">
            {t('community.eventForm.maxAttendeesLabel')}
          </label>
          <input
            id="event-max"
            type="number"
            min={1}
            step={1}
            value={maxAttendees}
            onChange={(e) => setMaxAttendees(e.target.value)}
            placeholder={t('community.eventForm.maxAttendeesPlaceholder')}
            className={inputClass}
          />
          {err('maxAttendees')}
        </div>
        <div>
          <label htmlFor="event-cost" className="block text-sm font-medium text-gray-700">
            {t('community.hub.costPerPerson')}
          </label>
          <input
            id="event-cost"
            inputMode="decimal"
            value={cost}
            onChange={(e) => setCost(e.target.value)}
            placeholder="0.00"
            className={inputClass}
          />
          {err('cost')}
        </div>
      </div>

      <div className="flex justify-end gap-3 pt-2">
        <button
          type="button"
          onClick={() => navigate(withBuilding('/community/events', buildingId))}
          className="px-4 py-2 text-sm font-medium text-gray-700 border border-gray-300 rounded-md hover:bg-gray-50"
        >
          {t('community.common.cancel')}
        </button>
        <button
          type="submit"
          disabled={createEvent.isPending}
          className="px-4 py-2 text-sm font-medium bg-blue-600 text-white rounded-md hover:bg-blue-700 disabled:opacity-50"
        >
          {createEvent.isPending
            ? t('community.eventForm.creating')
            : t('community.eventForm.create')}
        </button>
      </div>
    </form>
  );
}
