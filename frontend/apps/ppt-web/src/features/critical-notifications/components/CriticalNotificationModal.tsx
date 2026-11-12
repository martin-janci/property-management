/**
 * CriticalNotificationModal Component (Epic 8A, Story 8A.2)
 *
 * A modal that displays critical notifications and requires acknowledgment.
 * This modal blocks the UI until all notifications are acknowledged.
 */

import type { CriticalNotificationResponse } from '@ppt/api-client';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';

interface CriticalNotificationModalProps {
  notifications: CriticalNotificationResponse[];
  onAcknowledge: (notificationId: string) => Promise<void>;
  isAcknowledging?: boolean;
}

export function CriticalNotificationModal({
  notifications,
  onAcknowledge,
  isAcknowledging = false,
}: CriticalNotificationModalProps) {
  const { t } = useTranslation();
  const [currentIndex, setCurrentIndex] = useState(0);

  if (notifications.length === 0) return null;

  const currentNotification = notifications[currentIndex];
  const hasMore = currentIndex < notifications.length - 1;

  const handleAcknowledge = async () => {
    await onAcknowledge(currentNotification.id);
    if (hasMore) {
      setCurrentIndex((prev) => prev + 1);
    }
  };

  const formatDate = (dateString: string) => {
    return new Date(dateString).toLocaleDateString(undefined, {
      year: 'numeric',
      month: 'long',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  };

  return (
    <dialog className="fixed inset-0 z-50 overflow-y-auto bg-transparent" open aria-modal="true">
      {/* Backdrop - no click to close for critical notifications */}
      <div className="fixed inset-0 bg-black bg-opacity-50 transition-opacity" />

      {/* Modal */}
      <div className="flex min-h-full items-center justify-center p-4">
        <div className="relative transform overflow-hidden rounded-lg bg-white text-left shadow-xl transition-all sm:my-8 sm:w-full sm:max-w-lg">
          {/* Header */}
          <div className="bg-red-600 px-4 py-3 sm:px-6">
            <div className="flex items-center">
              <div className="flex-shrink-0">
                <svg
                  className="h-6 w-6 text-white"
                  fill="none"
                  viewBox="0 0 24 24"
                  strokeWidth="1.5"
                  stroke="currentColor"
                  aria-hidden="true"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    d="M12 9v3.75m9-.75a9 9 0 11-18 0 9 9 0 0118 0zm-9 3.75h.008v.008H12v-.008z"
                  />
                </svg>
              </div>
              <div className="ml-3 flex-1">
                <h3 className="text-base font-semibold text-white">
                  {t('criticalNotifications.modal.heading')}
                </h3>
              </div>
              {notifications.length > 1 && (
                <div className="text-sm text-red-200">
                  {t('criticalNotifications.modal.counter', {
                    current: currentIndex + 1,
                    total: notifications.length,
                  })}
                </div>
              )}
            </div>
          </div>

          {/* Content */}
          <div className="px-4 py-5 sm:p-6">
            <h4 className="text-lg font-semibold text-gray-900">{currentNotification.title}</h4>
            <p className="mt-2 text-sm text-gray-500">
              {t('criticalNotifications.modal.postedOn', {
                date: formatDate(currentNotification.createdAt),
              })}
            </p>
            <div className="mt-4">
              <p className="text-sm text-gray-700 whitespace-pre-wrap">
                {currentNotification.message}
              </p>
            </div>
          </div>

          {/* Actions */}
          <div className="bg-gray-50 px-4 py-3 sm:flex sm:flex-row-reverse sm:px-6">
            <button
              type="button"
              onClick={handleAcknowledge}
              disabled={isAcknowledging}
              className="inline-flex w-full justify-center rounded-md bg-red-600 px-3 py-2 text-sm font-semibold text-white shadow-sm hover:bg-red-500 disabled:opacity-50 disabled:cursor-not-allowed sm:w-auto"
            >
              {isAcknowledging ? (
                <>
                  <svg
                    className="animate-spin -ml-1 mr-2 h-4 w-4 text-white"
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
                      d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
                    />
                  </svg>
                  {t('criticalNotifications.modal.acknowledging')}
                </>
              ) : hasMore ? (
                t('criticalNotifications.modal.acknowledgeNext')
              ) : (
                t('criticalNotifications.modal.acknowledge')
              )}
            </button>
          </div>
        </div>
      </div>
    </dialog>
  );
}
