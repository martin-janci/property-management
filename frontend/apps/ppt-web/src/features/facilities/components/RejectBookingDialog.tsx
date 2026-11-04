/**
 * Reject Booking Dialog Component (Epic 56: Facility Booking).
 *
 * Modal dialog for rejecting a booking with a reason.
 */

import { useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useFocusTrap } from '../../../hooks/useFocusTrap';

interface RejectBookingDialogProps {
  bookingId: string;
  facilityName: string;
  onConfirm: (reason: string) => void;
  onCancel: () => void;
  isSubmitting?: boolean;
}

export function RejectBookingDialog({
  facilityName,
  onConfirm,
  onCancel,
  isSubmitting,
}: RejectBookingDialogProps) {
  const { t } = useTranslation();
  const [reason, setReason] = useState('');
  const [error, setError] = useState('');
  const dialogRef = useRef<HTMLDivElement>(null);
  useFocusTrap(dialogRef, onCancel);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!reason.trim()) {
      setError(t('facilities.bookings.rejectDialog.reasonRequired'));
      return;
    }
    onConfirm(reason.trim());
  };

  return (
    <div
      ref={dialogRef}
      className="fixed inset-0 z-50 overflow-y-auto"
      role="dialog"
      aria-modal="true"
      aria-labelledby="dialog-title"
      tabIndex={-1}
    >
      {/* Backdrop - click handler for dismiss, keyboard users use ESC or Cancel button */}
      <div
        className="fixed inset-0 bg-black bg-opacity-50 transition-opacity"
        onClick={onCancel}
        aria-hidden="true"
      />

      {/* Modal */}
      <div className="flex min-h-full items-center justify-center p-4">
        <div className="relative w-full max-w-md bg-white rounded-lg shadow-xl transform transition-all">
          {/* Header */}
          <div className="px-6 py-4 border-b">
            <h3 id="dialog-title" className="text-lg font-semibold text-gray-900">
              {t('facilities.bookings.rejectDialog.title')}
            </h3>
            <p className="mt-1 text-sm text-gray-500">
              {t('facilities.bookings.rejectDialog.subtitle', { name: facilityName })}
            </p>
          </div>

          {/* Content */}
          <form onSubmit={handleSubmit}>
            <div className="px-6 py-4">
              <label htmlFor="reason" className="block text-sm font-medium text-gray-700">
                {t('facilities.bookings.rejectDialog.reason')}
              </label>
              <textarea
                id="reason"
                value={reason}
                onChange={(e) => {
                  setReason(e.target.value);
                  setError('');
                }}
                rows={4}
                placeholder={t('facilities.bookings.rejectDialog.reasonPlaceholder')}
                className="mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
              />
              {error && <p className="mt-1 text-sm text-red-600">{error}</p>}
            </div>

            {/* Actions */}
            <div className="px-6 py-4 bg-gray-50 rounded-b-lg flex gap-3 justify-end">
              <button
                type="button"
                onClick={onCancel}
                className="px-4 py-2 border border-gray-300 rounded-md hover:bg-gray-100 transition-colors"
              >
                {t('facilities.bookings.rejectDialog.cancel')}
              </button>
              <button
                type="submit"
                disabled={isSubmitting}
                className="px-4 py-2 bg-red-600 text-white rounded-md hover:bg-red-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
              >
                {isSubmitting
                  ? t('facilities.bookings.rejectDialog.submitting')
                  : t('facilities.bookings.rejectDialog.submit')}
              </button>
            </div>
          </form>
        </div>
      </div>
    </div>
  );
}
