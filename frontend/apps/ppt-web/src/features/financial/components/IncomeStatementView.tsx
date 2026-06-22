/**
 * IncomeStatementView component (Story 11.7).
 *
 * Renders an income statement: revenue lines, expense lines, and the derived
 * totals / net income.
 */

import type { IncomeStatement } from '@ppt/api-client';
import { formatCurrency, formatDate } from '../utils/formatting';

interface IncomeStatementViewProps {
  data?: IncomeStatement;
  isLoading?: boolean;
  currency?: string;
}

function SectionRows({ lines, currency }: { lines: IncomeStatement['revenue']; currency: string }) {
  if (lines.length === 0) {
    return (
      <tr>
        <td colSpan={2} className="px-6 py-4 text-sm text-gray-400 text-center">
          No entries
        </td>
      </tr>
    );
  }
  return (
    <>
      {lines.map((line) => (
        <tr key={line.category}>
          <td className="px-6 py-3 text-sm text-gray-700">{line.category}</td>
          <td className="px-6 py-3 text-sm text-right text-gray-700 tabular-nums">
            {formatCurrency(line.amount, currency)}
          </td>
        </tr>
      ))}
    </>
  );
}

export function IncomeStatementView({
  data,
  isLoading,
  currency = 'EUR',
}: IncomeStatementViewProps) {
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
          Select a date range to view the income statement.
        </p>
      </div>
    );
  }

  return (
    <div className="bg-white rounded-lg shadow overflow-hidden">
      <div className="px-6 py-4 border-b border-gray-200">
        <h3 className="text-lg font-medium text-gray-900">Income Statement</h3>
        <p className="mt-1 text-sm text-gray-500">
          {formatDate(data.from_date)} – {formatDate(data.to_date)}
        </p>
      </div>

      <div className="overflow-x-auto">
        <table className="min-w-full divide-y divide-gray-200">
          <tbody className="divide-y divide-gray-200">
            <tr className="bg-gray-50">
              <th
                scope="col"
                className="px-6 py-2 text-left text-xs font-semibold text-gray-600 uppercase tracking-wider"
              >
                Revenue
              </th>
              <th
                scope="col"
                className="px-6 py-2 text-right text-xs font-semibold text-gray-600 uppercase tracking-wider"
              >
                Amount
              </th>
            </tr>
            <SectionRows lines={data.revenue} currency={currency} />
            <tr className="bg-gray-50/60">
              <td className="px-6 py-3 text-sm font-medium text-gray-900">Total Revenue</td>
              <td className="px-6 py-3 text-sm text-right font-medium text-gray-900 tabular-nums">
                {formatCurrency(data.total_revenue, currency)}
              </td>
            </tr>

            <tr className="bg-gray-50">
              <th
                scope="col"
                className="px-6 py-2 text-left text-xs font-semibold text-gray-600 uppercase tracking-wider"
              >
                Expenses
              </th>
              <th
                scope="col"
                className="px-6 py-2 text-right text-xs font-semibold text-gray-600 uppercase tracking-wider"
              >
                Amount
              </th>
            </tr>
            <SectionRows lines={data.expenses} currency={currency} />
            <tr className="bg-gray-50/60">
              <td className="px-6 py-3 text-sm font-medium text-gray-900">Total Expenses</td>
              <td className="px-6 py-3 text-sm text-right font-medium text-gray-900 tabular-nums">
                {formatCurrency(data.total_expenses, currency)}
              </td>
            </tr>
          </tbody>
          <tfoot className="bg-gray-100">
            <tr>
              <td className="px-6 py-4 text-sm font-bold text-gray-900">Net Income</td>
              <td
                className={`px-6 py-4 text-sm text-right font-bold tabular-nums ${
                  data.net_income < 0 ? 'text-red-600' : 'text-green-700'
                }`}
              >
                {formatCurrency(data.net_income, currency)}
              </td>
            </tr>
          </tfoot>
        </table>
      </div>
    </div>
  );
}
