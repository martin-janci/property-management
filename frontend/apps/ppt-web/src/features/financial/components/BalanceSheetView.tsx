/**
 * BalanceSheetView component (Story 11.7).
 *
 * Renders a balance sheet: accounts grouped by type with their balances, plus
 * the total assets / liabilities / net equity summary.
 */

import type { BalanceSheetReport } from '@ppt/api-client';
import { Fragment } from 'react';
import { formatCurrency, formatDate } from '../utils/formatting';

interface BalanceSheetViewProps {
  data?: BalanceSheetReport;
  isLoading?: boolean;
  currency?: string;
}

/** Humanise an account_type slug (e.g. `current_asset` → `Current Asset`). */
function formatAccountType(type: string): string {
  return type
    .split(/[_\s]+/)
    .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
    .join(' ');
}

export function BalanceSheetView({ data, isLoading, currency = 'EUR' }: BalanceSheetViewProps) {
  if (isLoading) {
    return (
      <div className="bg-white rounded-lg shadow p-6 animate-pulse space-y-3">
        {[1, 2, 3, 4, 5].map((i) => (
          <div key={i} className="h-8 bg-gray-200 rounded" />
        ))}
      </div>
    );
  }

  if (!data) {
    return (
      <div className="bg-white rounded-lg shadow p-6">
        <p className="text-gray-500 text-center py-8">
          Select an as-of date to view the balance sheet.
        </p>
      </div>
    );
  }

  // Group accounts by their type so the sheet reads section-by-section.
  const groups = data.accounts.reduce<Record<string, BalanceSheetReport['accounts']>>(
    (acc, line) => {
      const bucket = acc[line.account_type] ?? [];
      bucket.push(line);
      acc[line.account_type] = bucket;
      return acc;
    },
    {}
  );
  const groupKeys = Object.keys(groups).sort();

  return (
    <div className="bg-white rounded-lg shadow overflow-hidden">
      <div className="px-6 py-4 border-b border-gray-200">
        <h3 className="text-lg font-medium text-gray-900">Balance Sheet</h3>
        <p className="mt-1 text-sm text-gray-500">As of {formatDate(data.as_of_date)}</p>
      </div>

      <div className="overflow-x-auto">
        <table className="min-w-full divide-y divide-gray-200">
          <tbody className="divide-y divide-gray-200">
            {data.accounts.length === 0 ? (
              <tr>
                <td colSpan={2} className="px-6 py-8 text-sm text-gray-400 text-center">
                  No accounts
                </td>
              </tr>
            ) : (
              groupKeys.map((type) => (
                <Fragment key={type}>
                  <tr className="bg-gray-50">
                    <th
                      scope="col"
                      className="px-6 py-2 text-left text-xs font-semibold text-gray-600 uppercase tracking-wider"
                    >
                      {formatAccountType(type)}
                    </th>
                    <th
                      scope="col"
                      className="px-6 py-2 text-right text-xs font-semibold text-gray-600 uppercase tracking-wider"
                    >
                      Balance
                    </th>
                  </tr>
                  {groups[type].map((line) => (
                    <tr key={line.account_id}>
                      <td className="px-6 py-3 text-sm text-gray-700">{line.account_name}</td>
                      <td className="px-6 py-3 text-sm text-right text-gray-700 tabular-nums">
                        {formatCurrency(line.balance, currency)}
                      </td>
                    </tr>
                  ))}
                </Fragment>
              ))
            )}
          </tbody>
          <tfoot className="bg-gray-100">
            <tr>
              <td className="px-6 py-3 text-sm font-medium text-gray-900">Total Assets</td>
              <td className="px-6 py-3 text-sm text-right font-medium text-gray-900 tabular-nums">
                {formatCurrency(data.total_assets, currency)}
              </td>
            </tr>
            <tr>
              <td className="px-6 py-3 text-sm font-medium text-gray-900">Total Liabilities</td>
              <td className="px-6 py-3 text-sm text-right font-medium text-gray-900 tabular-nums">
                {formatCurrency(data.total_liabilities, currency)}
              </td>
            </tr>
            <tr>
              <td className="px-6 py-4 text-sm font-bold text-gray-900">Net Equity</td>
              <td
                className={`px-6 py-4 text-sm text-right font-bold tabular-nums ${
                  data.net_equity < 0 ? 'text-red-600' : 'text-gray-900'
                }`}
              >
                {formatCurrency(data.net_equity, currency)}
              </td>
            </tr>
          </tfoot>
        </table>
      </div>
    </div>
  );
}
