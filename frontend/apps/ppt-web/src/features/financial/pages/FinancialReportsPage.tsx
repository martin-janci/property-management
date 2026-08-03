/**
 * FinancialReportsPage - Story 11.7
 *
 * Financial statement reports: Income Statement, Balance Sheet, and Cash Flow.
 * Each tab has its own date control (from/to range for the statements, as-of for
 * the balance sheet) plus PDF / xlsx export buttons.
 *
 * Presentational only — data fetching, date state, and the export download live
 * in the route wrapper (routes/groups/financial.tsx).
 */

import type {
  BalanceSheetReport,
  CashFlowReport,
  IncomeStatement,
  ReportExportFormat,
  ReportType,
} from '@ppt/api-client';
import { useTranslation } from 'react-i18next';
import { BalanceSheetView, CashFlowView, IncomeStatementView } from '../components';

const TABS: { key: ReportType; label: string }[] = [
  { key: 'income-statement', label: 'Income Statement' },
  { key: 'balance-sheet', label: 'Balance Sheet' },
  { key: 'cash-flow', label: 'Cash Flow' },
];

export interface FinancialReportsPageProps {
  activeTab: ReportType;
  onTabChange: (tab: ReportType) => void;

  /** From/to range — used by income statement and cash flow. */
  fromDate: string;
  toDate: string;
  onFromDateChange: (date: string) => void;
  onToDateChange: (date: string) => void;

  /** As-of date — used by the balance sheet. */
  asOfDate: string;
  onAsOfDateChange: (date: string) => void;

  incomeStatement?: IncomeStatement;
  balanceSheet?: BalanceSheetReport;
  cashFlow?: CashFlowReport;

  isLoading?: boolean;
  error?: string | null;
  currency?: string;

  onExport: (format: ReportExportFormat) => void;
  /** Non-null while an export is downloading, so its button can show progress. */
  exportingFormat?: ReportExportFormat | null;
}

export function FinancialReportsPage({
  activeTab,
  onTabChange,
  fromDate,
  toDate,
  onFromDateChange,
  onToDateChange,
  asOfDate,
  onAsOfDateChange,
  incomeStatement,
  balanceSheet,
  cashFlow,
  isLoading,
  error,
  currency = 'EUR',
  onExport,
  exportingFormat,
}: FinancialReportsPageProps) {
  const { t } = useTranslation();
  const isRange = activeTab !== 'balance-sheet';

  return (
    <div className="min-h-screen bg-gray-100">
      {/* Header */}
      <div className="bg-white shadow">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-6">
          <h1 className="text-2xl font-bold text-gray-900">Financial Reports</h1>
          <p className="mt-1 text-sm text-gray-500">
            Income statement, balance sheet, and cash-flow reports.
          </p>

          {/* Tabs */}
          <div className="mt-4 border-b border-gray-200">
            <nav className="-mb-px flex gap-6" aria-label={t('aria.reportTabs')}>
              {TABS.map((tab) => (
                <button
                  key={tab.key}
                  type="button"
                  onClick={() => onTabChange(tab.key)}
                  className={`whitespace-nowrap border-b-2 py-3 px-1 text-sm font-medium ${
                    activeTab === tab.key
                      ? 'border-blue-600 text-blue-600'
                      : 'border-transparent text-gray-500 hover:border-gray-300 hover:text-gray-700'
                  }`}
                  aria-current={activeTab === tab.key ? 'page' : undefined}
                >
                  {tab.label}
                </button>
              ))}
            </nav>
          </div>

          {/* Date controls + export */}
          <div className="mt-4 flex flex-col gap-4 lg:flex-row lg:items-end lg:justify-between">
            <div className="flex flex-wrap items-end gap-3">
              {isRange ? (
                <>
                  <div className="flex flex-col">
                    <label htmlFor="report-from" className="text-xs font-medium text-gray-600">
                      From
                    </label>
                    <input
                      id="report-from"
                      type="date"
                      value={fromDate}
                      max={toDate || undefined}
                      onChange={(e) => onFromDateChange(e.target.value)}
                      className="mt-1 px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 sm:text-sm"
                    />
                  </div>
                  <div className="flex flex-col">
                    <label htmlFor="report-to" className="text-xs font-medium text-gray-600">
                      To
                    </label>
                    <input
                      id="report-to"
                      type="date"
                      value={toDate}
                      min={fromDate || undefined}
                      onChange={(e) => onToDateChange(e.target.value)}
                      className="mt-1 px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 sm:text-sm"
                    />
                  </div>
                </>
              ) : (
                <div className="flex flex-col">
                  <label htmlFor="report-as-of" className="text-xs font-medium text-gray-600">
                    As of
                  </label>
                  <input
                    id="report-as-of"
                    type="date"
                    value={asOfDate}
                    onChange={(e) => onAsOfDateChange(e.target.value)}
                    className="mt-1 px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 sm:text-sm"
                  />
                </div>
              )}
            </div>

            <div className="flex items-center gap-2">
              <span className="text-sm text-gray-500">Export:</span>
              <button
                type="button"
                onClick={() => onExport('pdf')}
                disabled={!!exportingFormat}
                className="px-3 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-md hover:bg-gray-50 disabled:opacity-50 disabled:cursor-not-allowed"
              >
                {exportingFormat === 'pdf' ? 'Exporting…' : 'PDF'}
              </button>
              <button
                type="button"
                onClick={() => onExport('xlsx')}
                disabled={!!exportingFormat}
                className="px-3 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-md hover:bg-gray-50 disabled:opacity-50 disabled:cursor-not-allowed"
              >
                {exportingFormat === 'xlsx' ? 'Exporting…' : 'Excel'}
              </button>
            </div>
          </div>
        </div>
      </div>

      {/* Main content */}
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        {error ? (
          <div className="bg-red-50 border border-red-200 rounded-lg p-6">
            <p className="text-sm text-red-700">{error}</p>
          </div>
        ) : (
          <>
            {activeTab === 'income-statement' && (
              <IncomeStatementView
                data={incomeStatement}
                isLoading={isLoading}
                currency={currency}
              />
            )}
            {activeTab === 'balance-sheet' && (
              <BalanceSheetView data={balanceSheet} isLoading={isLoading} currency={currency} />
            )}
            {activeTab === 'cash-flow' && (
              <CashFlowView data={cashFlow} isLoading={isLoading} currency={currency} />
            )}
          </>
        )}
      </div>
    </div>
  );
}
