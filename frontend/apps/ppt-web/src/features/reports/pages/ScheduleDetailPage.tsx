/**
 * ScheduleDetailPage - Report schedule detail with inline execution history table.
 *
 * Story 81.2 - Report Execution History
 *
 * Displays schedule metadata and a paginated execution history table with
 * status filtering, date-range filtering, download and retry actions.
 */

import type { ReportExecution, ReportExecutionStatus, ReportSchedule } from '@ppt/api-client';
import { useCallback, useState } from 'react';
import { type ExecutionFilters, HistoryFilters } from '../components/HistoryFilters';

// ============================================================================
// Sub-components
// ============================================================================

const STATUS_STYLES: Record<ReportExecutionStatus, { bg: string; text: string; label: string }> = {
  pending: { bg: 'bg-gray-100', text: 'text-gray-700', label: 'Pending' },
  running: { bg: 'bg-blue-100', text: 'text-blue-700', label: 'Running' },
  completed: { bg: 'bg-green-100', text: 'text-green-700', label: 'Completed' },
  failed: { bg: 'bg-red-100', text: 'text-red-700', label: 'Failed' },
  cancelled: { bg: 'bg-yellow-100', text: 'text-yellow-700', label: 'Cancelled' },
  skipped: { bg: 'bg-gray-100', text: 'text-gray-500', label: 'Skipped' },
};

function StatusBadge({ status }: { status: ReportExecutionStatus }) {
  const style = STATUS_STYLES[status];
  return (
    <span
      className={`inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium ${style.bg} ${style.text}`}
    >
      {status === 'running' && (
        <svg
          className="animate-spin -ml-0.5 mr-1.5 h-3 w-3"
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
      )}
      {style.label}
    </span>
  );
}

// ============================================================================
// Helpers
// ============================================================================

function formatDuration(startedAt: string, completedAt?: string): string {
  if (!completedAt) return '-';
  const durationMs = new Date(completedAt).getTime() - new Date(startedAt).getTime();
  if (durationMs < 1000) return `${durationMs}ms`;
  if (durationMs < 60_000) return `${Math.round(durationMs / 1000)}s`;
  return `${Math.round(durationMs / 60_000)}m`;
}

function formatDateTime(dateString: string): string {
  return new Date(dateString).toLocaleDateString('en-US', {
    month: 'short',
    day: 'numeric',
    year: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  });
}

