/**
 * NotificationStatsCard Component (Epic 8A, Story 8A.2)
 *
 * Displays acknowledgment statistics for a critical notification (admin view).
 */

import type { CriticalNotificationStats } from '@ppt/api-client';
import { useTranslation } from 'react-i18next';

interface NotificationStatsCardProps {
  stats: CriticalNotificationStats;
  isLoading?: boolean;
}

export function NotificationStatsCard({ stats, isLoading = false }: NotificationStatsCardProps) {
  const { t } = useTranslation();

  if (isLoading) {
    return (
      <div className="animate-pulse bg-gray-100 rounded-lg p-4">
        <div className="h-4 bg-gray-200 rounded w-1/2 mb-2" />
        <div className="h-8 bg-gray-200 rounded w-1/4" />
      </div>
    );
  }

  const acknowledgedPercentage =
    stats.totalUsers > 0 ? Math.round((stats.acknowledgedCount / stats.totalUsers) * 100) : 0;

  return (
    <div className="bg-white rounded-lg shadow p-4">
      <h4 className="text-sm font-medium text-gray-500 mb-3">
        {t('criticalNotifications.statsCard.heading')}
      </h4>

      {/* Progress bar */}
      <div className="mb-4">
        <div className="flex justify-between text-sm mb-1">
          <span className="font-medium text-gray-900">
            {t('criticalNotifications.statsCard.percentAcknowledged', {
              percent: acknowledgedPercentage,
            })}
          </span>
          <span className="text-gray-500">
            {t('criticalNotifications.statsCard.ofTotal', {
              done: stats.acknowledgedCount,
              total: stats.totalUsers,
            })}
          </span>
        </div>
        <div className="w-full bg-gray-200 rounded-full h-2.5">
          <div
            className="bg-green-600 h-2.5 rounded-full transition-all duration-300"
            style={{ width: `${acknowledgedPercentage}%` }}
          />
        </div>
      </div>

      {/* Stats grid */}
      <div className="grid grid-cols-3 gap-4 text-center">
        <div>
          <div className="text-2xl font-bold text-gray-900">{stats.totalUsers}</div>
          <div className="text-xs text-gray-500">
            {t('criticalNotifications.statsCard.totalUsers')}
          </div>
        </div>
        <div>
          <div className="text-2xl font-bold text-green-600">{stats.acknowledgedCount}</div>
          <div className="text-xs text-gray-500">
            {t('criticalNotifications.statsCard.acknowledged')}
          </div>
        </div>
        <div>
          <div className="text-2xl font-bold text-amber-600">{stats.pendingCount}</div>
          <div className="text-xs text-gray-500">
            {t('criticalNotifications.statsCard.pending')}
          </div>
        </div>
      </div>
    </div>
  );
}
