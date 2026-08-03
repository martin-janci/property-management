/**
 * ExecutionHistory - Display report execution history.
 *
 * Story 81.2 - Report Execution History
 */

import type { ReportExecution, ReportExecutionStatus } from '@ppt/api-client';
import { useCallback, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { type ExecutionFilters, HistoryFilters } from './HistoryFilters';

interface ExecutionHistoryProps {
  scheduleId: string;
  scheduleName: string;
  executions: ReportExecution[];
  isLoading?: boolean;
  hasMore?: boolean;
  onLoadMore?: () => void;
  onDownload?: (executionId: string) => void;
  onRetry?: (executionId: string) => Promise<void>;
  onClose: () => void;
}

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

function formatDuration(startedAt: string, completedAt?: string): string {
  if (!completedAt) return '-';

  const start = new Date(startedAt).getTime();
  const end = new Date(completedAt).getTime();
  const durationMs = end - start;

  if (durationMs < 1000) return `${durationMs}ms`;
  if (durationMs < 60000) return `${Math.round(durationMs / 1000)}s`;
  return `${Math.round(durationMs / 60000)}m`;
}

function formatDateTime(dateString: string): string {
  const date = new Date(dateString);
  return date.toLocaleDateString('en-US', {
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

export function ExecutionHistory({
  scheduleName,
  executions,
  isLoading,
  hasMore,
  onLoadMore,
  onDownload,
  onRetry,
  onClose,
}: ExecutionHistoryProps) {
  const { t } = useTranslation();
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

  // Helper function to check if a date is within a range
  const isDateInRange = (date: Date, fromDate?: Date, toDate?: Date): boolean => {
    if (fromDate && date < fromDate) {
      return false;
    }
    if (toDate && date > toDate) {
      return false;
    }
    return true;
  };

  // Filter executions based on filters
  const filteredExecutions = executions.filter((execution) => {
    if (filters.status && execution.status !== filters.status) {
      return false;
    }

    const executionDate = new Date(execution.startedAt);
    const fromDate = filters.dateFrom ? new Date(filters.dateFrom) : undefined;
    const toDate = filters.dateTo ? new Date(filters.dateTo) : undefined;
    if (toDate) {
      toDate.setHours(23, 59, 59, 999);
    }

    if (!isDateInRange(executionDate, fromDate, toDate)) {
      return false;
    }
    return true;
  });

  return (
    <div className="fixed inset-0 z-50 overflow-y-auto">
      {/* Backdrop */}
      <div className="fixed inset-0 bg-black bg-opacity-50" onClick={onClose} />

      {/* Modal */}
      <div className="flex min-h-full items-center justify-center p-4">
        <div className="relative bg-white rounded-lg shadow-xl max-w-4xl w-full max-h-[90vh] flex flex-col">
          {/* Header */}
          <div className="flex items-center justify-between px-6 py-4 border-b border-gray-200">
            <div>
              <h2 className="text-lg font-semibold text-gray-900">Execution History</h2>
              <p className="text-sm text-gray-500 mt-1">{scheduleName}</p>
            </div>
            <button
              type="button"
              onClick={onClose}
              className="text-gray-400 hover:text-gray-500"
              aria-label={t('aria.close')}
            >
              <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M6 18L18 6M6 6l12 12"
                />
              </svg>
            </button>
          </div>

          {/* Filters */}
          <div className="px-6 py-3 border-b border-gray-200 bg-gray-50">
            <HistoryFilters filters={filters} onChange={setFilters} />
          </div>

          {/* Content */}
          <div className="flex-1 overflow-y-auto">
            {isLoading && filteredExecutions.length === 0 ? (
              <div className="p-6">
                <div className="animate-pulse space-y-4">
                  {[1, 2, 3].map((i) => (
                    <div key={i} className="h-16 bg-gray-200 rounded" />
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
              <div className="divide-y divide-gray-200">
                {filteredExecutions.map((execution) => (
                  <div key={execution.id} className="px-6 py-4 hover:bg-gray-50">
                    <div className="flex items-start justify-between">
                      <div className="flex-1">
                        <div className="flex items-center gap-3">
                          <StatusBadge status={execution.status} />
                          <span className="text-sm text-gray-500">
                            {formatDateTime(execution.startedAt)}
                          </span>
                          {execution.completedAt && (
                            <span className="text-xs text-gray-400">
                              Duration: {formatDuration(execution.startedAt, execution.completedAt)}
                            </span>
                          )}
                        </div>

                        {execution.status === 'completed' && execution.fileName && (
                          <div className="mt-2 flex items-center gap-2 text-sm text-gray-600">
                            <svg
                              className="w-4 h-4"
                              fill="none"
                              stroke="currentColor"
                              viewBox="0 0 24 24"
                              aria-hidden="true"
                            >
                              <path
                                strokeLinecap="round"
                                strokeLinejoin="round"
                                strokeWidth={2}
                                d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
                              />
                            </svg>
                            <span>{execution.fileName}</span>
                            <span className="text-gray-400">
                              ({formatFileSize(execution.fileSize)})
                            </span>
                          </div>
                        )}

                        {execution.status === 'failed' && execution.error && (
                          <div className="mt-2 p-2 bg-red-50 rounded-md">
                            <p className="text-sm text-red-700 font-medium">
                              {execution.error.code}: {execution.error.message}
                            </p>
                            {execution.error.details && (
                              <p className="text-xs text-red-600 mt-1">{execution.error.details}</p>
                            )}
                          </div>
                        )}
                      </div>

                      {/* Actions */}
                      <div className="flex items-center gap-2 ml-4">
                        {execution.status === 'completed' && onDownload && (
                          <button
                            type="button"
                            onClick={() => onDownload(execution.id)}
                            className="inline-flex items-center px-3 py-1.5 text-sm font-medium text-blue-600 bg-blue-50 rounded-md hover:bg-blue-100"
                          >
                            <svg
                              className="w-4 h-4 mr-1"
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
                            className="inline-flex items-center px-3 py-1.5 text-sm font-medium text-orange-600 bg-orange-50 rounded-md hover:bg-orange-100 disabled:opacity-50"
                          >
                            {retryingId === execution.id ? (
                              <>
                                <svg
                                  className="animate-spin w-4 h-4 mr-1"
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
                                  className="w-4 h-4 mr-1"
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
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>

          {/* Load More / Footer */}
          {hasMore && (
            <div className="px-6 py-3 border-t border-gray-200 bg-gray-50 text-center">
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

          {/* Summary */}
          <div className="px-6 py-3 border-t border-gray-200 bg-gray-50">
            <div className="flex items-center justify-between text-sm text-gray-500">
              <span>
                Showing {filteredExecutions.length} of {executions.length} executions
              </span>
              <button
                type="button"
                onClick={onClose}
                className="px-4 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-md hover:bg-gray-50"
              >
                Close
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
