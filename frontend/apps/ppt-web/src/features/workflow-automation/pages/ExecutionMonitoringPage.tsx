/**
 * ExecutionMonitoringPage
 *
 * Monitor automation execution logs and statistics.
 * Part of Story 43.3: Execution Monitoring.
 */

import type { ExecutionLog, ExecutionStatus } from '@ppt/api-client';
import { useExecutionLogs, useExecutionStats } from '@ppt/api-client';
import { useQueryClient } from '@tanstack/react-query';
import { useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';

import { ExecutionDetailsModal } from '../components/ExecutionDetailsModal';
import { ExecutionLogItem } from '../components/ExecutionLogItem';
import { ExecutionStats } from '../components/ExecutionStats';

type DateRange = 'today' | 'week' | 'month' | 'all';

const UUID_PATTERN = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

const skeletonKeys = ['skeleton-1', 'skeleton-2', 'skeleton-3', 'skeleton-4', 'skeleton-5'];

export function ExecutionMonitoringPage() {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [statusFilter, setStatusFilter] = useState<ExecutionStatus | 'all'>('all');
  const [dateRange, setDateRange] = useState<DateRange>('week');
  const [ruleFilter, setRuleFilter] = useState<string>('');
  const [selectedLog, setSelectedLog] = useState<ExecutionLog | null>(null);

  // Memoised on the selected range: recomputing `new Date()` on every render
  // would change the query key each render and refetch in a loop.
  const since = useMemo(() => {
    const now = new Date();
    switch (dateRange) {
      case 'today': {
        const today = new Date(now);
        today.setHours(0, 0, 0, 0);
        return today.toISOString();
      }
      case 'week': {
        const weekAgo = new Date(now);
        weekAgo.setDate(weekAgo.getDate() - 7);
        return weekAgo.toISOString();
      }
      case 'month': {
        const monthAgo = new Date(now);
        monthAgo.setMonth(monthAgo.getMonth() - 1);
        return monthAgo.toISOString();
      }
      default:
        return undefined;
    }
  }, [dateRange]);

  // The backend filters by exact rule UUID; ignore partial input instead of
  // sending a value the API would reject.
  const trimmedRuleFilter = ruleFilter.trim();
  const ruleId = UUID_PATTERN.test(trimmedRuleFilter) ? trimmedRuleFilter : undefined;

  const {
    data: logsData,
    isLoading: logsLoading,
    error: logsError,
  } = useExecutionLogs({
    page: 1,
    pageSize: 50,
    ...(statusFilter !== 'all' && { status: statusFilter }),
    ...(since && { since }),
    ...(ruleId && { ruleId }),
  });

  const { data: statsData, isLoading: statsLoading } = useExecutionStats({
    since,
    ...(ruleId && { ruleId }),
  });

  const handleViewDetails = (log: ExecutionLog) => {
    setSelectedLog(log);
  };

  const logs = logsData?.data ?? [];
  const totalLogs = logsData?.total ?? 0;

  const stats = statsData ?? {
    totalExecutions: 0,
    successfulExecutions: 0,
    failedExecutions: 0,
    pendingExecutions: 0,
  };

  return (
    <div className="max-w-6xl mx-auto px-4 py-8">
      {/* Header */}
      <div className="mb-8">
        <h1 className="text-2xl font-bold text-gray-900">{t('automation.execution.title')}</h1>
        <p className="mt-1 text-sm text-gray-500">{t('automation.execution.subtitle')}</p>
      </div>

      {/* Stats */}
      <div className="mb-8">
        <ExecutionStats stats={stats} isLoading={statsLoading} />
      </div>

      {/* Filters */}
      <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-4 mb-6">
        <div className="flex flex-wrap gap-4">
          {/* Date Range */}
          <div>
            <label htmlFor="date-range" className="sr-only">
              {t('automation.execution.dateRangeLabel')}
            </label>
            <select
              id="date-range"
              value={dateRange}
              onChange={(e) => setDateRange(e.target.value as DateRange)}
              className="px-3 py-2 border border-gray-300 rounded-md text-sm focus:ring-blue-500 focus:border-blue-500"
            >
              <option value="today">{t('automation.execution.rangeToday')}</option>
              <option value="week">{t('automation.execution.rangeWeek')}</option>
              <option value="month">{t('automation.execution.rangeMonth')}</option>
              <option value="all">{t('automation.execution.rangeAll')}</option>
            </select>
          </div>

          {/* Status Filter */}
          <div>
            <label htmlFor="status-filter" className="sr-only">
              {t('automation.execution.statusLabel')}
            </label>
            <select
              id="status-filter"
              value={statusFilter}
              onChange={(e) => setStatusFilter(e.target.value as ExecutionStatus | 'all')}
              className="px-3 py-2 border border-gray-300 rounded-md text-sm focus:ring-blue-500 focus:border-blue-500"
            >
              <option value="all">{t('automation.execution.allStatus')}</option>
              <option value="completed">{t('automation.execution.statusCompleted')}</option>
              <option value="failed">{t('automation.execution.statusFailed')}</option>
              <option value="running">{t('automation.execution.statusRunning')}</option>
              <option value="pending">{t('automation.execution.statusPending')}</option>
              <option value="cancelled">{t('automation.execution.statusCancelled')}</option>
            </select>
          </div>

          {/* Rule Filter */}
          <div className="flex-1 min-w-48">
            <label htmlFor="rule-filter" className="sr-only">
              {t('automation.execution.ruleFilterLabel')}
            </label>
            <input
              id="rule-filter"
              type="text"
              value={ruleFilter}
              onChange={(e) => setRuleFilter(e.target.value)}
              placeholder={t('automation.execution.ruleFilterPlaceholder')}
              className="w-full px-3 py-2 border border-gray-300 rounded-md text-sm focus:ring-blue-500 focus:border-blue-500"
            />
          </div>

          {/* Refresh Button */}
          <button
            type="button"
            onClick={() => {
              queryClient.invalidateQueries({ queryKey: ['execution-logs'] });
              queryClient.invalidateQueries({ queryKey: ['execution-stats'] });
            }}
            className="inline-flex items-center px-3 py-2 text-sm font-medium text-gray-700 bg-gray-100 rounded-md hover:bg-gray-200"
          >
            <svg
              className="w-4 h-4 mr-2"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
              aria-hidden="true"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
              />
            </svg>
            {t('automation.execution.refresh')}
          </button>
        </div>
      </div>

      {/* Error */}
      {logsError && (
        <div className="bg-red-50 border border-red-200 rounded-lg p-4 mb-6">
          <p className="text-red-700">{t('automation.execution.loadError')}</p>
        </div>
      )}

      {/* Execution Logs */}
      {logsLoading ? (
        <div className="space-y-3">
          {skeletonKeys.map((key) => (
            <div
              key={key}
              className="bg-white rounded-lg shadow-sm border border-gray-200 p-4 animate-pulse"
            >
              <div className="flex items-start gap-3">
                <div className="w-8 h-8 bg-gray-200 rounded-full" />
                <div className="flex-1">
                  <div className="h-5 bg-gray-200 rounded w-1/3 mb-2" />
                  <div className="h-4 bg-gray-200 rounded w-1/2" />
                </div>
                <div className="h-8 bg-gray-200 rounded w-16" />
              </div>
            </div>
          ))}
        </div>
      ) : logs.length === 0 ? (
        <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-12 text-center">
          <svg
            className="mx-auto h-16 w-16 text-gray-300"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
            aria-hidden="true"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={1.5}
              d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2m-6 9l2 2 4-4"
            />
          </svg>
          <h3 className="mt-4 text-lg font-medium text-gray-900">
            {t('automation.execution.emptyTitle')}
          </h3>
          <p className="mt-2 text-sm text-gray-500">
            {statusFilter !== 'all' || dateRange !== 'all' || ruleFilter
              ? t('automation.execution.emptyBodyFiltered')
              : t('automation.execution.emptyBody')}
          </p>
          {(statusFilter !== 'all' || dateRange !== 'all' || ruleFilter) && (
            <button
              type="button"
              onClick={() => {
                setStatusFilter('all');
                setDateRange('all');
                setRuleFilter('');
              }}
              className="mt-4 text-sm text-blue-600 hover:text-blue-700"
            >
              {t('automation.execution.clearFilters')}
            </button>
          )}
        </div>
      ) : (
        <>
          <p className="text-sm text-gray-500 mb-4">
            {t('automation.execution.showing', { shown: logs.length, total: totalLogs })}
          </p>
          <div className="space-y-3">
            {logs.map((log) => (
              <ExecutionLogItem key={log.id} log={log} onViewDetails={handleViewDetails} />
            ))}
          </div>
        </>
      )}

      {/* Details Modal */}
      {selectedLog && (
        <ExecutionDetailsModal log={selectedLog} onClose={() => setSelectedLog(null)} />
      )}
    </div>
  );
}
