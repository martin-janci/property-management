/**
 * Facility List Component (Epic 56: Facility Booking).
 *
 * Displays a list of facilities with filtering and pagination.
 */

import type { FacilitySummary, FacilityType } from '@ppt/api-client';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { FacilityCard } from './FacilityCard';
import { FACILITY_TYPES } from './facilityTypes';

interface FacilityListProps {
  facilities: FacilitySummary[];
  total: number;
  page: number;
  pageSize: number;
  isLoading?: boolean;
  /** True when the facilities fetch failed. Renders a distinct error state
   * (with retry) instead of the empty "No facilities found" placeholder. */
  isError?: boolean;
  isManager?: boolean;
  onPageChange: (page: number) => void;
  /** Re-run the facilities fetch (wired to the error state's retry button). */
  onRetry?: () => void;
  onTypeFilter: (type?: FacilityType) => void;
  onBookableFilter: (bookable?: boolean) => void;
  /** Optional — the detail button is hidden when no handler is passed. */
  onView?: (id: string) => void;
  onBook: (id: string) => void;
  onEdit?: (id: string) => void;
  onCreate?: () => void;
}

export function FacilityList({
  facilities,
  total,
  page,
  pageSize,
  isLoading,
  isError,
  isManager,
  onPageChange,
  onRetry,
  onTypeFilter,
  onBookableFilter,
  onView,
  onBook,
  onEdit,
  onCreate,
}: FacilityListProps) {
  const { t } = useTranslation();
  const [typeFilter, setTypeFilter] = useState<FacilityType | ''>('');
  const [bookableFilter, setBookableFilter] = useState<string>('');

  const totalPages = Math.ceil(total / pageSize);

  const handleTypeChange = (value: string) => {
    setTypeFilter(value as FacilityType | '');
    onTypeFilter(value ? (value as FacilityType) : undefined);
  };

  const handleBookableChange = (value: string) => {
    setBookableFilter(value);
    if (value === '') {
      onBookableFilter(undefined);
    } else {
      onBookableFilter(value === 'true');
    }
  };

  return (
    <div>
      {/* Header */}
      <div className="mb-6 flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4">
        <h1 className="text-2xl font-bold text-gray-900">{t('facilities.title')}</h1>
        {isManager && onCreate && (
          <button
            type="button"
            onClick={onCreate}
            className="px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 transition-colors"
          >
            {t('facilities.addFacility')}
          </button>
        )}
      </div>

      {/* Filters */}
      <div className="mb-6 flex flex-wrap gap-4">
        <select
          aria-label={t('facilities.filters.typeLabel')}
          value={typeFilter}
          onChange={(e) => handleTypeChange(e.target.value)}
          className="px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
        >
          <option value="">{t('facilities.filters.allTypes')}</option>
          {FACILITY_TYPES.map((type) => (
            <option key={type} value={type}>
              {t(`facilities.types.${type}`)}
            </option>
          ))}
        </select>

        <select
          aria-label={t('facilities.filters.bookableLabel')}
          value={bookableFilter}
          onChange={(e) => handleBookableChange(e.target.value)}
          className="px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
        >
          <option value="">{t('facilities.filters.allFacilities')}</option>
          <option value="true">{t('facilities.filters.bookableOnly')}</option>
          <option value="false">{t('facilities.filters.nonBookable')}</option>
        </select>
      </div>

      {/* List */}
      {isLoading ? (
        <div className="flex items-center justify-center py-12">
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600" />
        </div>
      ) : isError ? (
        <div className="text-center py-12" role="alert">
          <svg
            className="mx-auto h-12 w-12 text-red-400"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
            aria-hidden="true"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"
            />
          </svg>
          <p className="mt-4 text-gray-900 font-medium">
            {t('facilities.errors.fetchFailedTitle')}
          </p>
          <p className="mt-1 text-gray-500">{t('facilities.errors.fetchFailedDescription')}</p>
          {onRetry && (
            <button
              type="button"
              onClick={onRetry}
              className="mt-4 px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 transition-colors"
            >
              {t('facilities.errors.retry')}
            </button>
          )}
        </div>
      ) : facilities.length === 0 ? (
        <div className="text-center py-12 text-gray-500">
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
              d="M19 21V5a2 2 0 00-2-2H7a2 2 0 00-2 2v16m14 0h2m-2 0h-5m-9 0H3m2 0h5M9 7h1m-1 4h1m4-4h1m-1 4h1m-5 10v-5a1 1 0 011-1h2a1 1 0 011 1v5m-4 0h4"
            />
          </svg>
          <p className="mt-4">{t('facilities.empty')}</p>
          {isManager && onCreate && (
            <button
              type="button"
              onClick={onCreate}
              className="mt-4 text-blue-600 hover:text-blue-800"
            >
              {t('facilities.addFirstFacility')}
            </button>
          )}
        </div>
      ) : (
        <div className="grid gap-4 md:grid-cols-2">
          {facilities.map((facility) => (
            <FacilityCard
              key={facility.id}
              facility={facility}
              onView={onView}
              onBook={onBook}
              onEdit={onEdit}
              isManager={isManager}
            />
          ))}
        </div>
      )}

      {/* Pagination */}
      {totalPages > 1 && (
        <div className="mt-6 flex items-center justify-between">
          <p className="text-sm text-gray-500">
            {t('facilities.pagination.showing', {
              from: (page - 1) * pageSize + 1,
              to: Math.min(page * pageSize, total),
              total,
            })}
          </p>
          <div className="flex gap-2">
            <button
              type="button"
              onClick={() => onPageChange(page - 1)}
              disabled={page === 1}
              className="px-3 py-1 border rounded-md disabled:opacity-50 disabled:cursor-not-allowed hover:bg-gray-50"
            >
              {t('facilities.pagination.previous')}
            </button>
            <span className="px-3 py-1">
              {t('facilities.pagination.page', { page, totalPages })}
            </span>
            <button
              type="button"
              onClick={() => onPageChange(page + 1)}
              disabled={page === totalPages}
              className="px-3 py-1 border rounded-md disabled:opacity-50 disabled:cursor-not-allowed hover:bg-gray-50"
            >
              {t('facilities.pagination.next')}
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