function formatFileSize(bytes?: number): string {
  if (!bytes) return '-';
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

const FREQUENCY_LABELS: Record<string, string> = {
  daily: 'Daily',
  weekly: 'Weekly',
  monthly: 'Monthly',
  quarterly: 'Quarterly',
  yearly: 'Yearly',
};

// ============================================================================
// Props
// ============================================================================

export interface ScheduleDetailPageProps {
  schedule: ReportSchedule;
  executions: ReportExecution[];
  isLoading?: boolean;
  hasMore?: boolean;
  onLoadMore?: () => void;
  onDownload?: (executionId: string) => void;
  onRetry?: (executionId: string) => Promise<void>;
  onBack?: () => void;
}

// ============================================================================
// Component
// ============================================================================

export function ScheduleDetailPage({
  schedule,
  executions,
  isLoading,
  hasMore,
  onLoadMore,
  onDownload,
  onRetry,
  onBack,
}: ScheduleDetailPageProps) {
  const [filters, setFilters] = useState<ExecutionFilters>({});
  const [retryingId, setRetryingId] = useState<string | null>(null);

  const handleRetry = useCallback(
    async (executionId: string) => {
      if (!onRetry) return;
      setRetryingId(executionId);
      try {
        await onRetry(executionId);
      } finally {
        setRetryingId(null);
      }
    },
    [onRetry]
  );

  // Client-side filter on already-fetched page
  const filteredExecutions = executions.filter((execution) => {
    if (filters.status && execution.status !== filters.status) return false;

    const executionDate = new Date(execution.startedAt);
    const fromDate = filters.dateFrom ? new Date(filters.dateFrom) : undefined;
    const toDate = filters.dateTo ? new Date(filters.dateTo) : undefined;
    if (toDate) toDate.setHours(23, 59, 59, 999);
    if (fromDate && executionDate < fromDate) return false;
    if (toDate && executionDate > toDate) return false;
    return true;
  });

  // Stats summary
  const completedCount = executions.filter((e) => e.status === 'completed').length;
  const failedCount = executions.filter((e) => e.status === 'failed').length;
  const runningCount = executions.filter((e) => e.status === 'running').length;

  return (
    <div className="min-h-screen bg-gray-100">
      {/* Page header */}
      <div className="bg-white shadow">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-6">
          <div className="flex items-center gap-4">
            {onBack && (
              <button
                type="button"
                onClick={onBack}
                className="p-2 text-gray-400 hover:text-gray-600 rounded-md hover:bg-gray-100"
                aria-label="Back to schedules"
              >
                <svg
                  className="w-5 h-5"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                  aria-hidden="true"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M15 19l-7-7 7-7"
                  />
                </svg>
              </button>
            )}
            <div>
              <h1 className="text-2xl font-bold text-gray-900">{schedule.name}</h1>
              <p className="text-sm text-gray-500 mt-1">Schedule execution history</p>
            </div>
          </div>
        </div>
      </div>

      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8 space-y-6">
        {/* Schedule metadata card */}
        <div className="bg-white rounded-lg shadow p-6">
          <h2 className="text-base font-semibold text-gray-900 mb-4">Schedule details</h2>
          <dl className="grid grid-cols-2 sm:grid-cols-4 gap-4">
            <div>
              <dt className="text-xs font-medium text-gray-500 uppercase tracking-wide">
                Frequency
              </dt>
              <dd className="mt-1 text-sm text-gray-900">
                {FREQUENCY_LABELS[schedule.frequency] ?? schedule.frequency} at {schedule.time}
              </dd>
            </div>
            <div>
              <dt className="text-xs font-medium text-gray-500 uppercase tracking-wide">Format</dt>
              <dd className="mt-1 text-sm text-gray-900 uppercase">{schedule.format}</dd>
            </div>
            <div>
              <dt className="text-xs font-medium text-gray-500 uppercase tracking-wide">Status</dt>
              <dd className="mt-1">
                <span
                  className={`inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium ${
                    schedule.is_active ? 'bg-green-100 text-green-700' : 'bg-gray-100 text-gray-500'
                  }`}
                >
                  {schedule.is_active ? 'Active' : 'Inactive'}
                </span>
              </dd>
            </div>
            <div>
              <dt className="text-xs font-medium text-gray-500 uppercase tracking-wide">
                Recipients
              </dt>
              <dd className="mt-1 text-sm text-gray-900">{schedule.recipients.length}</dd>
            </div>
            {schedule.last_run_at && (
              <div>
                <dt className="text-xs font-medium text-gray-500 uppercase tracking-wide">
                  Last run
                </dt>
                <dd className="mt-1 text-sm text-gray-900">
                  {formatDateTime(schedule.last_run_at)}
                </dd>
              </div>
            )}
            {schedule.next_run_at && schedule.is_active && (
              <div>
                <dt className="text-xs font-medium text-gray-500 uppercase tracking-wide">
                  Next run
                </dt>
                <dd className="mt-1 text-sm text-gray-900">
                  {formatDateTime(schedule.next_run_at)}
                </dd>
              </div>
            )}
          </dl>
        </div>

        {/* Stats bar */}
        <div className="grid grid-cols-3 gap-4">
          <div className="bg-white rounded-lg shadow p-4 text-center">
            <p className="text-2xl font-bold text-green-600">{completedCount}</p>
            <p className="text-xs text-gray-500 mt-1">Completed</p>
          </div>
          <div className="bg-white rounded-lg shadow p-4 text-center">
            <p className="text-2xl font-bold text-red-600">{failedCount}</p>
            <p className="text-xs text-gray-500 mt-1">Failed</p>
          </div>
          <div className="bg-white rounded-lg shadow p-4 text-center">
            <p className="text-2xl font-bold text-blue-600">{runningCount}</p>
            <p className="text-xs text-gray-500 mt-1">Running</p>
          </div>
        </div>

        {/* Execution history table */}
        <div className="bg-white rounded-lg shadow overflow-hidden">
          <div className="px-6 py-4 border-b border-gray-200 flex items-center justify-between">
            <h2 className="text-base font-semibold text-gray-900">Execution history</h2>
            <span className="text-sm text-gray-500">
              {filteredExecutions.length} of {executions.length} executions
            </span>
          </div>

          {/* Filters */}
          <div className="px-6 py-3 border-b border-gray-200 bg-gray-50">
            <HistoryFilters filters={filters} onChange={setFilters} />
          </div>

          {/* Table */}
          {isLoading && executions.length === 0 ? (
            <div className="p-6">
              <div className="animate-pulse space-y-3">
                {[1, 2, 3, 4, 5].map((i) => (
                  <div key={i} className="h-12 bg-gray-200 rounded" />
                ))}
              </div>
            </div>
          ) : filteredExecutions.length === 0 ? (
            <div className="p-12 text-center">
              <svg
                className="w-12 h-12 text-gray-400 mx-auto mb-4"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
                aria-hidden="true"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2"
                />
              </svg>
              <p className="text-gray-500">No executions found</p>
              {Object.keys(filters).length > 0 && (
                <button
                  type="button"
                  onClick={() => setFilters({})}
                  className="mt-2 text-blue-600 hover:text-blue-800 text-sm font-medium"
                >
                  Clear filters
                </button>
              )}
            </div>
          ) : (
            <div className="overflow-x-auto">
              <table className="min-w-full divide-y divide-gray-200">
                <thead className="bg-gray-50">
                  <tr>
                    <th
                      scope="col"
                      className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider"
                    >
                      Status
                    </th>
                    <th
                      scope="col"
                      className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider"
                    >
                      Started at
                    </th>
                    <th
                      scope="col"
                      className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider"
                    >
                      Duration
                    </th>
                    <th
                      scope="col"
                      className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider"
                    >
                      File
                    </th>
                    <th
                      scope="col"
                      className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider"
                    >
                      Size
                    </th>
                    <th
                      scope="col"
                      className="px-6 py-3 text-right text-xs font-medium text-gray-500 uppercase tracking-wider"
                    >
                      Actions
                    </th>
                  </tr>
                </thead>
                <tbody className="bg-white divide-y divide-gray-200">
                  {filteredExecutions.map((execution) => (
                    <tr key={execution.id} className="hover:bg-gray-50">
                      <td className="px-6 py-4 whitespace-nowrap">
                        <StatusBadge status={execution.status} />
                        {execution.status === 'failed' && execution.error && (
                          <p
                            className="text-xs text-red-600 mt-1 max-w-xs truncate"
                            title={execution.error.message}
                          >
                            {execution.error.code}: {execution.error.message}
                          </p>
                        )}
                      </td>
                      <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-900">
                        {formatDateTime(execution.startedAt)}
                      </td>
                      <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-500">
                        {formatDuration(execution.startedAt, execution.completedAt)}
                      </td>
                      <td className="px-6 py-4 text-sm text-gray-500 max-w-[200px] truncate">
                        {execution.fileName ?? '-'}
                      </td>
                      <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-500">
                        {formatFileSize(execution.fileSize)}
                      </td>
                      <td className="px-6 py-4 whitespace-nowrap text-right text-sm font-medium">
                        <div className="flex items-center justify-end gap-2">
                          {execution.status === 'completed' && onDownload && (
                            <button
                              type="button"
                              onClick={() => onDownload(execution.id)}
                              className="inline-flex items-center px-2.5 py-1 text-xs font-medium text-blue-600 bg-blue-50 rounded hover:bg-blue-100"
                            >
                              <svg
                                className="w-3.5 h-3.5 mr-1"
                                fill="none"
                                stroke="currentColor"
                                viewBox="0 0 24 24"
                                aria-hidden="true"
                              >
                                <path
                                  strokeLinecap="round"
                                  strokeLinejoin="round"
                                  strokeWidth={2}
                                  d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"
                                />
                              </svg>
                              Download
                            </button>
                          )}
                          {execution.status === 'failed' && onRetry && (
                            <button
                              type="button"
                              onClick={() => handleRetry(execution.id)}
                              disabled={retryingId === execution.id}
                              className="inline-flex items-center px-2.5 py-1 text-xs font-medium text-orange-600 bg-orange-50 rounded hover:bg-orange-100 disabled:opacity-50"
                            >
                              {retryingId === execution.id ? (
                                <>
                                  <svg
                                    className="animate-spin w-3.5 h-3.5 mr-1"
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
                                  Retrying...
                                </>
                              ) : (
                                <>
                                  <svg
                                    className="w-3.5 h-3.5 mr-1"
                                    fill="none"
                                    stroke="currentColor"
                                    viewBox="0 0 24 24"
                                    aria-hidden="true"
                                  >
                                    <path
                                      strokeLinecap="round"
                                      strokeLinejoin="round"
                                      strokeWidth={2}
                                      d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
                                    />
                                  </svg>
                                  Retry
                                </>
                              )}
                            </button>
                          )}
                        </div>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}

          {/* Load more footer */}
          {hasMore && (
            <div className="px-6 py-4 border-t border-gray-200 bg-gray-50 text-center">
              <button
                type="button"
                onClick={onLoadMore}
                disabled={isLoading}
                className="text-blue-600 hover:text-blue-800 text-sm font-medium disabled:opacity-50"
              >
                {isLoading ? 'Loading...' : 'Load more'}
              </button>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
