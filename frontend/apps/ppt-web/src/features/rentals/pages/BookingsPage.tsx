/**
 * BookingsPage - List all bookings with filters.
 * Epic 18: Short-Term Rental Integration (Story 18.2)
 */

import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { ConfirmationDialog } from '../../../components/ConfirmationDialog';
import { BookingCard } from '../components/BookingCard';
import { BookingFormDialog } from '../components/BookingFormDialog';
import { emptyBookingForm } from '../mappers';
import type {
  BookingFormValues,
  BookingListParams,
  BookingSource,
  BookingStatus,
  RentalBooking,
  RentalUnitOption,
} from '../types';

interface Building {
  id: string;
  name: string;
}

interface BookingsPageProps {
  bookings: RentalBooking[];
  total: number;
  buildings: Building[];
  units: RentalUnitOption[];
  isLoading?: boolean;
  onFilterChange: (params: BookingListParams) => void;
  onViewBooking: (id: string) => void;
  onEditBooking: (id: string) => void;
  /** Called after the user confirmed the cancellation. */
  onCancelBooking: (id: string) => void;
  onCheckIn: (id: string) => void;
  onCheckOut: (id: string) => void;
  /** `POST /rentals/bookings`; rejects (e.g. 409 unit unavailable) keep the dialog open. */
  onCreateBooking: (values: BookingFormValues) => Promise<void>;
  onBack: () => void;
}

const statuses: BookingStatus[] = [
  'pending',
  'confirmed',
  'checked_in',
  'checked_out',
  'cancelled',
  'no_show',
];

const sources: BookingSource[] = ['airbnb', 'booking', 'direct', 'other'];

