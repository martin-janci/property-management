/**
 * Booking List Component (Epic 56: Facility Booking).
 *
 * Displays a list of bookings with filtering and pagination.
 */

import type { BookingStatus, BookingWithDetails } from '@ppt/api-client';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { BookingCard } from './BookingCard';

interface BookingListProps {
  bookings: BookingWithDetails[];
  total: number;
  page: number;
  pageSize: number;
  isLoading?: boolean;
  isManager?: boolean;
  title?: string;
  onPageChange: (page: number) => void;
  onStatusFilter?: (status?: BookingStatus) => void;
  onDateRangeFilter?: (fromDate?: string, toDate?: string) => void;
  onView: (id: string) => void;
  onCancel?: (id: string) => void;
  onApprove?: (id: string) => void;
  onReject?: (id: string) => void;
}

const statusOptions: { value: BookingStatus; label: string }[] = [
  { value: 'pending', label: 'Pending' },
  { value: 'approved', label: 'Approved' },
  { value: 'rejected', label: 'Rejected' },
  { value: 'cancelled', label: 'Cancelled' },
  { value: 'completed', label: 'Completed' },
  { value: 'no_show', label: 'No Show' },
];

export function BookingList({
  bookings,
  total,
  page,
  pageSize,
  isLoading,
  isManager,
  title,
  onPageChange,
  onStatusFilter,
  onDateRangeFilter,
  onView,
  onCancel,
  onApprove,
  onReject,
}: BookingListProps) {
  const { t } = useTranslation();
  const displayTitle = title ?? t('facilities.bookings.listTitle');
  const [statusFilter, setStatusFilter] = useState<BookingStatus | ''>('');
  const [fromDate, setFromDate] = useState('');
  const [toDate, setToDate] = useState('');

  const totalPages = Math.ceil(total / pageSize);

  const handleStatusChange = (value: string) => {
    setStatusFilter(value as BookingStatus | '');
    onStatusFilter?.(value ? (value as BookingStatus) : undefined);
  };

  const handleFromDateChange = (value: string) => {
    setFromDate(value);
    onDateRangeFilter?.(value || undefined, toDate || undefined);
  };

  const handleToDateChange = (value: string) => {
    setToDate(value);
    onDateRangeFilter?.(fromDate || undefined, value || undefined);
  };

  return (
    <div>
      {/* Header */}
      <div className="mb-6">
        <h1 className="text-2xl font-bold text-gray-900">{displayTitle}</h1>
      </div>

      {/* Filters */}
      <div className="mb-6 flex flex-wrap gap-4">
        {onStatusFilter && (
          <select
            value={statusFilter}
            onChange={(e) => handleStatusChange(e.target.value)}
            className="px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
          >
            <option value="">All Statuses</option>
            {statusOptions.map((status) => (
              <option key={status.value} value={status.value}>
                {status.label}
              </option>
            ))}
          </select>
        )}

        {onDateRangeFilter && (
          <>
            <div className="flex items-center gap-2">
              <label htmlFor="fromDate" className="text-sm text-gray-600">
                From:
              </label>
              <input
                type="date"
                id="fromDate"
                value={fromDate}
                onChange={(e) => handleFromDateChange(e.target.value)}
                className="px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
              />
            </div>
            <div className="flex items-center gap-2">
              <label htmlFor="toDate" className="text-sm text-gray-600">
                To:
              </label>
              <input
                type="date"
                id="toDate"
                value={toDate}
                onChange={(e) => handleToDateChange(e.target.value)}
                className="px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
              />
            </div>
          </>
        )}
      </div>

      {/* List */}
      {isLoading ? (
        <div className="flex items-center justify-center py-12">
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600" />
        </div>
      ) : bookings.length === 0 ? (
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
              d="M8 7V3m8 4V3m-9 8h10M5 21h14a2 2 0 002-2V7a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z"
            />
          </svg>
          <p className="mt-4">{t('facilities.bookings.empty')}</p>
        </div>
      ) : (
        <div className="grid gap-4">
          {bookings.map((booking) => (
            <BookingCard
              key={booking.id}
              booking={booking}
              onView={onView}
              onCancel={onCancel}
              onApprove={onApprove}
              onReject={onReject}
              isManager={isManager}
            />
          ))}
        </div>
      )}

      {/* Pagination */}
      {totalPages > 1 && (
        <div className="mt-6 flex items-center justify-between">
          <p className="text-sm text-gray-500">
            Showing {(page - 1) * pageSize + 1} to {Math.min(page * pageSize, total)} of {total}
          </p>
          <div className="flex gap-2">
            <button
              type="button"
              onClick={() => onPageChange(page - 1)}
              disabled={page === 1}
              className="px-3 py-1 border rounded-md disabled:opacity-50 disabled:cursor-not-allowed hover:bg-gray-50"
            >
              Previous
            </button>
            <span className="px-3 py-1">
              Page {page} of {totalPages}
            </span>
            <button
              type="button"
              onClick={() => onPageChange(page + 1)}
              disabled={page === totalPages}
              className="px-3 py-1 border rounded-md disabled:opacity-50 disabled:cursor-not-allowed hover:bg-gray-50"
            >
              Next
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
