/**
 * BudgetManagementPage - Story 52.4
 *
 * Lists the organisation's budgets for a fiscal year (optionally per building)
 * with budgeted / actual / remaining totals, and links to create and detail.
 */

import type { BudgetStatus } from '@ppt/api-client';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { BuildingFilter } from '../components';
import { BUDGET_STATUS_STYLES } from '../utils/constants';
import { formatCurrency } from '../utils/formatting';

interface Building {
  id: string;
  name: string;
}

export type BudgetListStatus = BudgetStatus;

export interface BudgetListEntry {
  id: string;
  name: string;
  year: number;
  /** Building name, or undefined for an organisation-wide budget. */
  buildingName?: string;
  totalBudget: number;
  totalActual: number;
  /** Budgeted minus actual: positive = under budget, negative = overspent. */
  totalVariance: number;
  currency: string;
  status: BudgetListStatus;
}

export interface BudgetManagementPageProps {
  buildings: Building[];
  budgets: BudgetListEntry[];
  currentYear: number;
  summary: {
    totalBudget: number;
    totalActual: number;
    overallVariance: number;
    currency: string;
  };
  isLoading?: boolean;
  /** Load error message; renders an error state with an optional retry. */
  error?: string | null;
  onRetry?: () => void;
  /**
   * Create / open a budget. Optional so the page can render read-only; when
   * omitted the related buttons are hidden (or inert).
   */
  onNavigateToCreate?: () => void;
  onNavigateToDetail?: (budgetId: string) => void;
  onYearChange: (year: number) => void;
  onBuildingChange: (buildingId?: string) => void;
}

