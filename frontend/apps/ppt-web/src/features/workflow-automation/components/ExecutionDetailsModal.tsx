/**
 * ExecutionDetailsModal Component
 *
 * Display detailed execution log information.
 * Part of Story 43.3: Execution Monitoring.
 */

import type { ExecutionLog } from '@ppt/api-client';
import { useTranslation } from 'react-i18next';

interface ExecutionDetailsModalProps {
  log: ExecutionLog;
  onClose: () => void;
  onRetry?: (log: ExecutionLog) => void;
}

const statusConfig: Record<
  string,
  { color: string; bgColor: string; icon: string; label: string }
> = {
  pending: {
    color: 'text-gray-700',
    bgColor: 'bg-gray-100',
    icon: '⏳',
    label: 'Pending',
  },
  running: {
    color: 'text-blue-700',
    bgColor: 'bg-blue-100',
    icon: '🔄',
    label: 'Running',
  },
  completed: {
    color: 'text-green-700',
    bgColor: 'bg-green-100',
    icon: '✅',
    label: 'Completed',
  },
  failed: {
    color: 'text-red-700',
    bgColor: 'bg-red-100',
    icon: '❌',
    label: 'Failed',
  },
  cancelled: {
    color: 'text-yellow-700',
    bgColor: 'bg-yellow-100',
    icon: '⚠️',
    label: 'Cancelled',
  },
};