export function BookingsPage({
  bookings,
  total,
  buildings,
  units,
  isLoading,
  onFilterChange,
  onViewBooking,
  onEditBooking,
  onCancelBooking,
  onCheckIn,
  onCheckOut,
  onCreateBooking,
  onBack,
}: BookingsPageProps) {
  const { t } = useTranslation();
  const [showCreate, setShowCreate] = useState(false);
  const [cancelTarget, setCancelTarget] = useState<RentalBooking | null>(null);
  const [page, setPage] = useState(1);
  const [pageSize] = useState(12);
  const [selectedBuilding, setSelectedBuilding] = useState<string>('');
  const [selectedUnit, setSelectedUnit] = useState<string>('');
  const [selectedStatus, setSelectedStatus] = useState<BookingStatus | ''>('');
  const [selectedSource, setSelectedSource] = useState<BookingSource | ''>('');
  const [fromDate, setFromDate] = useState<string>('');
  const [toDate, setToDate] = useState<string>('');

  const filteredUnits = selectedBuilding
    ? units.filter((u) => u.buildingId === selectedBuilding)
    : units;

  const totalPages = Math.ceil(total / pageSize);

  const applyFilters = () => {
    const params: BookingListParams = {
      page: 1,
      limit: pageSize,
    };
    if (selectedBuilding) params.buildingId = selectedBuilding;
    if (selectedUnit) params.unitId = selectedUnit;
    if (selectedStatus) params.status = selectedStatus;
    if (selectedSource) params.platform = selectedSource;
    if (fromDate) params.fromDate = fromDate;
    if (toDate) params.toDate = toDate;

    setPage(1);
    onFilterChange(params);
  };

  const clearFilters = () => {
    setSelectedBuilding('');
    setSelectedUnit('');
    setSelectedStatus('');
    setSelectedSource('');
    setFromDate('');
    setToDate('');
    setPage(1);
    onFilterChange({ page: 1, limit: pageSize });
  };

  const handlePageChange = (newPage: number) => {
    setPage(newPage);
    onFilterChange({
      page: newPage,
      limit: pageSize,
      buildingId: selectedBuilding || undefined,
      unitId: selectedUnit || undefined,
      status: selectedStatus || undefined,
      platform: selectedSource || undefined,
      fromDate: fromDate || undefined,
      toDate: toDate || undefined,
    });
  };

  return (
    <div className="min-h-screen bg-gray-100">
      {/* Header */}
      <div className="bg-white shadow">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-6">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-4">
              <button type="button" onClick={onBack} className="text-gray-500 hover:text-gray-700">
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
                <h1 className="text-2xl font-bold text-gray-900">{t('rentals.bookings.title')}</h1>
                <p className="mt-1 text-sm text-gray-500">
                  {t('rentals.bookings.showing', { count: total })}
                </p>
              </div>
            </div>
            <button
              type="button"
              onClick={() => setShowCreate(true)}
              className="px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700"
            >
              {t('rentals.bookings.createBooking')}
            </button>
          </div>
        </div>
      </div>

      {/* Filters */}
      <div className="bg-white border-b">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-4">
          <div className="grid grid-cols-1 md:grid-cols-3 lg:grid-cols-6 gap-4">
            <div>
              <label htmlFor="building" className="block text-xs font-medium text-gray-500 mb-1">
                {t('rentals.bookings.building')}
              </label>
              <select
                id="building"
                value={selectedBuilding}
                onChange={(e) => {
                  setSelectedBuilding(e.target.value);
                  setSelectedUnit('');
                }}
                className="w-full px-3 py-2 border border-gray-300 rounded-md text-sm focus:ring-blue-500 focus:border-blue-500"
              >
                <option value="">{t('common.all')}</option>
                {buildings.map((b) => (
                  <option key={b.id} value={b.id}>
                    {b.name}
                  </option>
                ))}
              </select>
            </div>
            <div>
              <label htmlFor="unit" className="block text-xs font-medium text-gray-500 mb-1">
                {t('rentals.bookings.unit')}
              </label>
              <select
                id="unit"
                value={selectedUnit}
                onChange={(e) => setSelectedUnit(e.target.value)}
                className="w-full px-3 py-2 border border-gray-300 rounded-md text-sm focus:ring-blue-500 focus:border-blue-500"
              >
                <option value="">{t('common.all')}</option>
                {filteredUnits.map((u) => (
                  <option key={u.id} value={u.id}>
                    {u.name}
                  </option>
                ))}
              </select>
            </div>
            <div>
              <label htmlFor="status" className="block text-xs font-medium text-gray-500 mb-1">
                {t('rentals.bookings.status')}
              </label>
              <select
                id="status"
                value={selectedStatus}
                onChange={(e) => setSelectedStatus(e.target.value as BookingStatus | '')}
                className="w-full px-3 py-2 border border-gray-300 rounded-md text-sm focus:ring-blue-500 focus:border-blue-500"
              >
                <option value="">{t('common.all')}</option>
                {statuses.map((s) => (
                  <option key={s} value={s}>
                    {t(`rentals.status.${s}`)}
                  </option>
                ))}
              </select>
            </div>
            <div>
              <label htmlFor="source" className="block text-xs font-medium text-gray-500 mb-1">
                {t('rentals.bookings.source')}
              </label>
              <select
                id="source"
                value={selectedSource}
                onChange={(e) => setSelectedSource(e.target.value as BookingSource | '')}
                className="w-full px-3 py-2 border border-gray-300 rounded-md text-sm focus:ring-blue-500 focus:border-blue-500"
              >
                <option value="">{t('common.all')}</option>
                {sources.map((s) => (
                  <option key={s} value={s}>
                    {t(`rentals.source.${s}`)}
                  </option>
                ))}
              </select>
            </div>
            <div>
              <label htmlFor="fromDate" className="block text-xs font-medium text-gray-500 mb-1">
                {t('rentals.bookings.fromDate')}
              </label>
              <input
                type="date"
                id="fromDate"
                value={fromDate}
                onChange={(e) => setFromDate(e.target.value)}
                className="w-full px-3 py-2 border border-gray-300 rounded-md text-sm focus:ring-blue-500 focus:border-blue-500"
              />
            </div>
            <div>
              <label htmlFor="toDate" className="block text-xs font-medium text-gray-500 mb-1">
                {t('rentals.bookings.toDate')}
              </label>
              <input
                type="date"
                id="toDate"
                value={toDate}
                onChange={(e) => setToDate(e.target.value)}
                className="w-full px-3 py-2 border border-gray-300 rounded-md text-sm focus:ring-blue-500 focus:border-blue-500"
              />
            </div>
          </div>
          <div className="mt-4 flex items-center gap-2">
            <button
              type="button"
              onClick={applyFilters}
              className="px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700"
            >
              {t('rentals.bookings.applyFilters')}
            </button>
            <button
              type="button"
              onClick={clearFilters}
              className="px-4 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-md hover:bg-gray-50"
            >
              {t('rentals.bookings.clearFilters')}
            </button>
          </div>
        </div>
      </div>

      {/* Content */}
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        {isLoading ? (
          <div className="flex items-center justify-center h-64">
            <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600" />
          </div>
        ) : bookings.length === 0 ? (
          <div className="bg-white rounded-lg shadow p-8 text-center">
            <svg
              className="w-12 h-12 mx-auto text-gray-400"
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
            <p className="mt-4 text-gray-500">{t('rentals.bookings.noBookings')}</p>
            <button
              type="button"
              onClick={() => setShowCreate(true)}
              className="mt-4 px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700"
            >
              {t('rentals.bookings.createFirstBooking')}
            </button>
          </div>
        ) : (
          <>
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
              {bookings.map((booking) => (
                <BookingCard
                  key={booking.id}
                  booking={booking}
                  onView={onViewBooking}
                  onEdit={onEditBooking}
                  onCancel={(id) => setCancelTarget(bookings.find((b) => b.id === id) ?? null)}
                  onCheckIn={onCheckIn}
                  onCheckOut={onCheckOut}
                />
              ))}
            </div>

            {/* Pagination */}
            {totalPages > 1 && (
              <div className="mt-8 flex items-center justify-center gap-2">
                <button
                  type="button"
                  onClick={() => handlePageChange(page - 1)}
                  disabled={page === 1}
                  className="px-3 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-md hover:bg-gray-50 disabled:opacity-50 disabled:cursor-not-allowed"
                >
                  {t('common.previous')}
                </button>
                <span className="px-4 py-2 text-sm text-gray-700">
                  {t('rentals.bookings.page', { current: page, total: totalPages })}
                </span>
                <button
                  type="button"
                  onClick={() => handlePageChange(page + 1)}
                  disabled={page === totalPages}
                  className="px-3 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-md hover:bg-gray-50 disabled:opacity-50 disabled:cursor-not-allowed"
                >
                  {t('common.next')}
                </button>
              </div>
            )}
          </>
        )}
      </div>

      <ConfirmationDialog
        isOpen={cancelTarget !== null}
        title={t('rentals.bookings.cancelTitle')}
        message={
          cancelTarget
            ? t('rentals.bookings.cancelMessage', {
                guest: cancelTarget.guestName,
                unit: cancelTarget.unitName ?? '',
              })
            : ''
        }
        confirmLabel={t('rentals.bookings.cancelConfirm')}
        cancelLabel={t('rentals.bookings.keepBooking')}
        variant="danger"
        onConfirm={() => {
          if (cancelTarget) onCancelBooking(cancelTarget.id);
          setCancelTarget(null);
        }}
        onCancel={() => setCancelTarget(null)}
      />

      {showCreate && (
        <BookingFormDialog
          mode="create"
          initial={emptyBookingForm(selectedUnit)}
          units={units}
          onSubmit={onCreateBooking}
          onClose={() => setShowCreate(false)}
        />
      )}
    </div>
  );
}
