/**
 * CalendarPage - Availability calendar for one rental unit.
 * Epic 18: Short-Term Rental Integration (Story 18.2)
 *
 * Shows the unit's reservations and blocked dates for the visible month
 * (`GET /rentals/calendar/{unit_id}`), lets the manager block a date range
 * (`POST /rentals/calendar/blocks`) and remove a manual block
 * (`DELETE /rentals/calendar/blocks/{id}`). Clicking a reservation opens its
 * detail. All data comes in through props from the route wrapper.
 */

import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { CalendarView } from '../components/CalendarView';
import { addDaysIso, parseIsoDate, toIsoDate } from '../mappers';
import type { CalendarBlockReason, CalendarEvent, RentalUnitOption } from '../types';

export interface CalendarBlockInput {
  unitId: string;
  /** ISO date, first blocked day. */
  startDate: string;
  /** ISO date, last blocked day (inclusive). */
  endDate: string;
  reason: CalendarBlockReason;
  notes?: string;
}

interface CalendarPageProps {
  units: RentalUnitOption[];
  isLoadingUnits?: boolean;
  isUnitsError?: boolean;
  events: CalendarEvent[];
  selectedUnitId?: string;
  /** Visible month (any day in it). */
  month: Date;
  /** Calendar events are loading. */
  isLoading?: boolean;
  isError?: boolean;
  isSavingBlock?: boolean;
  onRetry?: () => void;
  onUnitChange: (unitId: string) => void;
  onMonthChange: (date: Date) => void;
  /** Reservation clicked. */
  onEventClick: (event: CalendarEvent) => void;
  onAddBlock: (input: CalendarBlockInput) => void;
  onDeleteBlock: (eventId: string) => void;
  onBack: () => void;
}

const BLOCK_REASONS: CalendarBlockReason[] = ['owner_use', 'maintenance', 'blocked'];

interface BlockDraft {
  startDate: string;
  endDate: string;
  reason: CalendarBlockReason;
  notes: string;
}