function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms}ms`;
  if (ms < 60000) return `${(ms / 1000).toFixed(2)}s`;
  return `${Math.floor(ms / 60000)}m ${Math.floor((ms % 60000) / 1000)}s`;
}

export function ExecutionDetailsModal({ log, onClose, onRetry }: ExecutionDetailsModalProps) {
  const { t } = useTranslation();
  const status = statusConfig[log.status] ?? statusConfig.pending;

  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50 p-4">
      <div className="bg-white rounded-lg shadow-xl max-w-3xl w-full max-h-[90vh] overflow-hidden flex flex-col">
        {/* Header */}
        <div className="p-6 border-b border-gray-200">
          <div className="flex items-start justify-between">
            <div className="flex items-start gap-3">
              <span className="text-3xl">{status.icon}</span>
              <div>
                <div className="flex items-center gap-2 flex-wrap">
                  <h2 className="text-xl font-bold text-gray-900">{log.ruleName}</h2>
                  <span
                    className={`px-2 py-0.5 text-xs font-medium rounded-full ${status.bgColor} ${status.color}`}
                  >
                    {status.label}
                  </span>
                </div>
                <p className="text-sm text-gray-500 mt-1">
                  Execution ID:{' '}
                  <code className="font-mono text-xs bg-gray-100 px-1 rounded">{log.id}</code>
                </p>
              </div>
            </div>
            <button
              type="button"
              onClick={onClose}
              className="text-gray-400 hover:text-gray-500"
              aria-label={t('aria.closeExecutionDetails')}
            >
              <svg
                className="w-6 h-6"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
                aria-hidden="true"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M6 18L18 6M6 6l12 12"
                />
              </svg>
            </button>
          </div>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto p-6 space-y-6">
          {/* Timing Information */}
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
            <div className="bg-gray-50 rounded-lg p-3">
              <p className="text-xs text-gray-500 uppercase font-medium">Started</p>
              <p className="text-sm font-medium text-gray-900 mt-1">
                {new Date(log.startedAt).toLocaleString()}
              </p>
            </div>
            {log.completedAt && (
              <div className="bg-gray-50 rounded-lg p-3">
                <p className="text-xs text-gray-500 uppercase font-medium">Completed</p>
                <p className="text-sm font-medium text-gray-900 mt-1">
                  {new Date(log.completedAt).toLocaleString()}
                </p>
              </div>
            )}
            {log.duration !== undefined && (
              <div className="bg-gray-50 rounded-lg p-3">
                <p className="text-xs text-gray-500 uppercase font-medium">Duration</p>
                <p className="text-sm font-medium text-gray-900 mt-1">
                  {formatDuration(log.duration)}
                </p>
              </div>
            )}
            {log.triggerType && (
              <div className="bg-gray-50 rounded-lg p-3">
                <p className="text-xs text-gray-500 uppercase font-medium">Trigger</p>
                <p className="text-sm font-medium text-gray-900 mt-1 capitalize">
                  {log.triggerType.replace('_', ' ')}
                </p>
              </div>
            )}
          </div>

          {/* Error Section */}
          {log.status === 'failed' && log.errorMessage && (
            <div className="bg-red-50 border border-red-200 rounded-lg p-4">
              <h3 className="text-sm font-medium text-red-800 mb-2 flex items-center gap-2">
                <svg
                  className="w-4 h-4"
                  fill="none"
                  viewBox="0 0 24 24"
                  stroke="currentColor"
                  aria-hidden="true"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"
                  />
                </svg>
                Error
              </h3>
              <pre className="text-xs text-red-700 font-mono whitespace-pre-wrap overflow-x-auto">
                {log.errorMessage}
              </pre>
              {log.errorStack && (
                <details className="mt-2">
                  <summary className="text-xs text-red-600 cursor-pointer hover:underline">
                    Show stack trace
                  </summary>
                  <pre className="mt-2 text-xs text-red-600 font-mono whitespace-pre-wrap overflow-x-auto">
                    {log.errorStack}
                  </pre>
                </details>
              )}
            </div>
          )}

          {/* Action Results */}
          {log.actionResults && log.actionResults.length > 0 && (
            <div>
              <h3 className="text-sm font-medium text-gray-900 mb-3">Action Results</h3>
              <div className="space-y-2">
                {log.actionResults.map((result, index) => {
                  const actionStatus = statusConfig[result.status] ?? statusConfig.pending;
                  return (
                    <div
                      key={`action-result-${result.actionName}-${index}`}
                      className="flex items-start gap-3 p-3 bg-gray-50 rounded-lg"
                    >
                      <span className="flex-shrink-0 w-6 h-6 bg-white rounded-full flex items-center justify-center text-xs font-medium text-gray-600 shadow-sm">
                        {index + 1}
                      </span>
                      <div className="flex-1 min-w-0">
                        <div className="flex items-center gap-2 mb-1">
                          <span className="font-medium text-gray-900 text-sm">
                            {result.actionName}
                          </span>
                          <span
                            className={`px-1.5 py-0.5 text-xs font-medium rounded ${actionStatus.bgColor} ${actionStatus.color}`}
                          >
                            {actionStatus.label}
                          </span>
                          {result.duration !== undefined && (
                            <span className="text-xs text-gray-400">
                              {formatDuration(result.duration)}
                            </span>
                          )}
                        </div>
                        {result.output !== undefined && result.output !== null && (
                          <p className="text-xs text-gray-500 font-mono bg-white p-2 rounded mt-1">
                            {typeof result.output === 'string'
                              ? result.output
                              : JSON.stringify(result.output, null, 2)}
                          </p>
                        )}
                        {result.error && (
                          <p className="text-xs text-red-600 font-mono bg-red-50 p-2 rounded mt-1">
                            {result.error}
                          </p>
                        )}
                      </div>
                    </div>
                  );
                })}
              </div>
            </div>
          )}

          {/* Trigger Context */}
          {log.triggerContext && (
            <div>
              <h3 className="text-sm font-medium text-gray-900 mb-3">Trigger Context</h3>
              <pre className="text-xs text-gray-700 font-mono bg-gray-50 p-4 rounded-lg overflow-x-auto">
                {JSON.stringify(log.triggerContext, null, 2)}
              </pre>
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="p-6 border-t border-gray-200 flex justify-between">
          <div>
            {log.status === 'failed' && onRetry && (
              <button
                type="button"
                onClick={() => onRetry(log)}
                className="inline-flex items-center px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700"
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
                Retry Execution
              </button>
            )}
          </div>
          <button
            type="button"
            onClick={onClose}
            className="px-4 py-2 text-sm font-medium text-gray-700 bg-gray-100 rounded-md hover:bg-gray-200"
          >
            Close
          </button>
        </div>
      </div>
    </div>
  );
}
