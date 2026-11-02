/**
 * EventsPage
 *
 * Page for browsing community events.
 * Part of Story 42.3: Community Events.
 */

import type { CommunityEvent, EventStatus, ListEventsParams, RsvpStatus } from '@ppt/api-client';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { EventCard } from '../components/EventCard';

interface EventsPageProps {
  events: CommunityEvent[];
  total: number;
  isLoading?: boolean;
  currentUserId?: string;
  rsvpingEventId?: string;
  onNavigateToCreate: () => void;
  onNavigateToEvent: (id: string) => void;
  onRsvp: (id: string, status: RsvpStatus) => void;
  onEditEvent: (id: string) => void;
  onDeleteEvent: (id: string) => void;
  onExportCalendar: (id: string) => void;
  onFilterChange: (params: ListEventsParams) => void;
}

type TimeFilter = 'upcoming' | 'past' | 'all';

const timeFilters: { value: TimeFilter; key: string }[] = [
  { value: 'upcoming', key: 'upcoming' },
  { value: 'past', key: 'past' },
  { value: 'all', key: 'all' },
];

const statusFilters: { value: EventStatus | ''; key: string }[] = [
  { value: '', key: 'all' },
  { value: 'published', key: 'published' },
  { value: 'draft', key: 'draft' },
  { value: 'cancelled', key: 'cancelled' },
  { value: 'completed', key: 'completed' },
];

const rsvpFilters: { value: RsvpStatus | ''; key: string }[] = [
  { value: '', key: 'all' },
  { value: 'going', key: 'going' },
  { value: 'maybe', key: 'maybe' },
  { value: 'not_going', key: 'notGoing' },
];

