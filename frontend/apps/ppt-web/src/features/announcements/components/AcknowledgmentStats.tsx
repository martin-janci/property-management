/**
 * AcknowledgmentStats Component (Story 6.2)
 *
 * Displays acknowledgment statistics for managers.
 */

import type { AcknowledgmentStats as AcknowledgmentStatsType } from '@ppt/api-client';
import { useTranslation } from 'react-i18next';

interface AcknowledgmentStatsProps {
  stats: AcknowledgmentStatsType;
  className?: string;
}

export function AcknowledgmentStats({ stats, className = '' }: AcknowledgmentStatsProps) {
  const { t } = useTranslation();
  const readPercentage =
    stats.totalTargeted > 0 ? Math.round((stats.readCount / stats.totalTargeted) * 100) : 0;

  const acknowledgedPercentage =
    stats.totalTargeted > 0 ? Math.round((stats.acknowledgedCount / stats.totalTargeted) * 100) : 0;

  return (
    <div className={`bg-gray-50 rounded-lg p-4 ${className}`}>
      <h4 className="text-sm font-medium text-gray-700 mb-3">
        {t('announcements.ackStats.title')}
      </h4>

      <div className="grid grid-cols-2 gap-4">
        {/* Read Stats */}
        <div>
          <div className="flex items-baseline gap-1">
            <span className="text-2xl font-semibold text-blue-600">{stats.readCount}</span>
            <span className="text-sm text-gray-500">/ {stats.totalTargeted}</span>
          </div>
          <div className="text-xs text-gray-500">
            {t('announcements.ackStats.read', { percent: readPercentage })}
          </div>
          <div className="mt-1 h-2 bg-gray-200 rounded-full overflow-hidden">
            <div
              className="h-full bg-blue-500 rounded-full transition-all duration-300"
              style={{ width: `${readPercentage}%` }}
            />
          </div>
        </div>

        {/* Acknowledged Stats */}
        <div>
          <div className="flex items-baseline gap-1">
            <span className="text-2xl font-semibold text-green-600">{stats.acknowledgedCount}</span>
            <span className="text-sm text-gray-500">/ {stats.totalTargeted}</span>
          </div>
          <div className="text-xs text-gray-500">
            {t('announcements.ackStats.acknowledged', { percent: acknowledgedPercentage })}
          </div>
          <div className="mt-1 h-2 bg-gray-200 rounded-full overflow-hidden">
            <div
              className="h-full bg-green-500 rounded-full transition-all duration-300"
              style={{ width: `${acknowledgedPercentage}%` }}
            />
          </div>
        </div>
      </div>

      {/* Pending Count */}
      {stats.pendingCount > 0 && (
        <div className="mt-3 pt-3 border-t border-gray-200">
          <span className="text-sm text-gray-600">
            <span className="font-medium text-amber-600">{stats.pendingCount}</span>{' '}
            {t('announcements.ackStats.pending', { count: stats.pendingCount })}
          </span>
        </div>
      )}
    </div>
  );
}
