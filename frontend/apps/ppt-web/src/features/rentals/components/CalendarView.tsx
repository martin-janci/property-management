/**
 * CalendarView Component
 *
 * Month grid (Monday first) showing a unit's reservations and blocked dates.
 * Month and weekday names come from `Intl` for the current UI language.
 * Epic 18: Short-Term Rental Integration (Story 18.2)
 */

import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { eventCoversDay, getMonthGridDays } from '../mappers';
import type { CalendarEvent, CalendarEventType } from '../types';

interface CalendarViewProps {
  events: CalendarEvent[];
  /** Visible month. When omitted the view keeps its own month state. */
  currentMonth?: Date;
  isLoading?: boolean;
  onDateSelect?: (date: Date) => void;
  onEventClick?: (event: CalendarEvent) => void;
  onMonthChange?: (date: Date) => void;
  /** Drag across days to select a range; receives the first and last selected day. */
  onAddBlock?: (startDate: Date, endDate: Date) => void;
}

const eventTypeColors: Record<CalendarEventType, { bg: string; border: string; text: string }> = {
  booking: { bg: 'bg-blue-100', border: 'border-blue-400', text: 'text-blue-800' },
  block: { bg: 'bg-gray-200', border: 'border-gray-400', text: 'text-gray-700' },
  maintenance: { bg: 'bg-orange-100', border: 'border-orange-400', text: 'text-orange-800' },
};

function isSameDay(d1: Date, d2: Date): boolean {
  return (
    d1.getFullYear() === d2.getFullYear() &&
    d1.getMonth() === d2.getMonth() &&
    d1.getDate() === d2.getDate()
  );
}

function orderRange(a: Date, b: Date): [Date, Date] {
  return a <= b ? [a, b] : [b, a];
}