export function BudgetManagementPage({
  buildings,
  budgets,
  currentYear,
  summary,
  isLoading,
  error,
  onRetry,
  onNavigateToCreate,
  onNavigateToDetail,
  onYearChange,
  onBuildingChange,
}: BudgetManagementPageProps) {
  const { t } = useTranslation();
  const [selectedBuildingId, setSelectedBuildingId] = useState<string>();
  const [selectedYear, setSelectedYear] = useState(currentYear);

  const handleBuildingChange = (buildingId?: string) => {
    setSelectedBuildingId(buildingId);
    onBuildingChange(buildingId);
  };

  const handleYearChange = (year: number) => {
    setSelectedYear(year);
    onYearChange(year);
  };

  const years = [currentYear - 2, currentYear - 1, currentYear, currentYear + 1];
  const varianceColor = summary.overallVariance >= 0 ? 'text-green-600' : 'text-red-600';

  const createButton = (labelKey: string, extraClass = '') =>
    onNavigateToCreate && (
      <button
        type="button"
        onClick={onNavigateToCreate}
        className={`px-4 py-2 text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 rounded-md ${extraClass}`}
      >
        {t(labelKey)}
      </button>
    );

  let content: React.ReactNode;
  if (isLoading) {
    content = (
      <div className="flex items-center justify-center py-12" role="status">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600" />
        <span className="sr-only">{t('financial.budgetPages.loading')}</span>
      </div>
    );
  } else if (error) {
    content = (
      <div className="bg-white rounded-lg shadow p-8 text-center" role="alert">
        <p className="text-red-700">{t('financial.budgetPages.list.loadFailed')}</p>
        <p className="mt-1 text-sm text-gray-500">{error}</p>
        {onRetry && (
          <button
            type="button"
            onClick={onRetry}
            className="mt-4 px-4 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-md hover:bg-gray-50"
          >
            {t('financial.budgetPages.retry')}
          </button>
        )}
      </div>
    );
  } else {
    content = (
      <>
        {/* Summary */}
        <div className="grid grid-cols-1 md:grid-cols-3 gap-6 mb-8">
          <div className="bg-white rounded-lg shadow p-6">
            <p className="text-sm text-gray-500">{t('financial.budgetPages.list.totalBudget')}</p>
            <p className="mt-1 text-2xl font-semibold text-gray-900">
              {formatCurrency(summary.totalBudget, summary.currency)}
            </p>
          </div>
          <div className="bg-white rounded-lg shadow p-6">
            <p className="text-sm text-gray-500">{t('financial.budgetPages.list.totalActual')}</p>
            <p className="mt-1 text-2xl font-semibold text-gray-900">
              {formatCurrency(summary.totalActual, summary.currency)}
            </p>
          </div>
          <div className="bg-white rounded-lg shadow p-6">
            <p className="text-sm text-gray-500">{t('financial.budgetPages.list.remaining')}</p>
            <p className={`mt-1 text-2xl font-semibold ${varianceColor}`}>
              {formatCurrency(summary.overallVariance, summary.currency)}
            </p>
          </div>
        </div>

        {/* Budgets Grid */}
        <div>
          <h2 className="text-lg font-semibold text-gray-900 mb-4">
            {t('financial.budgetPages.list.budgetsList')}
          </h2>

          {budgets.length === 0 ? (
            <div className="bg-white rounded-lg shadow p-8 text-center">
              <svg
                className="mx-auto h-12 w-12 text-gray-400"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
                aria-hidden="true"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={1.5}
                  d="M9 7h6m0 10v-3m-3 3h.01M9 17h.01M9 14h.01M12 14h.01M15 11h.01M12 11h.01M9 11h.01M7 21h10a2 2 0 002-2V5a2 2 0 00-2-2H7a2 2 0 00-2 2v14a2 2 0 002 2z"
                />
              </svg>
              <p className="mt-4 text-gray-500">
                {t('financial.budgetPages.list.noBudgets', { year: selectedYear })}
              </p>
              {createButton('financial.budgetPages.list.createFirst', 'mt-4')}
            </div>
          ) : (
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
              {budgets.map((budget) => (
                <button
                  type="button"
                  key={budget.id}
                  onClick={() => onNavigateToDetail?.(budget.id)}
                  disabled={!onNavigateToDetail}
                  className="bg-white rounded-lg shadow p-6 text-left hover:shadow-md transition-shadow"
                >
                  <div className="flex items-start justify-between gap-2">
                    <div>
                      <h3 className="font-semibold text-gray-900">{budget.name}</h3>
                      <p className="text-sm text-gray-500">
                        {budget.year} ·{' '}
                        {budget.buildingName ?? t('financial.budgetPages.wholeOrganization')}
                      </p>
                    </div>
                    <span
                      className={`px-2 py-1 text-xs font-medium rounded whitespace-nowrap ${BUDGET_STATUS_STYLES[budget.status] ?? BUDGET_STATUS_STYLES.draft}`}
                    >
                      {t(`financial.budgetPages.status.${budget.status}`)}
                    </span>
                  </div>
                  <div className="mt-4 space-y-2">
                    <div className="flex justify-between text-sm">
                      <span className="text-gray-500">{t('financial.budgetPages.budgeted')}</span>
                      <span className="font-medium">
                        {formatCurrency(budget.totalBudget, budget.currency)}
                      </span>
                    </div>
                    <div className="flex justify-between text-sm">
                      <span className="text-gray-500">{t('financial.budgetPages.actual')}</span>
                      <span className="font-medium">
                        {formatCurrency(budget.totalActual, budget.currency)}
                      </span>
                    </div>
                    <div className="flex justify-between text-sm">
                      <span className="text-gray-500">{t('financial.budgetPages.remaining')}</span>
                      <span
                        className={`font-medium ${budget.totalVariance >= 0 ? 'text-green-600' : 'text-red-600'}`}
                      >
                        {formatCurrency(budget.totalVariance, budget.currency)}
                      </span>
                    </div>
                  </div>
                </button>
              ))}
            </div>
          )}
        </div>
      </>
    );
  }

  return (
    <div className="min-h-screen bg-gray-100">
      {/* Header */}
      <div className="bg-white shadow">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-6">
          <div className="flex flex-col lg:flex-row lg:items-center lg:justify-between gap-4">
            <div>
              <h1 className="text-2xl font-bold text-gray-900">
                {t('financial.budgetPages.list.title')}
              </h1>
              <p className="mt-1 text-sm text-gray-500">
                {t('financial.budgetPages.list.subtitle')}
              </p>
            </div>
            <div className="flex flex-wrap items-center gap-4">
              <BuildingFilter
                buildings={buildings}
                selectedBuildingId={selectedBuildingId}
                onChange={handleBuildingChange}
              />
              <select
                value={selectedYear}
                onChange={(e) => handleYearChange(Number(e.target.value))}
                aria-label={t('financial.budgetPages.list.yearFilter')}
                className="px-3 py-2 border border-gray-300 rounded-md focus:ring-blue-500 focus:border-blue-500"
              >
                {years.map((year) => (
                  <option key={year} value={year}>
                    {year}
                  </option>
                ))}
              </select>
              {createButton('financial.budgetPages.list.createNew')}
            </div>
          </div>
        </div>
      </div>

      {/* Main Content */}
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">{content}</div>
    </div>
  );
}
