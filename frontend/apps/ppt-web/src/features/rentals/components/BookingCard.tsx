/**
 * BookingCard Component
 *
 * Displays a booking summary card with guest info and status.
 * Epic 18: Short-Term Rental Integration (Story 18.2)
 */

import { useTranslation } from 'react-i18next';
import { parseIsoDate } from '../mappers';
import type { BookingSource, BookingStatus, RentalBooking } from '../types';

interface BookingCardProps {
  booking: RentalBooking;
  onView?: (id: string) => void;
  onEdit?: (id: string) => void;
  onCancel?: (id: string) => void;
  onCheckIn?: (id: string) => void;
  onCheckOut?: (id: string) => void;
}

const statusColors: Record<BookingStatus, string> = {
  pending: 'bg-yellow-100 text-yellow-800',
  confirmed: 'bg-blue-100 text-blue-800',
  checked_in: 'bg-green-100 text-green-800',
  checked_out: 'bg-gray-100 text-gray-800',
  cancelled: 'bg-red-100 text-red-800',
  no_show: 'bg-orange-100 text-orange-800',
};

const sourceIcons: Record<BookingSource, string> = {
  airbnb: 'A',
  booking: 'B',
  direct: 'D',
  other: 'O',
};

const sourceColors: Record<BookingSource, string> = {
  airbnb: 'bg-rose-500 text-white',
  booking: 'bg-blue-600 text-white',
  direct: 'bg-green-600 text-white',
  other: 'bg-gray-500 text-white',
};

function formatDateRange(checkIn: string, checkOut: string, language: string): string {
  const fmt = new Intl.DateTimeFormat(language, { month: 'short', day: 'numeric' });
  return `${fmt.format(parseIsoDate(checkIn))} – ${fmt.format(parseIsoDate(checkOut))}`;
}

function getNights(checkIn: string, checkOut: string): number {
  const diffTime = Math.abs(parseIsoDate(checkOut).getTime() - parseIsoDate(checkIn).getTime());
  return Math.round(diffTime / (1000 * 60 * 60 * 24));
}

function formatCurrency(
  amount: number | undefined,
  currency: string | undefined,
  language: string
): string {
  if (amount === undefined) return '-';
  return new Intl.NumberFormat(language, {
    style: 'currency',
    currency: currency || 'EUR',
  }).format(amount);
}

export function BookingCard({
  booking,
  onView,
  onEdit,
  onCancel,
  onCheckIn,
  onCheckOut,
}: BookingCardProps) {
  const { t, i18n } = useTranslation();
  const nights = getNights(booking.checkIn, booking.checkOut);
  const isPast = new Date(booking.checkOut) < new Date();
  const isActive = booking.status === 'checked_in';
  const canCheckIn = booking.status === 'confirmed' && !isPast;
  const canCheckOut = booking.status === 'checked_in';
  const canCancel = ['pending', 'confirmed'].includes(booking.status);

  return (
    <div className="bg-white rounded-lg shadow overflow-hidden hover:shadow-md transition-shadow">
      {/* Header with source badge */}
      <div className="flex items-center justify-between px-4 py-3 bg-gray-50 border-b">
        <div className="flex items-center gap-2">
          <span
            className={`w-6 h-6 flex items-center justify-center text-xs font-bold rounded ${sourceColors[booking.source]}`}
            title={booking.source}
          >
            {sourceIcons[booking.source]}
          </span>
          <span className="text-sm font-medium text-gray-700">
            {booking.unitName || t('rentals.booking.unit')}
          </span>
        </div>
        <span className={`px-2 py-1 text-xs font-medium rounded ${statusColors[booking.status]}`}>
          {t(`rentals.status.${booking.status}`)}
        </span>
      </div>

      {/* Content */}
      <div className="p-4">
        {/* Guest Info */}
        <div className="flex items-start justify-between">
          <div>
            <h3 className="text-lg font-semibold text-gray-900">{booking.guestName}</h3>
            {booking.guestEmail && <p className="text-sm text-gray-500">{booking.guestEmail}</p>}
          </div>
          <div className="text-right">
            <p className="text-lg font-bold text-gray-900">
              {formatCurrency(booking.totalPrice, booking.currency, i18n.language)}
            </p>
            <p className="text-xs text-gray-500">{t('rentals.nightsCount', { count: nights })}</p>
          </div>
        </div>

        {/* Date & Guest Count */}
        <div className="mt-4 flex items-center gap-4 text-sm text-gray-600">
          <div className="flex items-center gap-1">
            <svg
              className="w-4 h-4 text-gray-400"
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
            <span>{formatDateRange(booking.checkIn, booking.checkOut, i18n.language)}</span>
          </div>
          <div className="flex items-center gap-1">
            <svg
              className="w-4 h-4 text-gray-400"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
              aria-hidden="true"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0z"
              />
            </svg>
            <span>{t('rentals.guestsCount', { count: booking.guestCount })}</span>
          </div>
        </div>

        {/* Building Info */}
        {booking.buildingName && (
          <p className="mt-2 text-xs text-gray-400">{booking.buildingName}</p>
        )}

        {/* Platform Booking ID */}
        {booking.platformBookingId && (
          <p className="mt-1 text-xs text-gray-400">
            {t('rentals.bookingId')}: {booking.platformBookingId}
          </p>
        )}

        {/* Active Indicator */}
        {isActive && (
          <div className="mt-3 flex items-center gap-2 text-green-600">
            <span className="relative flex h-2 w-2">
              <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-green-400 opacity-75" />
              <span className="relative inline-flex rounded-full h-2 w-2 bg-green-500" />
            </span>
            <span className="text-sm font-medium">{t('rentals.currentlyStaying')}</span>
          </div>
        )}
      </div>

      {/* Actions */}
      <div className="px-4 py-3 bg-gray-50 border-t flex items-center justify-between">
        <button
          type="button"
          onClick={() => onView?.(booking.id)}
          className="text-sm text-blue-600 hover:text-blue-800 font-medium"
        >
          {t('common.view')}
        </button>
        <div className="flex items-center gap-2">
          {canCheckIn && (
            <button
              type="button"
              onClick={() => onCheckIn?.(booking.id)}
              className="px-3 py-1 text-sm bg-green-600 text-white rounded-md hover:bg-green-700"
            >
              {t('rentals.checkIn')}
            </button>
          )}
          {canCheckOut && (
            <button
              type="button"
              onClick={() => onCheckOut?.(booking.id)}
              className="px-3 py-1 text-sm bg-blue-600 text-white rounded-md hover:bg-blue-700"
            >
              {t('rentals.checkOut')}
            </button>
          )}
          {!isPast && booking.status !== 'cancelled' && (
            <button
              type="button"
              onClick={() => onEdit?.(booking.id)}
              className="p-1 text-gray-400 hover:text-gray-600"
              title={t('common.edit')}
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
          )}
          {canCancel && (
            <button
              type="button"
              onClick={() => onCancel?.(booking.id)}
              className="p-1 text-gray-400 hover:text-red-600"
              title={t('common.cancel')}
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
                  d="M6 18L18 6M6 6l12 12"
                />
              </svg>
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
