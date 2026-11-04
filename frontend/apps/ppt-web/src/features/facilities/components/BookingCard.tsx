/**
 * Booking Card Component (Epic 56: Facility Booking).
 *
 * Displays a booking summary with status and actions.
 */

import type { BookingStatus, BookingWithDetails } from '@ppt/api-client';
import type { TFunction } from 'i18next';
import { useTranslation } from 'react-i18next';

interface BookingCardProps {
  booking: BookingWithDetails;
  onView?: (id: string) => void;
  onCancel?: (id: string) => void;
  onApprove?: (id: string) => void;
  onReject?: (id: string) => void;
  isManager?: boolean;
}

const statusColors: Record<BookingStatus, string> = {
  pending: 'bg-yellow-100 text-yellow-800',
  approved: 'bg-green-100 text-green-800',
  rejected: 'bg-red-100 text-red-800',
  cancelled: 'bg-gray-100 text-gray-800',
  completed: 'bg-blue-100 text-blue-800',
  no_show: 'bg-orange-100 text-orange-800',
};

function formatDateTime(dateStr: string, locale: string): string {
  const date = new Date(dateStr);
  return date.toLocaleString(locale, {
    weekday: 'short',
    month: 'short',
    day: 'numeric',
    hour: 'numeric',
    minute: '2-digit',
  });
}

function formatDuration(start: string, end: string, t: TFunction): string {
  const startDate = new Date(start);
  const endDate = new Date(end);
  const durationMs = endDate.getTime() - startDate.getTime();
  const hours = Math.floor(durationMs / (1000 * 60 * 60));
  const minutes = Math.floor((durationMs % (1000 * 60 * 60)) / (1000 * 60));

  if (hours === 0) return t('facilities.bookings.durationMinutes', { minutes });
  if (minutes === 0) return t('facilities.bookings.durationHours', { hours });
  return t('facilities.bookings.durationHoursMinutes', { hours, minutes });
}

export function BookingCard({
  booking,
  onView,
  onCancel,
  onApprove,
  onReject,
  isManager,
}: BookingCardProps) {
  const { t, i18n } = useTranslation();
  const canCancel = booking.status === 'pending' || booking.status === 'approved';
  const canApprove = isManager && booking.status === 'pending';
  const canReject = isManager && booking.status === 'pending';

  return (
    <div className="bg-white rounded-lg shadow p-4 hover:shadow-md transition-shadow">
      <div className="flex items-start justify-between">
        <div className="flex-1">
          <div className="flex items-center gap-2">
            <h3 className="text-lg font-semibold text-gray-900">{booking.facility_name}</h3>
            <span
              className={`px-2 py-1 text-xs font-medium rounded ${statusColors[booking.status]}`}
            >
              {t(`facilities.bookings.status.${booking.status}`)}
            </span>
          </div>

          <div className="mt-2 space-y-1 text-sm text-gray-600">
            <p className="flex items-center gap-2">
              <svg className="w-4 h-4" fill="currentColor" viewBox="0 0 20 20" aria-hidden="true">
                <path
                  fillRule="evenodd"
                  d="M6 2a1 1 0 00-1 1v1H4a2 2 0 00-2 2v10a2 2 0 002 2h12a2 2 0 002-2V6a2 2 0 00-2-2h-1V3a1 1 0 10-2 0v1H7V3a1 1 0 00-1-1zm0 5a1 1 0 000 2h8a1 1 0 100-2H6z"
                  clipRule="evenodd"
                />
              </svg>
              {formatDateTime(booking.start_time, i18n.language)}
            </p>
            <p className="flex items-center gap-2">
              <svg className="w-4 h-4" fill="currentColor" viewBox="0 0 20 20" aria-hidden="true">
                <path
                  fillRule="evenodd"
                  d="M10 18a8 8 0 100-16 8 8 0 000 16zm1-12a1 1 0 10-2 0v4a1 1 0 00.293.707l2.828 2.829a1 1 0 101.415-1.415L11 9.586V6z"
                  clipRule="evenodd"
                />
              </svg>
              {t('facilities.bookings.duration', {
                duration: formatDuration(booking.start_time, booking.end_time, t),
              })}
            </p>
            {booking.purpose && (
              <p className="flex items-start gap-2">
                <svg
                  className="w-4 h-4 mt-0.5"
                  fill="currentColor"
                  viewBox="0 0 20 20"
                  aria-hidden="true"
                >
                  <path
                    fillRule="evenodd"
                    d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7-4a1 1 0 11-2 0 1 1 0 012 0zM9 9a1 1 0 000 2v3a1 1 0 001 1h1a1 1 0 100-2v-3a1 1 0 00-1-1H9z"
                    clipRule="evenodd"
                  />
                </svg>
                <span className="line-clamp-2">{booking.purpose}</span>
              </p>
            )}
            {booking.attendees_count && (
              <p className="flex items-center gap-2">
                <svg className="w-4 h-4" fill="currentColor" viewBox="0 0 20 20" aria-hidden="true">
                  <path d="M10 9a3 3 0 100-6 3 3 0 000 6zm-7 9a7 7 0 1114 0H3z" />
                </svg>
                {t('facilities.bookings.attendees', { count: booking.attendees_count })}
              </p>
            )}
          </div>

          {booking.rejection_reason && (
            <div className="mt-2 p-2 bg-red-50 rounded text-sm text-red-700">
              <strong>{t('facilities.bookings.rejectionReason')}</strong> {booking.rejection_reason}
            </div>
          )}

          {booking.cancellation_reason && (
            <div className="mt-2 p-2 bg-gray-50 rounded text-sm text-gray-700">
              <strong>{t('facilities.bookings.cancellationReason')}</strong>{' '}
              {booking.cancellation_reason}
            </div>
          )}
        </div>

        {booking.total_fee && (
          <div className="text-right">
            <span className="text-lg font-semibold text-gray-900">{booking.total_fee}</span>
            {booking.deposit_paid && (
              <p className="text-xs text-green-600">{t('facilities.bookings.depositPaid')}</p>
            )}
          </div>
        )}
      </div>

      {/* Actions */}
      <div className="mt-4 flex items-center gap-2 border-t pt-3">
        {onView && (
          <button
            type="button"
            onClick={() => onView(booking.id)}
            className="text-sm text-blue-600 hover:text-blue-800"
          >
            {t('facilities.bookings.viewDetails')}
          </button>
        )}
        {canApprove && (
          <button
            type="button"
            onClick={() => onApprove?.(booking.id)}
            className="text-sm px-3 py-1 bg-green-600 text-white rounded hover:bg-green-700"
          >
            {t('facilities.bookings.approve')}
          </button>
        )}
        {canReject && (
          <button
            type="button"
            onClick={() => onReject?.(booking.id)}
            className="text-sm px-3 py-1 bg-red-600 text-white rounded hover:bg-red-700"
          >
            {t('facilities.bookings.reject')}
          </button>
        )}
        {canCancel && !isManager && (
          <button
            type="button"
            onClick={() => onCancel?.(booking.id)}
            className="text-sm text-red-600 hover:text-red-800 ml-auto"
          >
            {t('facilities.bookings.cancelBooking')}
          </button>
        )}
      </div>
    </div>
  );
}