export function CalendarView({
  events,
  currentMonth,
  isLoading,
  onDateSelect,
  onEventClick,
  onMonthChange,
  onAddBlock,
}: CalendarViewProps) {
  const { t, i18n } = useTranslation();
  const [internalMonth, setInternalMonth] = useState(() => currentMonth ?? new Date());
  const viewDate = currentMonth ?? internalMonth;
  const [selectionStart, setSelectionStart] = useState<Date | null>(null);
  const [selectionEnd, setSelectionEnd] = useState<Date | null>(null);
  const [isSelecting, setIsSelecting] = useState(false);

  const year = viewDate.getFullYear();
  const month = viewDate.getMonth();
  const days = getMonthGridDays(viewDate);
  const today = new Date();
  const language = i18n.language || 'en';

  const monthTitle = new Intl.DateTimeFormat(language, { month: 'long', year: 'numeric' }).format(
    new Date(year, month, 1)
  );
  const weekdayFormat = new Intl.DateTimeFormat(language, { weekday: 'short' });
  const weekDays = days.slice(0, 7).map((d) => weekdayFormat.format(d));
  const dayLabelFormat = new Intl.DateTimeFormat(language, { dateStyle: 'full' });

  const changeMonth = (next: Date) => {
    setInternalMonth(next);
    onMonthChange?.(next);
  };

  const eventLabel = (event: CalendarEvent): string =>
    event.type === 'booking'
      ? event.title
      : t(`rentals.calendar.blockReasons.${event.blockReason ?? 'blocked'}`);

  const handleDateMouseDown = (date: Date) => {
    if (onAddBlock) {
      setIsSelecting(true);
      setSelectionStart(date);
      setSelectionEnd(date);
    }
    onDateSelect?.(date);
  };

  const handleDateMouseEnter = (date: Date) => {
    if (isSelecting && selectionStart) {
      setSelectionEnd(date);
    }
  };

  const handleMouseUp = () => {
    if (isSelecting && selectionStart && selectionEnd && onAddBlock) {
      const [start, end] = orderRange(selectionStart, selectionEnd);
      onAddBlock(start, end);
    }
    setIsSelecting(false);
    setSelectionStart(null);
    setSelectionEnd(null);
  };

  const isDateSelected = (date: Date): boolean => {
    if (!selectionStart || !selectionEnd) return false;
    const [start, end] = orderRange(selectionStart, selectionEnd);
    return date >= start && date <= end;
  };

  return (
    <div
      className="bg-white rounded-lg shadow"
      onMouseUp={handleMouseUp}
      onMouseLeave={handleMouseUp}
    >
      {/* Header */}
      <div className="flex items-center justify-between px-4 sm:px-6 py-4 border-b">
        <div className="flex items-center gap-4">
          <h2 className="text-lg font-semibold text-gray-900 first-letter:uppercase">
            {monthTitle}
          </h2>
          <button
            type="button"
            onClick={() => changeMonth(new Date())}
            className="text-sm text-blue-600 hover:text-blue-800"
          >
            {t('rentals.calendar.today')}
          </button>
        </div>
        <div className="flex items-center gap-2">
          <button
            type="button"
            onClick={() => changeMonth(new Date(year, month - 1, 1))}
            className="p-2 hover:bg-gray-100 rounded-lg"
            aria-label={t('rentals.calendar.previousMonth')}
          >
            <svg
              className="w-5 h-5 text-gray-600"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
              aria-hidden="true"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M15 19l-7-7 7-7"
              />
            </svg>
          </button>
          <button
            type="button"
            onClick={() => changeMonth(new Date(year, month + 1, 1))}
            className="p-2 hover:bg-gray-100 rounded-lg"
            aria-label={t('rentals.calendar.nextMonth')}
          >
            <svg
              className="w-5 h-5 text-gray-600"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
              aria-hidden="true"
            >
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5l7 7-7 7" />
            </svg>
          </button>
        </div>
      </div>

      {/* Legend */}
      <div className="px-4 sm:px-6 py-2 border-b flex flex-wrap items-center gap-4 text-xs">
        <div className="flex items-center gap-1">
          <span className="w-3 h-3 rounded bg-blue-100 border border-blue-400" />
          <span className="text-gray-600">{t('rentals.calendar.booking')}</span>
        </div>
        <div className="flex items-center gap-1">
          <span className="w-3 h-3 rounded bg-gray-200 border border-gray-400" />
          <span className="text-gray-600">{t('rentals.calendar.blocked')}</span>
        </div>
        <div className="flex items-center gap-1">
          <span className="w-3 h-3 rounded bg-orange-100 border border-orange-400" />
          <span className="text-gray-600">{t('rentals.calendar.maintenance')}</span>
        </div>
      </div>

      {/* Calendar Grid */}
      <div className="p-2 sm:p-4">
        {isLoading ? (
          <div className="h-96 flex items-center justify-center" data-testid="calendar-loading">
            <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600" />
          </div>
        ) : (
          <>
            {/* Week day headers */}
            <div className="grid grid-cols-7 mb-2">
              {weekDays.map((day) => (
                <div key={day} className="text-center text-xs font-medium text-gray-500 py-2">
                  {day}
                </div>
              ))}
            </div>

            {/* Days grid */}
            <div className="grid grid-cols-7 gap-1">
              {days.map((date) => {
                const isCurrentMonth = date.getMonth() === month;
                const isToday = isSameDay(date, today);
                const dateEvents = events.filter((event) => eventCoversDay(event, date));
                const isSelected = isDateSelected(date);

                return (
                  <div
                    key={date.getTime()}
                    data-testid="calendar-day"
                    title={dayLabelFormat.format(date)}
                    className={`
                      min-h-[72px] sm:min-h-[100px] p-1 border rounded transition-colors select-none
                      ${onAddBlock ? 'cursor-pointer' : ''}
                      ${isCurrentMonth ? 'bg-white' : 'bg-gray-50'}
                      ${isToday ? 'border-blue-500 border-2' : 'border-gray-200'}
                      ${isSelected ? 'bg-blue-50' : ''}
                      hover:bg-gray-50
                    `}
                    onMouseDown={() => handleDateMouseDown(date)}
                    onMouseEnter={() => handleDateMouseEnter(date)}
                  >
                    <div
                      className={`
                        text-sm font-medium mb-1
                        ${isCurrentMonth ? 'text-gray-900' : 'text-gray-400'}
                        ${isToday ? 'text-blue-600' : ''}
                      `}
                    >
                      {date.getDate()}
                    </div>
                    <div className="space-y-1">
                      {dateEvents.slice(0, 3).map((event) => {
                        const colors = eventTypeColors[event.type];
                        const label = eventLabel(event);
                        return (
                          <button
                            key={event.id}
                            type="button"
                            onMouseDown={(e) => e.stopPropagation()}
                            onClick={(e) => {
                              e.stopPropagation();
                              onEventClick?.(event);
                            }}
                            className={`
                              w-full text-left text-xs px-1 py-0.5 rounded truncate
                              ${colors.bg} ${colors.border} ${colors.text}
                              border-l-2 hover:opacity-80
                            `}
                            title={label}
                          >
                            {label}
                          </button>
                        );
                      })}
                      {dateEvents.length > 3 && (
                        <div className="text-xs text-gray-500 px-1">
                          +{dateEvents.length - 3} {t('rentals.calendar.more')}
                        </div>
                      )}
                    </div>
                  </div>
                );
              })}
            </div>
          </>
        )}
      </div>
    </div>
  );
}
