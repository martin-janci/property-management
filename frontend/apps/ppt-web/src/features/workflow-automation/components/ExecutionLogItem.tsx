/**
 * ExecutionLogItem Component
 *
 * Display a single execution log entry.
 * Part of Story 43.3: Execution Monitoring.
 */

import type { ExecutionLog } from '@ppt/api-client';
import type { TFunction } from 'i18next';
import { useTranslation } from 'react-i18next';

interface ExecutionLogItemProps {
  log: ExecutionLog;
  onViewDetails: (log: ExecutionLog) => void;
  isExpanded?: boolean;
}

const statusConfig: Record<string, { color: string; icon: string; labelKey: string }> = {
  pending: {
    color: 'bg-gray-100 text-gray-700',
    icon: '⏳',
    labelKey: 'automation.execution.statusPending',
  },
  running: {
    color: 'bg-blue-100 text-blue-700',
    icon: '🔄',
    labelKey: 'automation.execution.statusRunning',
  },
  completed: {
    color: 'bg-green-100 text-green-700',
    icon: '✅',
    labelKey: 'automation.execution.statusCompleted',
  },
  failed: {
    color: 'bg-red-100 text-red-700',
    icon: '❌',
    labelKey: 'automation.execution.statusFailed',
  },
  cancelled: {
    color: 'bg-yellow-100 text-yellow-700',
    icon: '⚠️',
    labelKey: 'automation.execution.statusCancelled',
  },
};

function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms}ms`;
  if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
  return `${Math.floor(ms / 60000)}m ${Math.floor((ms % 60000) / 1000)}s`;
}

function formatTimestamp(date: string, t: TFunction): string {
  const d = new Date(date);
  const now = new Date();
  const diff = now.getTime() - d.getTime();

  if (diff < 60000) return t('automation.execution.justNow');
  if (diff < 3600000)
    return t('automation.execution.minutesAgo', { count: Math.floor(diff / 60000) });
  if (diff < 86400000)
    return t('automation.execution.hoursAgo', { count: Math.floor(diff / 3600000) });
  if (diff < 604800000)
    return t('automation.execution.daysAgo', { count: Math.floor(diff / 86400000) });

  return d.toLocaleDateString(undefined, {
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  });
}

export function ExecutionLogItem({ log, onViewDetails, isExpanded }: ExecutionLogItemProps) {
  const { t } = useTranslation();
  const status = statusConfig[log.status] ?? statusConfig.pending;

  return (
    <div
      className={`bg-white border border-gray-200 rounded-lg p-4 transition-shadow ${
        isExpanded ? 'shadow-md' : 'hover:shadow-sm'
      }`}
    >
      <div className="flex items-start justify-between">
        <div className="flex items-start gap-3 flex-1 min-w-0">
          <span className="text-xl flex-shrink-0">{status.icon}</span>
          <div className="min-w-0 flex-1">
            <div className="flex items-center gap-2 flex-wrap mb-1">
              <h4 className="font-medium text-gray-900 truncate">{log.ruleName}</h4>
              <span className={`px-2 py-0.5 text-xs font-medium rounded-full ${status.color}`}>
                {t(status.labelKey)}
              </span>
            </div>
            <div className="flex flex-wrap gap-3 text-xs text-gray-500">
              <span className="flex items-center gap-1">
                <svg
                  className="w-3 h-3"
                  fill="none"
                  viewBox="0 0 24 24"
                  stroke="currentColor"
                  aria-hidden="true"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"
                  />
                </svg>
                {formatTimestamp(log.startedAt, t)}
              </span>
              {log.duration !== undefined && (
                <span className="flex items-center gap-1">
                  <svg
                    className="w-3 h-3"
                    fill="none"
                    viewBox="0 0 24 24"
                    stroke="currentColor"
                    aria-hidden="true"
                  >
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      strokeWidth={2}
                      d="M13 10V3L4 14h7v7l9-11h-7z"
                    />
                  </svg>
                  {formatDuration(log.duration)}
                </span>
              )}
              {log.triggerType && (
                <span className="flex items-center gap-1">
                  <svg
                    className="w-3 h-3"
                    fill="none"
                    viewBox="0 0 24 24"
                    stroke="currentColor"
                    aria-hidden="true"
                  >
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      strokeWidth={2}
                      d="M15 17h5l-1.405-1.405A2.032 2.032 0 0118 14.158V11a6.002 6.002 0 00-4-5.659V5a2 2 0 10-4 0v.341C7.67 6.165 6 8.388 6 11v3.159c0 .538-.214 1.055-.595 1.436L4 17h5m6 0v1a3 3 0 11-6 0v-1m6 0H9"
                    />
                  </svg>
                  {log.triggerType === 'time_based'
                    ? t('automation.execution.scheduled')
                    : log.triggerType === 'event_based'
                      ? t('automation.execution.event')
                      : log.triggerType === 'manual'
                        ? t('automation.execution.manual')
                        : t('automation.execution.condition')}
                </span>
              )}
            </div>
          </div>
        </div>

        <button
          type="button"
          onClick={() => onViewDetails(log)}
          className="flex-shrink-0 ml-4 px-3 py-1 text-xs font-medium text-gray-600 bg-gray-100 rounded hover:bg-gray-200 transition-colors"
        >
          {t('automation.execution.details')}
        </button>
      </div>

      {/* Error Message */}
      {log.status === 'failed' && log.errorMessage && (
        <div className="mt-3 p-3 bg-red-50 rounded-md">
          <p className="text-xs text-red-700 font-mono">{log.errorMessage}</p>
        </div>
      )}

      {/* Action Results Summary */}
      {log.actionResults && log.actionResults.length > 0 && (
        <div className="mt-3 pt-3 border-t border-gray-100">
          <div className="flex items-center gap-2 text-xs">
            <span className="text-gray-500">{t('automation.execution.actionsLabel')}</span>
            <div className="flex gap-1">
              {log.actionResults.map((result, index) => (
                <span
                  key={`action-${result.actionName}-${index}`}
                  className={`w-2 h-2 rounded-full ${
                    result.status === 'completed'
                      ? 'bg-green-500'
                      : result.status === 'failed'
                        ? 'bg-red-500'
                        : result.status === 'skipped'
                          ? 'bg-gray-300'
                          : 'bg-blue-500'
                  }`}
                  title={`${result.actionName}: ${result.status}`}
                />
              ))}
            </div>
            <span className="text-gray-400">
              {t('automation.execution.actionsCompleted', {
                completed: log.actionResults.filter((r) => r.status === 'completed').length,
                total: log.actionResults.length,
              })}
            </span>
          </div>
        </div>
      )}
    </div>
  );
}
