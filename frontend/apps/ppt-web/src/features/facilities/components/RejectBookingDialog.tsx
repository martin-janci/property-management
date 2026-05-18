/**
 * Reject Booking Dialog Component (Epic 56: Facility Booking).
 *
 * Modal dialog for rejecting a booking with a reason.
 */

import { useRef, useState } from 'react';
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
  const [reason, setReason] = useState('');
  const [error, setError] = useState('');
  const dialogRef = useRef<HTMLDivElement>(null);
  useFocusTrap(dialogRef, onCancel);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!reason.trim()) {
      setError('Please provide a reason for rejection');
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
              Reject Booking
            </h3>
            <p className="mt-1 text-sm text-gray-500">Rejecting booking for: {facilityName}</p>
          </div>

          {/* Content */}
          <form onSubmit={handleSubmit}>
            <div className="px-6 py-4">
              <label htmlFor="reason" className="block text-sm font-medium text-gray-700">
                Reason for Rejection *
              </label>
              <textarea
                id="reason"
                value={reason}
                onChange={(e) => {
                  setReason(e.target.value);
                  setError('');
                }}
                rows={4}
                placeholder="Please explain why this booking is being rejected..."
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
                Cancel
              </button>
              <button
                type="submit"
                disabled={isSubmitting}
                className="px-4 py-2 bg-red-600 text-white rounded-md hover:bg-red-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
              >
                {isSubmitting ? 'Rejecting...' : 'Reject Booking'}
              </button>
            </div>
          </form>
        </div>
      </div>
    </div>
  );
}
