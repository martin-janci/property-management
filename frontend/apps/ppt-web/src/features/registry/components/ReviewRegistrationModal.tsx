/**
 * ReviewRegistrationModal Component
 *
 * Modal for approving/rejecting registrations (Epic 57).
 */

import { useState } from 'react';
import { useTranslation } from 'react-i18next';

interface ReviewRegistrationModalProps {
  registrationType: 'pet' | 'vehicle';
  registrationName: string;
  isOpen: boolean;
  onClose: () => void;
  onApprove: () => void;
  onReject: (reason: string) => void;
  isSubmitting?: boolean;
}

export function ReviewRegistrationModal({
  registrationType,
  registrationName,
  isOpen,
  onClose,
  onApprove,
  onReject,
  isSubmitting,
}: ReviewRegistrationModalProps) {
  const { t } = useTranslation();
  const [action, setAction] = useState<'approve' | 'reject' | null>(null);
  const [rejectionReason, setRejectionReason] = useState('');

  if (!isOpen) return null;

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (action === 'approve') {
      onApprove();
    } else if (action === 'reject' && rejectionReason.trim()) {
      onReject(rejectionReason);
    }
  };

  const resetAndClose = () => {
    setAction(null);
    setRejectionReason('');
    onClose();
  };

  return (
    <div className="fixed inset-0 z-50 overflow-y-auto">
      <div className="flex min-h-screen items-center justify-center p-4">
        {/* Backdrop */}
        <div
          className="fixed inset-0 bg-black bg-opacity-25 transition-opacity"
          onKeyDown={(e) => {
            if (e.key === 'Escape') resetAndClose();
          }}
          role="button"
          tabIndex={0}
          aria-label={t('aria.closeModal')}
        />

        {/* Modal */}
        <div className="relative bg-white rounded-lg shadow-xl max-w-md w-full p-6">
          <div className="mb-4">
            <h3 className="text-lg font-medium text-gray-900">
              Review {registrationType === 'pet' ? 'Pet' : 'Vehicle'} Registration
            </h3>
            <p className="mt-1 text-sm text-gray-500">{registrationName}</p>
          </div>

          <form onSubmit={handleSubmit} className="space-y-4">
            {/* Action Selection */}
            {!action && (
              <div className="space-y-3">
                <span className="block text-sm font-medium text-gray-700">Choose action:</span>
                <div className="grid grid-cols-2 gap-3">
                  <button
                    type="button"
                    onClick={() => setAction('approve')}
                    className="px-4 py-3 bg-green-50 text-green-700 border-2 border-green-200 rounded-md hover:bg-green-100 hover:border-green-300 transition-colors"
                  >
                    <svg
                      className="w-6 h-6 mx-auto mb-1"
                      fill="none"
                      stroke="currentColor"
                      viewBox="0 0 24 24"
                    >
                      <title>Approve</title>
                      <path
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        strokeWidth={2}
                        d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"
                      />
                    </svg>
                    Approve
                  </button>
                  <button
                    type="button"
                    onClick={() => setAction('reject')}
                    className="px-4 py-3 bg-red-50 text-red-700 border-2 border-red-200 rounded-md hover:bg-red-100 hover:border-red-300 transition-colors"
                  >
                    <svg
                      className="w-6 h-6 mx-auto mb-1"
                      fill="none"
                      stroke="currentColor"
                      viewBox="0 0 24 24"
                    >
                      <title>Reject</title>
                      <path
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        strokeWidth={2}
                        d="M10 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2m7-2a9 9 0 11-18 0 9 9 0 0118 0z"
                      />
                    </svg>
                    Reject
                  </button>
                </div>
              </div>
            )}

            {/* Approve Confirmation */}
            {action === 'approve' && (
              <div className="space-y-4">
                <div className="bg-green-50 border border-green-200 rounded-md p-4">
                  <div className="flex">
                    <svg
                      className="w-5 h-5 text-green-400"
                      fill="none"
                      stroke="currentColor"
                      viewBox="0 0 24 24"
                    >
                      <title>Info</title>
                      <path
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        strokeWidth={2}
                        d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
                      />
                    </svg>
                    <div className="ml-3">
                      <p className="text-sm text-green-700">
                        Are you sure you want to approve this registration? The owner will be
                        notified.
                      </p>
                    </div>
                  </div>
                </div>

                <div className="flex justify-end gap-3">
                  <button
                    type="button"
                    onClick={() => setAction(null)}
                    disabled={isSubmitting}
                    className="px-4 py-2 border border-gray-300 rounded-md text-gray-700 hover:bg-gray-50 disabled:opacity-50"
                  >
                    Back
                  </button>
                  <button
                    type="submit"
                    disabled={isSubmitting}
                    className="px-4 py-2 bg-green-600 text-white rounded-md hover:bg-green-700 disabled:opacity-50 flex items-center gap-2"
                  >
                    {isSubmitting && (
                      <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-white" />
                    )}
                    {isSubmitting ? 'Approving...' : 'Confirm Approval'}
                  </button>
                </div>
              </div>
            )}

            {/* Reject Form */}
            {action === 'reject' && (
              <div className="space-y-4">
                <div>
                  <label
                    htmlFor="rejectionReason"
                    className="block text-sm font-medium text-gray-700 mb-1"
                  >
                    Reason for rejection *
                  </label>
                  <textarea
                    id="rejectionReason"
                    required
                    rows={4}
                    value={rejectionReason}
                    onChange={(e) => setRejectionReason(e.target.value)}
                    className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                    placeholder="Please provide a reason for rejecting this registration..."
                  />
                  <p className="mt-1 text-sm text-gray-500">This will be sent to the owner.</p>
                </div>

                <div className="flex justify-end gap-3">
                  <button
                    type="button"
                    onClick={() => setAction(null)}
                    disabled={isSubmitting}
                    className="px-4 py-2 border border-gray-300 rounded-md text-gray-700 hover:bg-gray-50 disabled:opacity-50"
                  >
                    Back
                  </button>
                  <button
                    type="submit"
                    disabled={isSubmitting || !rejectionReason.trim()}
                    className="px-4 py-2 bg-red-600 text-white rounded-md hover:bg-red-700 disabled:opacity-50 flex items-center gap-2"
                  >
                    {isSubmitting && (
                      <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-white" />
                    )}
                    {isSubmitting ? 'Rejecting...' : 'Confirm Rejection'}
                  </button>
                </div>
              </div>
            )}
          </form>

          {/* Close button (only shown when no action selected) */}
          {!action && (
            <div className="mt-6 flex justify-end">
              <button
                type="button"
                onClick={resetAndClose}
                className="px-4 py-2 border border-gray-300 rounded-md text-gray-700 hover:bg-gray-50"
              >
                Cancel
              </button>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
