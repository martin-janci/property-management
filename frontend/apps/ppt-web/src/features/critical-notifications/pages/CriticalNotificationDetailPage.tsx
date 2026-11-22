/**
 * CriticalNotificationDetailPage - Detail view for a single critical notification.
 * Epic 8A, Story 8A.2
 */

import type { CriticalNotificationResponse, CriticalNotificationStats } from '@ppt/api-client';
import { useTranslation } from 'react-i18next';
import { getActiveLocale } from '../../../lib/locale';
import { NotificationStatsCard } from '../components';

interface CriticalNotificationDetailPageProps {
  notification: CriticalNotificationResponse;
  stats?: CriticalNotificationStats;
  isLoading?: boolean;
  isAcknowledging?: boolean;
  onAcknowledge: () => void;
  onBack: () => void;
  isAdmin?: boolean;
}

export function CriticalNotificationDetailPage({
  notification,
  stats,
  isLoading,
  isAcknowledging,
  onAcknowledge,
  onBack,
  isAdmin = false,
}: CriticalNotificationDetailPageProps) {
  const { t } = useTranslation();

  if (isLoading) {
    return (
      <div className="flex items-center justify-center py-12">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-red-600" />
      </div>
    );
  }

  return (
    <div className="max-w-3xl mx-auto px-4 py-8">
      {/* Back button */}
      <button
        type="button"
        onClick={onBack}
        className="mb-4 text-blue-600 hover:text-blue-800 flex items-center gap-1"
      >
        <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
        </svg>
        {t('criticalNotifications.backToList')}
      </button>

      <div className="bg-white rounded-lg shadow">
        {/* Header */}
        <div className="p-6 border-b">
          <div className="flex items-start justify-between">
            <div className="flex items-start gap-3">
              <div
                className={`p-2 rounded-lg ${
                  notification.isAcknowledged ? 'bg-green-100' : 'bg-red-100'
                }`}
              >
                {notification.isAcknowledged ? (
                  <svg
                    className="w-6 h-6 text-green-600"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      strokeWidth={2}
                      d="M5 13l4 4L19 7"
                    />
                  </svg>
                ) : (
                  <svg
                    className="w-6 h-6 text-red-600"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      strokeWidth={2}
                      d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"
                    />
                  </svg>
                )}
              </div>
              <div>
                <h1 className="text-2xl font-bold text-gray-900">{notification.title}</h1>
                <p className="mt-1 text-sm text-gray-500">
                  {t('criticalNotifications.createdAt')}{' '}
                  {new Date(notification.createdAt).toLocaleString(getActiveLocale())}
                </p>
              </div>
            </div>
            <span
              className={`px-2 py-1 text-xs font-medium rounded ${
                notification.isAcknowledged
                  ? 'bg-green-100 text-green-800'
                  : 'bg-red-100 text-red-800'
              }`}
            >
              {notification.isAcknowledged
                ? t('criticalNotifications.acknowledged')
                : t('criticalNotifications.requiresAction')}
            </span>
          </div>
        </div>

        {/* Message content */}
        <div className="p-6 border-b">
          <h2 className="text-sm font-semibold text-gray-700 mb-3">
            {t('criticalNotifications.message')}
          </h2>
          <div className="prose prose-sm max-w-none">
            <div className="whitespace-pre-wrap text-gray-700">{notification.message}</div>
          </div>
        </div>

        {/* Acknowledgment info */}
        {notification.isAcknowledged && notification.acknowledgedAt && (
          <div className="p-6 border-b bg-green-50">
            <h2 className="text-sm font-semibold text-green-700 mb-2">
              {t('criticalNotifications.acknowledgmentInfo')}
            </h2>
            <p className="text-sm text-green-800">
              {t('criticalNotifications.acknowledgedAt')}{' '}
              {new Date(notification.acknowledgedAt).toLocaleString(getActiveLocale())}
            </p>
          </div>
        )}

        {/* Stats for admins */}
        {isAdmin && stats && (
          <div className="p-6 border-b">
            <h2 className="text-sm font-semibold text-gray-700 mb-3">
              {t('criticalNotifications.acknowledmentStats')}
            </h2>
            <NotificationStatsCard stats={stats} />
          </div>
        )}

        {/* Actions */}
        <div className="p-6 bg-gray-50 flex items-center gap-3">
          {!notification.isAcknowledged && (
            <button
              type="button"
              onClick={onAcknowledge}
              disabled={isAcknowledging}
              className="px-4 py-2 text-sm font-medium text-white bg-red-600 rounded-md hover:bg-red-700 disabled:opacity-50 flex items-center gap-2"
            >
              {isAcknowledging && (
                <span className="animate-spin rounded-full h-4 w-4 border-b-2 border-white" />
              )}
              {t('criticalNotifications.acknowledge')}
            </button>
          )}
          <button
            type="button"
            onClick={onBack}
            className="px-4 py-2 text-sm font-medium text-gray-600 bg-white border border-gray-300 rounded-md hover:bg-gray-50"
          >
            {t('common.back')}
          </button>
        </div>
      </div>
    </div>
  );
}
