/**
 * SubmissionPreviewModal - Preview modal for guest report submission.
 * Epic 41: Government Portal UI (Story 41.1)
 */

import { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { CountryFlag } from './CountrySelector';

// Progress animation constants
const INITIAL_PROGRESS = 10;
const PROGRESS_INCREMENT = 15;
const MAX_SIMULATED_PROGRESS = 90;
const PROGRESS_INTERVAL_MS = 200;

interface GuestData {
  id: string;
  firstName: string;
  lastName: string;
  dateOfBirth: string;
  nationality: string;
  documentType: string;
  documentNumber: string;
  checkInDate: string;
  checkOutDate: string;
}

interface SubmissionPreviewModalProps {
  isOpen: boolean;
  onClose: () => void;
  onConfirm: () => Promise<void>;
  guests: GuestData[];
  reportType: string;
  countryCode: string;
  periodStart: string;
  periodEnd: string;
  isSubmitting?: boolean;
}

export function SubmissionPreviewModal({
  isOpen,
  onClose,
  onConfirm,
  guests,
  reportType,
  countryCode,
  periodStart,
  periodEnd,
  isSubmitting = false,
}: SubmissionPreviewModalProps) {
  const { t } = useTranslation();
  const [progress, setProgress] = useState(0);
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  // Cleanup interval on unmount to prevent memory leaks
  useEffect(() => {
    return () => {
      if (intervalRef.current) {
        clearInterval(intervalRef.current);
      }
    };
  }, []);

  if (!isOpen) return null;

  const handleConfirm = async () => {
    setProgress(INITIAL_PROGRESS);
    try {
      intervalRef.current = setInterval(() => {
        setProgress((prev) => Math.min(prev + PROGRESS_INCREMENT, MAX_SIMULATED_PROGRESS));
      }, PROGRESS_INTERVAL_MS);

      await onConfirm();

      if (intervalRef.current) {
        clearInterval(intervalRef.current);
        intervalRef.current = null;
      }
      setProgress(100);
    } catch {
      if (intervalRef.current) {
        clearInterval(intervalRef.current);
        intervalRef.current = null;
      }
      setProgress(0);
    }
  };

  const formatDate = (dateString: string) => {
    return new Date(dateString).toLocaleDateString('en-GB', {
      day: '2-digit',
      month: 'short',
      year: 'numeric',
    });
  };

  return (
    <div className="fixed inset-0 z-50 overflow-y-auto">
      <div className="flex min-h-screen items-center justify-center p-4">
        <button
          type="button"
          className="fixed inset-0 bg-black bg-opacity-50 transition-opacity cursor-pointer"
          onClick={onClose}
          aria-label={t('aria.closeModal')}
        />
        <div className="relative w-full max-w-3xl rounded-lg bg-white shadow-xl">
          <div className="flex items-center justify-between border-b border-gray-200 px-6 py-4">
            <div>
              <h2 className="text-lg font-semibold text-gray-900">
                Preview Guest Report Submission
              </h2>
              <p className="mt-1 text-sm text-gray-500">
                Review the data before submitting to the government portal
              </p>
            </div>
            <button
              type="button"
              onClick={onClose}
              disabled={isSubmitting}
              className="rounded-md p-2 text-gray-400 hover:bg-gray-100 hover:text-gray-500 disabled:opacity-50"
            >
              <svg
                className="h-5 w-5"
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
              <span className="sr-only">Close</span>
            </button>
          </div>

          <div className="px-6 py-4 max-h-[60vh] overflow-y-auto">
            <div className="mb-6 grid grid-cols-2 gap-4 rounded-lg bg-gray-50 p-4">
              <div>
                <span className="text-sm text-gray-500">Report Type</span>
                <p className="font-medium text-gray-900">{reportType}</p>
              </div>
              <div>
                <span className="text-sm text-gray-500">Country</span>
                <p className="font-medium text-gray-900">
                  <CountryFlag countryCode={countryCode} /> {countryCode}
                </p>
              </div>
              <div>
                <span className="text-sm text-gray-500">Period</span>
                <p className="font-medium text-gray-900">
                  {formatDate(periodStart)} - {formatDate(periodEnd)}
                </p>
              </div>
              <div>
                <span className="text-sm text-gray-500">Total Guests</span>
                <p className="font-medium text-gray-900">{guests.length}</p>
              </div>
            </div>

            <div className="overflow-hidden rounded-lg border border-gray-200">
              <table className="min-w-full divide-y divide-gray-200">
                <thead className="bg-gray-50">
                  <tr>
                    <th className="px-4 py-3 text-left text-xs font-medium uppercase tracking-wider text-gray-500">
                      Guest
                    </th>
                    <th className="px-4 py-3 text-left text-xs font-medium uppercase tracking-wider text-gray-500">
                      Document
                    </th>
                    <th className="px-4 py-3 text-left text-xs font-medium uppercase tracking-wider text-gray-500">
                      Stay Period
                    </th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-gray-200 bg-white">
                  {guests.map((guest) => (
                    <tr key={guest.id}>
                      <td className="whitespace-nowrap px-4 py-3">
                        <div>
                          <div className="font-medium text-gray-900">
                            {guest.firstName} {guest.lastName}
                          </div>
                          <div className="text-sm text-gray-500">
                            Born: {formatDate(guest.dateOfBirth)} | {guest.nationality}
                          </div>
                        </div>
                      </td>
                      <td className="whitespace-nowrap px-4 py-3">
                        <div className="text-sm text-gray-900">{guest.documentType}</div>
                        <div className="text-sm text-gray-500">{guest.documentNumber}</div>
                      </td>
                      <td className="whitespace-nowrap px-4 py-3">
                        <div className="text-sm text-gray-900">
                          {formatDate(guest.checkInDate)} - {formatDate(guest.checkOutDate)}
                        </div>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>

            {guests.length === 0 && (
              <div className="py-8 text-center text-gray-500">
                No guests to report in this period.
              </div>
            )}
          </div>

          {isSubmitting && progress > 0 && (
            <div className="px-6">
              <div className="mb-2 flex justify-between text-sm">
                <span className="text-gray-600">Submitting to government portal...</span>
                <span className="font-medium text-blue-600">{progress}%</span>
              </div>
              <div className="h-2 w-full overflow-hidden rounded-full bg-gray-200">
                <div
                  className="h-full rounded-full bg-blue-600 transition-all duration-300"
                  style={{ width: `${progress}%` }}
                />
              </div>
            </div>
          )}

          <div className="flex items-center justify-end gap-3 border-t border-gray-200 px-6 py-4">
            <button
              type="button"
              onClick={onClose}
              disabled={isSubmitting}
              className="rounded-md border border-gray-300 bg-white px-4 py-2 text-sm font-medium text-gray-700 hover:bg-gray-50 disabled:opacity-50"
            >
              Cancel
            </button>
            <button
              type="button"
              onClick={handleConfirm}
              disabled={isSubmitting || guests.length === 0}
              className="inline-flex items-center gap-2 rounded-md bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {isSubmitting ? (
                <>
                  <svg
                    className="h-4 w-4 animate-spin"
                    fill="none"
                    viewBox="0 0 24 24"
                    aria-hidden="true"
                  >
                    <circle
                      className="opacity-25"
                      cx="12"
                      cy="12"
                      r="10"
                      stroke="currentColor"
                      strokeWidth="4"
                    />
                    <path
                      className="opacity-75"
                      fill="currentColor"
                      d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"
                    />
                  </svg>
                  Submitting...
                </>
              ) : (
                <>
                  <svg
                    className="h-4 w-4"
                    fill="none"
                    viewBox="0 0 24 24"
                    stroke="currentColor"
                    aria-hidden="true"
                  >
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      strokeWidth={2}
                      d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"
                    />
                  </svg>
                  Submit to Portal
                </>
              )}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

export type { GuestData };
