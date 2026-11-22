/**
 * TaxSummaryCard Component
 *
 * Displays year-to-date income, estimated tax, and previous year comparison.
 * Epic 18: Short-Term Rental Integration - UC-29 Tax Export
 */

import { useTranslation } from 'react-i18next';
import { getActiveLocale } from '../../../lib/locale';
import type { TaxSummary } from '../types';

interface TaxSummaryCardProps {
  summary: TaxSummary;
  onExport?: () => void;
  onViewFullReport?: () => void;
  isLoading?: boolean;
}

function formatCurrency(amount: number, currency: string): string {
  return new Intl.NumberFormat(getActiveLocale(), {
    style: 'currency',
    currency,
  }).format(amount);
}

function formatPercentage(value: number): string {
  const sign = value >= 0 ? '+' : '';
  return `${sign}${value.toFixed(1)}%`;
}

export function TaxSummaryCard({
  summary,
  onExport,
  onViewFullReport,
  isLoading,
}: TaxSummaryCardProps) {
  const { t } = useTranslation();
  const isPositiveChange = summary.incomeChangePercent >= 0;

  if (isLoading) {
    return (
      <div className="bg-white rounded-lg shadow p-6">
        <div className="animate-pulse">
          <div className="h-4 bg-gray-200 rounded w-1/3 mb-4" />
          <div className="h-8 bg-gray-200 rounded w-1/2 mb-2" />
          <div className="h-4 bg-gray-200 rounded w-2/3" />
        </div>
      </div>
    );
  }

  return (
    <div className="bg-white rounded-lg shadow overflow-hidden">
      {/* Header */}
      <div className="px-6 py-4 bg-gradient-to-r from-indigo-500 to-purple-600">
        <div className="flex items-center justify-between">
          <div>
            <h3 className="text-lg font-semibold text-white">{t('rentals.tax.summaryTitle')}</h3>
            <p className="text-indigo-100 text-sm">
              {t('rentals.tax.year', { year: summary.year })}
            </p>
          </div>
          <div className="p-2 bg-white/20 rounded-lg">
            <svg
              className="w-6 h-6 text-white"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
              aria-hidden="true"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M9 14l6-6m-5.5.5h.01m4.99 5h.01M19 21V5a2 2 0 00-2-2H7a2 2 0 00-2 2v16l3.5-2 3.5 2 3.5-2 3.5 2zM10 8.5a.5.5 0 11-1 0 .5.5 0 011 0zm5 5a.5.5 0 11-1 0 .5.5 0 011 0z"
              />
            </svg>
          </div>
        </div>
      </div>

      {/* Content */}
      <div className="p-6">
        {/* YTD Income */}
        <div className="mb-6">
          <p className="text-sm font-medium text-gray-500 mb-1">{t('rentals.tax.ytdIncome')}</p>
          <div className="flex items-baseline gap-3">
            <span className="text-3xl font-bold text-gray-900">
              {formatCurrency(summary.ytdIncome, summary.currency)}
            </span>
            <span
              className={`inline-flex items-center px-2 py-0.5 rounded text-sm font-medium ${
                isPositiveChange ? 'bg-green-100 text-green-800' : 'bg-red-100 text-red-800'
              }`}
            >
              {isPositiveChange ? (
                <svg
                  className="w-4 h-4 mr-1"
                  fill="currentColor"
                  viewBox="0 0 20 20"
                  aria-hidden="true"
                >
                  <path
                    fillRule="evenodd"
                    d="M5.293 9.707a1 1 0 010-1.414l4-4a1 1 0 011.414 0l4 4a1 1 0 01-1.414 1.414L11 7.414V15a1 1 0 11-2 0V7.414L6.707 9.707a1 1 0 01-1.414 0z"
                    clipRule="evenodd"
                  />
                </svg>
              ) : (
                <svg
                  className="w-4 h-4 mr-1"
                  fill="currentColor"
                  viewBox="0 0 20 20"
                  aria-hidden="true"
                >
                  <path
                    fillRule="evenodd"
                    d="M14.707 10.293a1 1 0 010 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 111.414-1.414L9 12.586V5a1 1 0 012 0v7.586l2.293-2.293a1 1 0 011.414 0z"
                    clipRule="evenodd"
                  />
                </svg>
              )}
              {formatPercentage(summary.incomeChangePercent)}
            </span>
          </div>
          <p className="text-sm text-gray-500 mt-1">
            {t('rentals.tax.vsLastYear', {
              amount: formatCurrency(summary.previousYearIncome, summary.currency),
            })}
          </p>
        </div>

        {/* Metrics Grid */}
        <div className="grid grid-cols-2 gap-4 mb-6">
          <div className="p-3 bg-gray-50 rounded-lg">
            <p className="text-xs font-medium text-gray-500 uppercase tracking-wide">
              {t('rentals.tax.estimatedTax')}
            </p>
            <p className="mt-1 text-xl font-semibold text-gray-900">
              {formatCurrency(summary.estimatedTax, summary.currency)}
            </p>
            <p className="text-xs text-gray-500">
              {t('rentals.tax.atRate', { rate: (summary.taxRate * 100).toFixed(0) })}
            </p>
          </div>
          <div className="p-3 bg-gray-50 rounded-lg">
            <p className="text-xs font-medium text-gray-500 uppercase tracking-wide">
              {t('rentals.tax.netProfit')}
            </p>
            <p className="mt-1 text-xl font-semibold text-green-600">
              {formatCurrency(summary.ytdNetProfit, summary.currency)}
            </p>
            <p className="text-xs text-gray-500">{t('rentals.tax.afterExpenses')}</p>
          </div>
        </div>

        {/* Previous Year Comparison */}
        <div className="border-t pt-4">
          <div className="flex items-center justify-between text-sm">
            <span className="text-gray-500">{t('rentals.tax.previousYearTax')}</span>
            <span className="font-medium text-gray-900">
              {formatCurrency(summary.previousYearTax, summary.currency)}
            </span>
          </div>
          <div className="flex items-center justify-between text-sm mt-2">
            <span className="text-gray-500">{t('rentals.tax.ytdExpenses')}</span>
            <span className="font-medium text-gray-900">
              {formatCurrency(summary.ytdExpenses, summary.currency)}
            </span>
          </div>
        </div>
      </div>

      {/* Actions */}
      <div className="px-6 py-4 bg-gray-50 border-t flex items-center justify-between">
        <button
          type="button"
          onClick={onViewFullReport}
          className="text-sm text-indigo-600 hover:text-indigo-800 font-medium"
        >
          {t('rentals.tax.viewFullReport')}
        </button>
        <button
          type="button"
          onClick={onExport}
          className="inline-flex items-center px-4 py-2 text-sm font-medium text-white bg-indigo-600 rounded-md hover:bg-indigo-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500"
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
              d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"
            />
          </svg>
          {t('rentals.tax.exportReport')}
        </button>
      </div>
    </div>
  );
}
