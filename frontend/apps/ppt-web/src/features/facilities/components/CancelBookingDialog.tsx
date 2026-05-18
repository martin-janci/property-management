/**
 * Cancel Booking Dialog Component (Epic 56: Facility Booking).
 *
 * Modal dialog for cancelling a booking with an optional reason.
 */

import { useRef, useState } from 'react';
import { useFocusTrap } from '../../../hooks/useFocusTrap';

interface CancelBookingDialogProps {
  bookingId: string;
  facilityName: string;
  startTime: string;
  onConfirm: (reason?: string) => void;
  onCancel: () => void;
  isSubmitting?: boolean;
}

function formatDateTime(dateStr: string): string {
  const date = new Date(dateStr);
  return date.toLocaleString('en-US', {
    weekday: 'long',
    month: 'long',
    day: 'numeric',
    hour: 'numeric',
    minute: '2-digit',
  });
}

export function CancelBookingDialog({
  facilityName,
  startTime,
  onConfirm,
  onCancel,
  isSubmitting,
}: CancelBookingDialogProps) {
  const [reason, setReason] = useState('');
  const dialogRef = useRef<HTMLDivElement>(null);
  useFocusTrap(dialogRef, onCancel);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    onConfirm(reason.trim() || undefined);
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
              Cancel Booking
            </h3>
          </div>

          {/* Content */}
          <form onSubmit={handleSubmit}>
            <div className="px-6 py-4">
              <div className="bg-yellow-50 border border-yellow-200 rounded-md p-4 mb-4">
                <div className="flex">
                  <svg
                    className="h-5 w-5 text-yellow-400"
                    viewBox="0 0 20 20"
                    fill="currentColor"
                    aria-hidden="true"
                  >
                    <path
                      fillRule="evenodd"
                      d="M8.485 2.495c.673-1.167 2.357-1.167 3.03 0l6.28 10.875c.673 1.167-.17 2.625-1.516 2.625H3.72c-1.347 0-2.189-1.458-1.515-2.625L8.485 2.495zM10 5a.75.75 0 01.75.75v3.5a.75.75 0 01-1.5 0v-3.5A.75.75 0 0110 5zm0 9a1 1 0 100-2 1 1 0 000 2z"
                      clipRule="evenodd"
                    />
                  </svg>
                  <div className="ml-3">
                    <p className="text-sm text-yellow-700">
                      Are you sure you want to cancel this booking?
                    </p>
                  </div>
                </div>
              </div>

              <div className="mb-4 p-3 bg-gray-50 rounded-md">
                <p className="font-medium text-gray-900">{facilityName}</p>
                <p className="text-sm text-gray-600">{formatDateTime(startTime)}</p>
              </div>

              <label htmlFor="reason" className="block text-sm font-medium text-gray-700">
                Reason for Cancellation (optional)
              </label>
              <textarea
                id="reason"
                value={reason}
                onChange={(e) => setReason(e.target.value)}
                rows={3}
                placeholder="Let others know why you're cancelling..."
                className="mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
              />
            </div>

            {/* Actions */}
            <div className="px-6 py-4 bg-gray-50 rounded-b-lg flex gap-3 justify-end">
              <button
                type="button"
                onClick={onCancel}
                className="px-4 py-2 border border-gray-300 rounded-md hover:bg-gray-100 transition-colors"
              >
                Keep Booking
              </button>
              <button
                type="submit"
                disabled={isSubmitting}
                className="px-4 py-2 bg-red-600 text-white rounded-md hover:bg-red-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
              >
                {isSubmitting ? 'Cancelling...' : 'Cancel Booking'}
              </button>
            </div>
          </form>
        </div>
      </div>
    </div>
  );
}
