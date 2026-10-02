/**
 * ExecutionStats Component
 *
 * Display execution statistics summary.
 * Part of Story 43.3: Execution Monitoring.
 */

import { useTranslation } from 'react-i18next';

interface ExecutionStatsProps {
  stats: {
    totalExecutions: number;
    successfulExecutions: number;
    failedExecutions: number;
    pendingExecutions: number;
    averageDuration?: number;
    successRate?: number;
  };
  isLoading?: boolean;
}

function formatDuration(ms: number): string {
  if (ms < 1000) return `${Math.round(ms)}ms`;
  if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
  return `${Math.floor(ms / 60000)}m ${Math.floor((ms % 60000) / 1000)}s`;
}

export function ExecutionStats({ stats, isLoading }: ExecutionStatsProps) {
  const { t } = useTranslation();
  if (isLoading) {
    return (
      <div className="grid grid-cols-2 md:grid-cols-5 gap-4">
        {['stat-1', 'stat-2', 'stat-3', 'stat-4', 'stat-5'].map((key) => (
          <div
            key={key}
            className="bg-white rounded-lg shadow-sm border border-gray-200 p-4 animate-pulse"
          >
            <div className="h-3 bg-gray-200 rounded w-1/2 mb-2" />
            <div className="h-6 bg-gray-200 rounded w-2/3" />
          </div>
        ))}
      </div>
    );
  }

  const successRate =
    stats.successRate ??
    (stats.totalExecutions > 0
      ? Math.round((stats.successfulExecutions / stats.totalExecutions) * 100)
      : 0);

  return (
    <div className="grid grid-cols-2 md:grid-cols-5 gap-4">
      {/* Total Executions */}
      <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-4">
        <p className="text-xs text-gray-500 uppercase font-medium">
          {t('automation.execution.statTotal')}
        </p>
        <p className="text-2xl font-bold text-gray-900 mt-1">{stats.totalExecutions}</p>
      </div>

      {/* Successful */}
      <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-4">
        <p className="text-xs text-gray-500 uppercase font-medium">
          {t('automation.execution.statSuccessful')}
        </p>
        <p className="text-2xl font-bold text-green-600 mt-1">{stats.successfulExecutions}</p>
      </div>

      {/* Failed */}
      <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-4">
        <p className="text-xs text-gray-500 uppercase font-medium">
          {t('automation.execution.statFailed')}
        </p>
        <p className="text-2xl font-bold text-red-600 mt-1">{stats.failedExecutions}</p>
      </div>

      {/* Success Rate */}
      <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-4">
        <p className="text-xs text-gray-500 uppercase font-medium">
          {t('automation.execution.statSuccessRate')}
        </p>
        <div className="flex items-baseline gap-1 mt-1">
          <p
            className={`text-2xl font-bold ${successRate >= 90 ? 'text-green-600' : successRate >= 70 ? 'text-yellow-600' : 'text-red-600'}`}
          >
            {successRate}%
          </p>
        </div>
        <div className="mt-2 h-1.5 bg-gray-200 rounded-full overflow-hidden">
          <div
            className={`h-full ${successRate >= 90 ? 'bg-green-500' : successRate >= 70 ? 'bg-yellow-500' : 'bg-red-500'}`}
            style={{ width: `${successRate}%` }}
          />
        </div>
      </div>

      {/* Average Duration */}
      <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-4">
        <p className="text-xs text-gray-500 uppercase font-medium">
          {t('automation.execution.statAvgDuration')}
        </p>
        <p className="text-2xl font-bold text-gray-900 mt-1">
          {stats.averageDuration !== undefined ? formatDuration(stats.averageDuration) : '-'}
        </p>
      </div>
    </div>
  );
}
