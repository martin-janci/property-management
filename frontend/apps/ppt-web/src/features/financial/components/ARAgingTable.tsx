/**
 * ARAgingTable component for displaying accounts receivable aging report.
 */

import type { ARReportEntry, ARReportTotals } from '@ppt/api-client';
import { formatCurrency } from '../utils/formatting';

interface ARAgingTableProps {
  entries: ARReportEntry[];
  totals: ARReportTotals;
  isLoading?: boolean;
  onRowClick?: (unitId: string) => void;
}

export function ARAgingTable({ entries, totals, isLoading, onRowClick }: ARAgingTableProps) {
  if (isLoading) {
    return (
      <div className="bg-white rounded-lg shadow p-6">
        <h3 className="text-lg font-medium text-gray-900 mb-4">Accounts Receivable Aging</h3>
        <div className="animate-pulse space-y-3">
          {[1, 2, 3].map((i) => (
            <div key={i} className="h-10 bg-gray-200 rounded" />
          ))}
        </div>
      </div>
    );
  }

  if (entries.length === 0) {
    return (
      <div className="bg-white rounded-lg shadow p-6">
        <h3 className="text-lg font-medium text-gray-900 mb-4">Accounts Receivable Aging</h3>
        <p className="text-gray-500 text-center py-8">No outstanding receivables</p>
      </div>
    );
  }

  return (
    <div className="bg-white rounded-lg shadow overflow-hidden">
      <div className="px-6 py-4 border-b border-gray-200">
        <h3 className="text-lg font-medium text-gray-900">Accounts Receivable Aging</h3>
      </div>

      <div className="overflow-x-auto">
        <table className="min-w-full divide-y divide-gray-200">
          <thead className="bg-gray-50">
            <tr>
              <th
                scope="col"
                className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider"
              >
                Unit
              </th>
              <th
                scope="col"
                className="px-6 py-3 text-right text-xs font-medium text-gray-500 uppercase tracking-wider"
              >
                Current
              </th>
              <th
                scope="col"
                className="px-6 py-3 text-right text-xs font-medium text-gray-500 uppercase tracking-wider"
              >
                1-30 Days
              </th>
              <th
                scope="col"
                className="px-6 py-3 text-right text-xs font-medium text-gray-500 uppercase tracking-wider"
              >
                31-60 Days
              </th>
              <th
                scope="col"
                className="px-6 py-3 text-right text-xs font-medium text-gray-500 uppercase tracking-wider"
              >
                60+ Days
              </th>
              <th
                scope="col"
                className="px-6 py-3 text-right text-xs font-medium text-gray-500 uppercase tracking-wider"
              >
                Total
              </th>
            </tr>
          </thead>
          <tbody className="bg-white divide-y divide-gray-200">
            {entries.map((entry) => (
              <tr
                key={entry.unit_id}
                className={onRowClick ? 'cursor-pointer hover:bg-gray-50' : ''}
                onClick={() => onRowClick?.(entry.unit_id)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter' || e.key === ' ') {
                    e.preventDefault();
                    onRowClick?.(entry.unit_id);
                  }
                }}
                tabIndex={onRowClick ? 0 : undefined}
                role={onRowClick ? 'button' : undefined}
                aria-label={onRowClick ? `View details for unit ${entry.designation}` : undefined}
              >
                <td className="px-6 py-4 whitespace-nowrap text-sm font-medium text-gray-900">
                  {entry.designation}
                </td>
                <td className="px-6 py-4 whitespace-nowrap text-sm text-right text-gray-500">
                  {formatCurrency(entry.current)}
                </td>
                <td className="px-6 py-4 whitespace-nowrap text-sm text-right text-gray-500">
                  {formatCurrency(entry.days_30)}
                </td>
                <td className="px-6 py-4 whitespace-nowrap text-sm text-right text-gray-500">
                  {formatCurrency(entry.days_60)}
                </td>
                <td
                  className={`px-6 py-4 whitespace-nowrap text-sm text-right ${
                    entry.days_90_plus > 0 ? 'text-red-600 font-medium' : 'text-gray-500'
                  }`}
                >
                  {formatCurrency(entry.days_90_plus)}
                </td>
                <td className="px-6 py-4 whitespace-nowrap text-sm text-right font-medium text-gray-900">
                  {formatCurrency(entry.total)}
                </td>
              </tr>
            ))}
          </tbody>
          <tfoot className="bg-gray-50">
            <tr>
              <td className="px-6 py-4 whitespace-nowrap text-sm font-bold text-gray-900">Total</td>
              <td className="px-6 py-4 whitespace-nowrap text-sm text-right font-bold text-gray-900">
                {formatCurrency(totals.current)}
              </td>
              <td className="px-6 py-4 whitespace-nowrap text-sm text-right font-bold text-gray-900">
                {formatCurrency(totals.days_30)}
              </td>
              <td className="px-6 py-4 whitespace-nowrap text-sm text-right font-bold text-gray-900">
                {formatCurrency(totals.days_60)}
              </td>
              <td
                className={`px-6 py-4 whitespace-nowrap text-sm text-right font-bold ${
                  totals.days_90_plus > 0 ? 'text-red-600' : 'text-gray-900'
                }`}
              >
                {formatCurrency(totals.days_90_plus)}
              </td>
              <td className="px-6 py-4 whitespace-nowrap text-sm text-right font-bold text-gray-900">
                {formatCurrency(totals.total)}
              </td>
            </tr>
          </tfoot>
        </table>
      </div>
    </div>
  );
}
