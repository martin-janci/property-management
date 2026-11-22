/**
 * ReportPreview component for previewing report results.
 */

import type { ReportField, ReportResult } from '@ppt/api-client';
import { getActiveLocale } from '../../../lib/locale';

interface ReportPreviewProps {
  result: ReportResult | null;
  fields: ReportField[];
  isLoading?: boolean;
  error?: string;
}

function formatValue(value: unknown, type: ReportField['type'], currency = 'EUR'): string {
  if (value === null || value === undefined) return '-';

  switch (type) {
    case 'currency':
      return new Intl.NumberFormat('en-US', {
        style: 'currency',
        currency: currency,
      }).format(value as number);
    case 'percentage':
      return `${(value as number).toFixed(2)}%`;
    case 'number':
      return new Intl.NumberFormat('en-US').format(value as number);
    case 'date':
      return new Date(value as string).toLocaleDateString('en-US', {
        year: 'numeric',
        month: 'short',
        day: 'numeric',
      });
    default:
      return String(value);
  }
}

export function ReportPreview({ result, fields, isLoading, error }: ReportPreviewProps) {
  if (isLoading) {
    return (
      <div className="bg-white rounded-lg shadow p-6">
        <div className="animate-pulse space-y-4">
          <div className="h-6 bg-gray-200 rounded w-1/4" />
          <div className="space-y-2">
            {[1, 2, 3, 4, 5].map((i) => (
              <div key={i} className="h-10 bg-gray-200 rounded" />
            ))}
          </div>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="bg-white rounded-lg shadow p-6">
        <div className="text-center py-8">
          <svg
            className="w-12 h-12 text-red-400 mx-auto mb-4"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
            aria-hidden="true"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"
            />
          </svg>
          <p className="text-red-600 font-medium">Error loading preview</p>
          <p className="text-sm text-gray-500 mt-1">{error}</p>
        </div>
      </div>
    );
  }

  if (!result) {
    return (
      <div className="bg-white rounded-lg shadow p-6">
        <div className="text-center py-8">
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
              d="M9 17v-2m3 2v-4m3 4v-6m2 10H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
            />
          </svg>
          <p className="text-gray-500">Configure your report and click Preview</p>
        </div>
      </div>
    );
  }

  return (
    <div className="bg-white rounded-lg shadow overflow-hidden">
      {/* Header */}
      <div className="px-6 py-4 border-b border-gray-200 flex items-center justify-between">
        <div>
          <h3 className="text-lg font-medium text-gray-900">Preview</h3>
          <p className="text-sm text-gray-500">
            {result.metadata.row_count} rows • {result.metadata.execution_time_ms}ms
          </p>
        </div>
        <span className="text-xs text-gray-400">
          Generated: {new Date(result.generated_at).toLocaleString(getActiveLocale())}
        </span>
      </div>

      {/* Table */}
      {result.data.length === 0 ? (
        <div className="text-center py-8">
          <p className="text-gray-500">No data matches the current filters</p>
        </div>
      ) : (
        <div className="overflow-x-auto">
          <table className="min-w-full divide-y divide-gray-200">
            <thead className="bg-gray-50">
              <tr>
                {fields.map((field) => (
                  <th
                    key={field.id}
                    className={`px-6 py-3 text-xs font-medium text-gray-500 uppercase tracking-wider ${
                      field.type === 'number' ||
                      field.type === 'currency' ||
                      field.type === 'percentage'
                        ? 'text-right'
                        : 'text-left'
                    }`}
                  >
                    {field.name}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody className="bg-white divide-y divide-gray-200">
              {result.data.slice(0, 100).map((row, index) => {
                // Create a stable key from field values to avoid using array index
                const rowKey =
                  fields.map((field) => `${field.id}:${row[field.source] ?? ''}`).join('|') ||
                  `row-${index}`;
                return (
                  <tr key={rowKey} className="hover:bg-gray-50">
                    {fields.map((field) => (
                      <td
                        key={field.id}
                        className={`px-6 py-4 whitespace-nowrap text-sm ${
                          field.type === 'number' ||
                          field.type === 'currency' ||
                          field.type === 'percentage'
                            ? 'text-right text-gray-900'
                            : 'text-left text-gray-500'
                        }`}
                      >
                        {formatValue(row[field.source], field.type)}
                      </td>
                    ))}
                  </tr>
                );
              })}
            </tbody>
            {result.totals && (
              <tfoot className="bg-gray-50">
                <tr>
                  {fields.map((field, index) => (
                    <td
                      key={`total-${field.id}`}
                      className={`px-6 py-4 whitespace-nowrap text-sm font-bold ${
                        field.type === 'number' ||
                        field.type === 'currency' ||
                        field.type === 'percentage'
                          ? 'text-right text-gray-900'
                          : 'text-left text-gray-500'
                      }`}
                    >
                      {index === 0
                        ? 'Total'
                        : result.totals?.[field.source]
                          ? formatValue(result.totals?.[field.source], field.type)
                          : ''}
                    </td>
                  ))}
                </tr>
              </tfoot>
            )}
          </table>
        </div>
      )}

      {result.data.length > 100 && (
        <div className="px-6 py-3 bg-gray-50 border-t text-center">
          <p className="text-sm text-gray-500">
            Showing first 100 rows of {result.metadata.row_count}
          </p>
        </div>
      )}
    </div>
  );
}