export function CalendarPage({
  units,
  isLoadingUnits,
  isUnitsError,
  events,
  selectedUnitId,
  month,
  isLoading,
  isError,
  isSavingBlock,
  onRetry,
  onUnitChange,
  onMonthChange,
  onEventClick,
  onAddBlock,
  onDeleteBlock,
  onBack,
}: CalendarPageProps) {
  const { t, i18n } = useTranslation();
  const [draft, setDraft] = useState<BlockDraft | null>(null);
  const [openBlock, setOpenBlock] = useState<CalendarEvent | null>(null);

  const language = i18n.language || 'en';
  const dateFormat = new Intl.DateTimeFormat(language, {
    weekday: 'short',
    day: 'numeric',
    month: 'short',
    year: 'numeric',
  });
  const formatIso = (iso: string) => dateFormat.format(parseIsoDate(iso));

  const selectedUnit = units.find((u) => u.id === selectedUnitId);
  const bookingCount = events.filter((e) => e.type === 'booking').length;
  const blockCount = events.length - bookingCount;

  const openDraft = (start: Date, end: Date) => {
    if (!selectedUnitId) return;
    setDraft({
      startDate: toIsoDate(start),
      endDate: toIsoDate(end),
      reason: 'owner_use',
      notes: '',
    });
  };

  const draftInvalid =
    !!draft && (!draft.startDate || !draft.endDate || draft.endDate < draft.startDate);
  const draftDays =
    draft && !draftInvalid
      ? Math.round(
          (parseIsoDate(draft.endDate).getTime() - parseIsoDate(draft.startDate).getTime()) /
            86_400_000
        ) + 1
      : 0;

  const confirmDraft = () => {
    if (!selectedUnitId || !draft || draftInvalid) return;
    onAddBlock({
      unitId: selectedUnitId,
      startDate: draft.startDate,
      endDate: draft.endDate,
      reason: draft.reason,
      notes: draft.notes.trim() || undefined,
    });
    setDraft(null);
  };

  const handleEventClick = (event: CalendarEvent) => {
    if (event.type === 'booking') onEventClick(event);
    else setOpenBlock(event);
  };

  const blockLabel = (event: CalendarEvent) =>
    t(`rentals.calendar.blockReasons.${event.blockReason ?? 'blocked'}`);

  const renderBody = () => {
    if (isLoadingUnits) {
      return (
        <div
          className="bg-white rounded-lg shadow p-12 flex justify-center"
          data-testid="units-loading"
        >
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600" />
        </div>
      );
    }
    if (isUnitsError) {
      return (
        <div className="bg-white rounded-lg shadow p-12 text-center">
          <h2 className="text-lg font-medium text-gray-900">
            {t('rentals.calendar.unitsLoadFailed')}
          </h2>
          {onRetry && (
            <button
              type="button"
              onClick={onRetry}
              className="mt-4 px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700"
            >
              {t('common.retry')}
            </button>
          )}
        </div>
      );
    }
    if (units.length === 0) {
      return (
        <div className="bg-white rounded-lg shadow p-12 text-center">
          <h2 className="text-lg font-medium text-gray-900">{t('rentals.calendar.noUnits')}</h2>
          <p className="mt-2 text-sm text-gray-500">{t('rentals.calendar.noUnitsDesc')}</p>
        </div>
      );
    }
    if (!selectedUnitId) {
      return (
        <div className="bg-white rounded-lg shadow p-12 text-center">
          <svg
            className="w-16 h-16 mx-auto text-gray-400"
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
          <h2 className="mt-4 text-lg font-medium text-gray-900">
            {t('rentals.calendar.selectUnitPrompt')}
          </h2>
          <p className="mt-2 text-sm text-gray-500">{t('rentals.calendar.selectUnitDesc')}</p>
        </div>
      );
    }
    return (
      <>
        <div className="mb-6 flex flex-wrap items-center justify-between gap-4">
          <div>
            <h2 className="text-lg font-semibold text-gray-900">{selectedUnit?.name}</h2>
            <p className="text-sm text-gray-500">{selectedUnit?.buildingName}</p>
          </div>
          <div className="flex flex-wrap items-center gap-4 text-sm">
            <span className="flex items-center gap-2 text-gray-600" data-testid="booking-count">
              <span className="w-3 h-3 rounded bg-blue-100 border border-blue-400" />
              {t('rentals.calendar.bookingsCount', { count: bookingCount })}
            </span>
            <span className="flex items-center gap-2 text-gray-600" data-testid="block-count">
              <span className="w-3 h-3 rounded bg-gray-200 border border-gray-400" />
              {t('rentals.calendar.blocksCount', { count: blockCount })}
            </span>
            <button
              type="button"
              onClick={() => {
                const today = new Date();
                openDraft(today, today);
              }}
              className="px-3 py-2 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700"
            >
              {t('rentals.calendar.blockDatesButton')}
            </button>
          </div>
        </div>

        {isError ? (
          <div className="bg-white rounded-lg shadow p-12 text-center" role="alert">
            <h2 className="text-lg font-medium text-gray-900">
              {t('rentals.calendar.loadFailed')}
            </h2>
            {onRetry && (
              <button
                type="button"
                onClick={onRetry}
                className="mt-4 px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700"
              >
                {t('common.retry')}
              </button>
            )}
          </div>
        ) : (
          <CalendarView
            events={events}
            currentMonth={month}
            isLoading={isLoading}
            onEventClick={handleEventClick}
            onMonthChange={onMonthChange}
            onAddBlock={openDraft}
          />
        )}

        {!isLoading && !isError && events.length === 0 && (
          <p className="mt-4 text-sm text-gray-500 text-center">{t('rentals.calendar.noEvents')}</p>
        )}

        <div className="mt-6 bg-blue-50 border border-blue-200 rounded-lg p-4">
          <p className="text-sm font-medium text-blue-800">{t('rentals.calendar.tip')}</p>
          <p className="mt-1 text-sm text-blue-700">{t('rentals.calendar.tipDesc')}</p>
        </div>
      </>
    );
  };

  return (
    <div className="min-h-screen bg-gray-100">
      {/* Header */}
      <div className="bg-white shadow">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-6">
          <div className="flex flex-wrap items-center justify-between gap-4">
            <div className="flex items-center gap-4">
              <button
                type="button"
                onClick={onBack}
                className="text-gray-500 hover:text-gray-700"
                aria-label={t('common.back')}
              >
                <svg
                  className="w-6 h-6"
                  fill="none"
                  viewBox="0 0 24 24"
                  stroke="currentColor"
                  aria-hidden="true"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M10 19l-7-7m0 0l7-7m-7 7h18"
                  />
                </svg>
              </button>
              <div>
                <h1 className="text-2xl font-bold text-gray-900">{t('rentals.calendar.title')}</h1>
                <p className="mt-1 text-sm text-gray-500">{t('rentals.calendar.subtitle')}</p>
              </div>
            </div>
            {units.length > 0 && (
              <div>
                <label htmlFor="unit-select" className="sr-only">
                  {t('rentals.calendar.selectUnit')}
                </label>
                <select
                  id="unit-select"
                  value={selectedUnitId || ''}
                  onChange={(e) => onUnitChange(e.target.value)}
                  className="max-w-full px-4 py-2 border border-gray-300 rounded-md text-sm focus:ring-blue-500 focus:border-blue-500"
                >
                  <option value="">{t('rentals.calendar.selectUnit')}</option>
                  {units.map((unit) => (
                    <option key={unit.id} value={unit.id}>
                      {unit.name} – {unit.buildingName}
                    </option>
                  ))}
                </select>
              </div>
            )}
          </div>
        </div>
      </div>

      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">{renderBody()}</div>

      {/* Block dates dialog */}
      {draft && (
        <div className="fixed inset-0 z-50 overflow-y-auto">
          <div className="flex min-h-full items-center justify-center p-4">
            <div className="fixed inset-0 bg-black/25" aria-hidden="true" />
            <div
              className="relative bg-white rounded-lg shadow-xl max-w-md w-full p-6"
              role="dialog"
              aria-modal="true"
              aria-labelledby="block-dialog-title"
            >
              <h3 id="block-dialog-title" className="text-lg font-semibold text-gray-900">
                {t('rentals.calendar.blockDates')}
              </h3>
              <p className="mt-2 text-sm text-gray-500">{t('rentals.calendar.blockDatesDesc')}</p>

              <div className="mt-4 grid grid-cols-2 gap-3">
                <div>
                  <label htmlFor="block-start" className="block text-sm font-medium text-gray-700">
                    {t('rentals.calendar.blockFrom')}
                  </label>
                  <input
                    id="block-start"
                    type="date"
                    value={draft.startDate}
                    onChange={(e) => setDraft({ ...draft, startDate: e.target.value })}
                    className="mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md text-sm"
                  />
                </div>
                <div>
                  <label htmlFor="block-end" className="block text-sm font-medium text-gray-700">
                    {t('rentals.calendar.blockTo')}
                  </label>
                  <input
                    id="block-end"
                    type="date"
                    value={draft.endDate}
                    min={draft.startDate}
                    onChange={(e) => setDraft({ ...draft, endDate: e.target.value })}
                    className="mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md text-sm"
                  />
                </div>
              </div>
              <p className="mt-2 text-xs text-gray-500" data-testid="block-days">
                {draftInvalid
                  ? t('rentals.calendar.invalidRange')
                  : t('rentals.calendar.daysCount', { count: draftDays })}
              </p>

              <div className="mt-4">
                <label htmlFor="block-reason" className="block text-sm font-medium text-gray-700">
                  {t('rentals.calendar.reason')}
                </label>
                <select
                  id="block-reason"
                  value={draft.reason}
                  onChange={(e) =>
                    setDraft({ ...draft, reason: e.target.value as CalendarBlockReason })
                  }
                  className="mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md text-sm"
                >
                  {BLOCK_REASONS.map((r) => (
                    <option key={r} value={r}>
                      {t(`rentals.calendar.blockReasons.${r}`)}
                    </option>
                  ))}
                </select>
              </div>

              <div className="mt-4">
                <label htmlFor="block-notes" className="block text-sm font-medium text-gray-700">
                  {t('rentals.calendar.notes')} ({t('common.optional')})
                </label>
                <textarea
                  id="block-notes"
                  value={draft.notes}
                  onChange={(e) => setDraft({ ...draft, notes: e.target.value })}
                  rows={3}
                  placeholder={t('rentals.calendar.reasonPlaceholder')}
                  className="mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500"
                />
              </div>

              <div className="mt-6 flex items-center justify-end gap-3">
                <button
                  type="button"
                  onClick={() => setDraft(null)}
                  className="px-4 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-md hover:bg-gray-50"
                >
                  {t('common.cancel')}
                </button>
                <button
                  type="button"
                  onClick={confirmDraft}
                  disabled={draftInvalid || isSavingBlock}
                  className="px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700 disabled:opacity-50"
                >
                  {t('rentals.calendar.confirmBlock')}
                </button>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Block detail dialog */}
      {openBlock && (
        <div className="fixed inset-0 z-50 overflow-y-auto">
          <div className="flex min-h-full items-center justify-center p-4">
            <div className="fixed inset-0 bg-black/25" aria-hidden="true" />
            <div
              className="relative bg-white rounded-lg shadow-xl max-w-md w-full p-6"
              role="dialog"
              aria-modal="true"
              aria-labelledby="block-detail-title"
            >
              <h3 id="block-detail-title" className="text-lg font-semibold text-gray-900">
                {blockLabel(openBlock)}
              </h3>
              <p className="mt-2 text-sm text-gray-700">
                {formatIso(openBlock.startDate)} –{' '}
                {formatIso(
                  openBlock.endDate > openBlock.startDate
                    ? addDaysIso(openBlock.endDate, -1)
                    : openBlock.startDate
                )}
              </p>
              <div className="mt-6 flex items-center justify-end gap-3">
                <button
                  type="button"
                  onClick={() => setOpenBlock(null)}
                  className="px-4 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-md hover:bg-gray-50"
                >
                  {t('common.close')}
                </button>
                <button
                  type="button"
                  onClick={() => {
                    onDeleteBlock(openBlock.id);
                    setOpenBlock(null);
                  }}
                  className="px-4 py-2 text-sm font-medium text-white bg-red-600 rounded-md hover:bg-red-700"
                >
                  {t('rentals.calendar.removeBlock')}
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
