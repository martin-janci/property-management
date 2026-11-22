/**
 * UsageChart component - displays usage metrics with visual progress bars.
 */

import { useTranslation } from 'react-i18next';
import { getActiveLocale } from '../../../lib/locale';
import type { UsageMetricType, UsageSummary } from '../types';

interface UsageChartProps {
  usageSummaries: UsageSummary[];
  className?: string;
}

const metricIcons: Record<UsageMetricType, string> = {
  users:
    'M12 4.354a4 4 0 110 5.292M15 21H3v-1a6 6 0 0112 0v1zm0 0h6v-1a6 6 0 00-9-5.197M13 7a4 4 0 11-8 0 4 4 0 018 0z',
  buildings:
    'M19 21V5a2 2 0 00-2-2H7a2 2 0 00-2 2v16m14 0h2m-2 0h-5m-9 0H3m2 0h5M9 7h1m-1 4h1m4-4h1m-1 4h1m-5 10v-5a1 1 0 011-1h2a1 1 0 011 1v5m-4 0h4',
  units:
    'M3 12l2-2m0 0l7-7 7 7M5 10v10a1 1 0 001 1h3m10-11l2 2m-2-2v10a1 1 0 01-1 1h-3m-6 0a1 1 0 001-1v-4a1 1 0 011-1h2a1 1 0 011 1v4a1 1 0 001 1m-6 0h6',
  storage:
    'M4 7v10c0 2.21 3.582 4 8 4s8-1.79 8-4V7M4 7c0 2.21 3.582 4 8 4s8-1.79 8-4M4 7c0-2.21 3.582-4 8-4s8 1.79 8 4m0 5c0 2.21-3.582 4-8 4s-8-1.79-8-4',
  api_calls: 'M8 9l3 3-3 3m5 0h3M5 20h14a2 2 0 002-2V6a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z',
};

const metricColors: Record<UsageMetricType, { bg: string; fill: string; text: string }> = {
  users: { bg: 'bg-blue-100', fill: 'bg-blue-500', text: 'text-blue-600' },
  buildings: { bg: 'bg-purple-100', fill: 'bg-purple-500', text: 'text-purple-600' },
  units: { bg: 'bg-green-100', fill: 'bg-green-500', text: 'text-green-600' },
  storage: { bg: 'bg-yellow-100', fill: 'bg-yellow-500', text: 'text-yellow-600' },
  api_calls: { bg: 'bg-pink-100', fill: 'bg-pink-500', text: 'text-pink-600' },
};

function formatUsageValue(value: number, metricType: UsageMetricType): string {
  if (metricType === 'storage') {
    // Convert to appropriate unit (assuming bytes)
    if (value >= 1073741824) {
      return `${(value / 1073741824).toFixed(1)} GB`;
    }
    if (value >= 1048576) {
      return `${(value / 1048576).toFixed(1)} MB`;
    }
    if (value >= 1024) {
      return `${(value / 1024).toFixed(1)} KB`;
    }
    return `${value} B`;
  }
  if (metricType === 'api_calls') {
    if (value >= 1000000) {
      return `${(value / 1000000).toFixed(1)}M`;
    }
    if (value >= 1000) {
      return `${(value / 1000).toFixed(1)}K`;
    }
  }
  return value.toLocaleString(getActiveLocale());
}

function UsageMetricItem({ summary }: { summary: UsageSummary }) {
  const { t } = useTranslation();
  const colors = metricColors[summary.metricType];
  const icon = metricIcons[summary.metricType];

  const percentage = summary.limit
    ? Math.min((summary.currentUsage / summary.limit) * 100, 100)
    : 0;
  const isNearLimit = percentage >= 80;
  const isAtLimit = percentage >= 100;

  return (
    <div className="p-4 bg-white rounded-lg border border-gray-200">
      <div className="flex items-start justify-between">
        <div className="flex items-center gap-3">
          <div className={`p-2 rounded-lg ${colors.bg}`}>
            <svg
              className={`w-5 h-5 ${colors.text}`}
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d={icon} />
            </svg>
          </div>
          <div>
            <p className="font-medium text-gray-900">
              {t(`subscription.usage.metrics.${summary.metricType}`)}
            </p>
            <p className="text-sm text-gray-500">
              {formatUsageValue(summary.currentUsage, summary.metricType)}
              {summary.limit !== undefined && (
                <>
                  {' / '}
                  {formatUsageValue(summary.limit, summary.metricType)}
                </>
              )}
            </p>
          </div>
        </div>
        {summary.limit !== undefined && (
          <span
            className={`text-sm font-medium ${
              isAtLimit ? 'text-red-600' : isNearLimit ? 'text-yellow-600' : 'text-gray-500'
            }`}
          >
            {percentage.toFixed(0)}%
          </span>
        )}
      </div>

      {summary.limit !== undefined && (
        <div className="mt-3">
          <div className={`h-2 rounded-full ${colors.bg}`}>
            <div
              className={`h-full rounded-full transition-all ${
                isAtLimit ? 'bg-red-500' : isNearLimit ? 'bg-yellow-500' : colors.fill
              }`}
              style={{ width: `${percentage}%` }}
            />
          </div>
          {isNearLimit && !isAtLimit && (
            <p className="mt-1 text-xs text-yellow-600">{t('subscription.usage.nearingLimit')}</p>
          )}
          {isAtLimit && (
            <p className="mt-1 text-xs text-red-600">{t('subscription.usage.atLimit')}</p>
          )}
        </div>
      )}
    </div>
  );
}

export function UsageChart({ usageSummaries, className = '' }: UsageChartProps) {
  const { t } = useTranslation();

  if (usageSummaries.length === 0) {
    return (
      <div className={`bg-white rounded-lg shadow p-6 ${className}`}>
        <h3 className="text-lg font-semibold text-gray-900 mb-4">
          {t('subscription.usage.title')}
        </h3>
        <p className="text-gray-500 text-center py-8">{t('subscription.usage.noData')}</p>
      </div>
    );
  }

  const periodStart = usageSummaries[0]?.periodStart;
  const periodEnd = usageSummaries[0]?.periodEnd;

  return (
    <div className={`bg-white rounded-lg shadow ${className}`}>
      <div className="p-6 border-b">
        <h3 className="text-lg font-semibold text-gray-900">{t('subscription.usage.title')}</h3>
        {periodStart && periodEnd && (
          <p className="text-sm text-gray-500 mt-1">
            {t('subscription.usage.period', {
              start: new Date(periodStart).toLocaleDateString(getActiveLocale()),
              end: new Date(periodEnd).toLocaleDateString(getActiveLocale()),
            })}
          </p>
        )}
      </div>
      <div className="p-6">
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          {usageSummaries.map((summary) => (
            <UsageMetricItem key={summary.metricType} summary={summary} />
          ))}
        </div>
      </div>
    </div>
  );
}