export function EventsPage({
  events,
  total,
  isLoading,
  currentUserId,
  rsvpingEventId,
  onNavigateToCreate,
  onNavigateToEvent,
  onRsvp,
  onEditEvent,
  onDeleteEvent,
  onExportCalendar,
  onFilterChange,
}: EventsPageProps) {
  const { t } = useTranslation();
  const [page, setPage] = useState(1);
  const [pageSize] = useState(12);
  const [timeFilter, setTimeFilter] = useState<TimeFilter>('upcoming');
  const [statusFilter, setStatusFilter] = useState<EventStatus | ''>('');
  const [rsvpFilter, setRsvpFilter] = useState<RsvpStatus | ''>('');

  const totalPages = Math.ceil(total / pageSize);

  const handleTimeFilterChange = (filter: TimeFilter) => {
    setTimeFilter(filter);
    const now = new Date().toISOString();
    const params: ListEventsParams = { page: 1, pageSize };

    if (filter === 'upcoming') {
      params.fromDate = now;
    } else if (filter === 'past') {
      params.toDate = now;
    }

    if (statusFilter) params.status = statusFilter;
    if (rsvpFilter) params.userRsvp = rsvpFilter;

    setPage(1);
    onFilterChange(params);
  };

  const handleStatusFilterChange = (status: EventStatus | '') => {
    setStatusFilter(status);
    const now = new Date().toISOString();
    const params: ListEventsParams = { page: 1, pageSize };

    if (timeFilter === 'upcoming') params.fromDate = now;
    if (timeFilter === 'past') params.toDate = now;
    if (status) params.status = status;
    if (rsvpFilter) params.userRsvp = rsvpFilter;

    setPage(1);
    onFilterChange(params);
  };

  const handleRsvpFilterChange = (rsvp: RsvpStatus | '') => {
    setRsvpFilter(rsvp);
    const now = new Date().toISOString();
    const params: ListEventsParams = { page: 1, pageSize };

    if (timeFilter === 'upcoming') params.fromDate = now;
    if (timeFilter === 'past') params.toDate = now;
    if (statusFilter) params.status = statusFilter;
    if (rsvp) params.userRsvp = rsvp;

    setPage(1);
    onFilterChange(params);
  };

  const handlePageChange = (newPage: number) => {
    setPage(newPage);
    const now = new Date().toISOString();
    const params: ListEventsParams = { page: newPage, pageSize };

    if (timeFilter === 'upcoming') params.fromDate = now;
    if (timeFilter === 'past') params.toDate = now;
    if (statusFilter) params.status = statusFilter;
    if (rsvpFilter) params.userRsvp = rsvpFilter;

    onFilterChange(params);
  };

  return (
    <div className="max-w-7xl mx-auto px-4 py-8">
      {/* Header */}
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">{t('community.events.title')}</h1>
          <p className="mt-1 text-sm text-gray-500">{t('community.events.subtitle')}</p>
        </div>
        <button
          type="button"
          onClick={onNavigateToCreate}
          className="inline-flex items-center px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700"
        >
          <svg
            className="w-5 h-5 mr-2"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
            aria-hidden="true"
          >
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4v16m8-8H4" />
          </svg>
          {t('community.events.create')}
        </button>
      </div>

      {/* Filters */}
      <div className="bg-white rounded-lg shadow p-4 mb-6">
        <div className="flex flex-wrap items-center gap-4">
          {/* Time Filter */}
          <div className="flex items-center gap-1 border rounded-lg p-1">
            {timeFilters.map((filter) => (
              <button
                key={filter.value}
                type="button"
                onClick={() => handleTimeFilterChange(filter.value)}
                className={`px-3 py-1 text-sm rounded-md transition-colors ${
                  timeFilter === filter.value
                    ? 'bg-blue-600 text-white'
                    : 'text-gray-600 hover:bg-gray-100'
                }`}
              >
                {t(`community.events.time.${filter.key}`)}
              </button>
            ))}
          </div>

          {/* Status Filter */}
          <select
            value={statusFilter}
            onChange={(e) => handleStatusFilterChange(e.target.value as EventStatus | '')}
            className="px-3 py-1 text-sm border border-gray-300 rounded-md focus:ring-blue-500 focus:border-blue-500"
          >
            {statusFilters.map((status) => (
              <option key={status.value} value={status.value}>
                {t(`community.events.status.${status.key}`)}
              </option>
            ))}
          </select>

          {/* RSVP Filter */}
          <select
            value={rsvpFilter}
            onChange={(e) => handleRsvpFilterChange(e.target.value as RsvpStatus | '')}
            className="px-3 py-1 text-sm border border-gray-300 rounded-md focus:ring-blue-500 focus:border-blue-500"
          >
            {rsvpFilters.map((rsvp) => (
              <option key={rsvp.value} value={rsvp.value}>
                {t(`community.events.rsvp.${rsvp.key}`)}
              </option>
            ))}
          </select>

          <span className="ml-auto text-sm text-gray-500">
            {total} {total === 1 ? t('community.events.event') : t('community.events.events')}
          </span>
        </div>
      </div>

      {/* Events Grid */}
      {isLoading ? (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
          {['skeleton-1', 'skeleton-2', 'skeleton-3', 'skeleton-4', 'skeleton-5', 'skeleton-6'].map(
            (key) => (
              <div key={key} className="bg-white rounded-lg shadow animate-pulse">
                <div className="h-36 bg-gray-200" />
                <div className="p-4 space-y-3">
                  <div className="h-3 bg-gray-200 rounded w-1/4" />
                  <div className="h-5 bg-gray-200 rounded w-3/4" />
                  <div className="h-4 bg-gray-200 rounded w-1/2" />
                  <div className="h-4 bg-gray-200 rounded w-1/3" />
                </div>
              </div>
            )
          )}
        </div>
      ) : events.length === 0 ? (
        <div className="bg-white rounded-lg shadow p-8 text-center">
          <svg
            className="mx-auto h-12 w-12 text-gray-400"
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
          <h3 className="mt-4 text-lg font-medium text-gray-900">
            {t('community.events.emptyTitle')}
          </h3>
          <p className="mt-2 text-sm text-gray-500">
            {timeFilter === 'upcoming'
              ? t('community.events.emptyBodyUpcoming')
              : t('community.events.emptyBodyDefault')}
          </p>
          <button
            type="button"
            onClick={onNavigateToCreate}
            className="mt-4 inline-flex items-center px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700"
          >
            {t('community.events.create')}
          </button>
        </div>
      ) : (
        <>
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
            {events.map((event) => (
              <EventCard
                key={event.id}
                event={event}
                isCurrentUserOrganizer={event.createdBy === currentUserId}
                isRsvping={rsvpingEventId === event.id}
                onView={onNavigateToEvent}
                onRsvp={onRsvp}
                onEdit={onEditEvent}
                onDelete={onDeleteEvent}
                onExportCalendar={onExportCalendar}
              />
            ))}
          </div>

          {/* Pagination */}
          {totalPages > 1 && (
            <div className="flex items-center justify-center gap-2 mt-8">
              <button
                type="button"
                onClick={() => handlePageChange(page - 1)}
                disabled={page === 1}
                className="px-3 py-1 text-sm border rounded-md disabled:opacity-50 disabled:cursor-not-allowed hover:bg-gray-50"
              >
                {t('community.common.previous')}
              </button>
              <span className="text-sm text-gray-600">
                {t('community.common.pageOf', { page, total: totalPages })}
              </span>
              <button
                type="button"
                onClick={() => handlePageChange(page + 1)}
                disabled={page === totalPages}
                className="px-3 py-1 text-sm border rounded-md disabled:opacity-50 disabled:cursor-not-allowed hover:bg-gray-50"
              >
                {t('community.common.next')}
              </button>
            </div>
          )}
        </>
      )}
    </div>
  );
}
