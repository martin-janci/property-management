/**
 * BudgetOverview component for displaying budget summary with charts.
 */

import { useTranslation } from 'react-i18next';
import { formatCurrency } from '../utils/formatting';
import { BudgetCard } from './BudgetCard';

interface BudgetCategory {
  name: string;
  budgeted: number;
  actual: number;
}

interface BudgetOverviewProps {
  categories: BudgetCategory[];
  year: number;
  currency?: string;
  onEditBudget?: () => void;
  isLoading?: boolean;
}

export function BudgetOverview({
  categories,
  year,
  currency = 'EUR',
  onEditBudget,
  isLoading,
}: BudgetOverviewProps) {
  const { t, i18n } = useTranslation();

  if (isLoading) {
    return (
      <div className="bg-white rounded-lg shadow p-6">
        <div className="animate-pulse space-y-4">
          <div className="h-8 bg-gray-200 rounded w-1/3" />
          <div className="grid grid-cols-3 gap-4">
            {[1, 2, 3].map((i) => (
              <div key={i} className="h-32 bg-gray-200 rounded" />
            ))}
          </div>
        </div>
      </div>
    );
  }

  const totalBudgeted = categories.reduce((sum, cat) => sum + cat.budgeted, 0);
  const totalActual = categories.reduce((sum, cat) => sum + cat.actual, 0);
  const totalVariance = totalBudgeted - totalActual;
  const percentUsed = totalBudgeted > 0 ? (totalActual / totalBudgeted) * 100 : 0;

  const overBudgetCategories = categories.filter((cat) => cat.actual > cat.budgeted);

  if (categories.length === 0) {
    return (
      <div className="bg-white rounded-lg shadow p-6">
        <div className="text-center py-8">
          <p className="text-gray-500 mb-4">{t('financial.budget.noBudgetForYear', { year })}</p>
          {onEditBudget && (
            <button
              type="button"
              onClick={onEditBudget}
              className="px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700"
            >
              {t('financial.budgets.createNew')}
            </button>
          )}
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Summary Header */}
      <div className="bg-white rounded-lg shadow p-6">
        <div className="flex items-center justify-between mb-6">
          <div>
            <h3 className="text-lg font-medium text-gray-900">
              {t('financial.budget.overviewTitle', { year })}
            </h3>
            <p className="text-sm text-gray-500">
              {t('financial.budget.percentOfBudgetUsed', {
                percent: percentUsed.toLocaleString(i18n.language, {
                  minimumFractionDigits: 1,
                  maximumFractionDigits: 1,
                }),
              })}
            </p>
          </div>
          {onEditBudget && (
            <button
              type="button"
              onClick={onEditBudget}
              className="px-4 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-md hover:bg-gray-50"
            >
              {t('financial.budget.editBudget')}
            </button>
          )}
        </div>

        {/* Overall Progress */}
        <div className="mb-6">
          <div className="h-4 bg-gray-200 rounded-full overflow-hidden">
            <div
              className={`h-full transition-all duration-500 ${
                percentUsed >= 100
                  ? 'bg-red-500'
                  : percentUsed >= 80
                    ? 'bg-yellow-500'
                    : 'bg-green-500'
              }`}
              style={{ width: `${Math.min(percentUsed, 100)}%` }}
            />
          </div>
        </div>

        {/* Summary Stats */}
        <div className="grid grid-cols-4 gap-4">
          <div className="text-center p-4 bg-gray-50 rounded-lg">
            <p className="text-sm text-gray-500">{t('financial.budget.totalBudget')}</p>
            <p className="text-xl font-bold text-gray-900">
              {formatCurrency(totalBudgeted, currency)}
            </p>
          </div>
          <div className="text-center p-4 bg-gray-50 rounded-lg">
            <p className="text-sm text-gray-500">{t('financial.budget.totalSpent')}</p>
            <p className="text-xl font-bold text-gray-900">
              {formatCurrency(totalActual, currency)}
            </p>
          </div>
          <div className="text-center p-4 bg-gray-50 rounded-lg">
            <p className="text-sm text-gray-500">{t('financial.budget.remaining')}</p>
            <p
              className={`text-xl font-bold ${
                totalVariance >= 0 ? 'text-green-600' : 'text-red-600'
              }`}
            >
              {formatCurrency(totalVariance, currency)}
            </p>
          </div>
          <div className="text-center p-4 bg-gray-50 rounded-lg">
            <p className="text-sm text-gray-500">{t('financial.budget.overBudget')}</p>
            <p
              className={`text-xl font-bold ${
                overBudgetCategories.length > 0 ? 'text-red-600' : 'text-green-600'
              }`}
            >
              {t('financial.budget.categoryCount', { count: overBudgetCategories.length })}
            </p>
          </div>
        </div>

        {/* Warning for over-budget */}
        {overBudgetCategories.length > 0 && (
          <div className="mt-4 p-4 bg-red-50 border border-red-200 rounded-lg">
            <div className="flex items-start">
              <svg
                className="w-5 h-5 text-red-400 mr-2 mt-0.5"
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
              <div>
                <p className="text-sm font-medium text-red-800">
                  {t('financial.budget.alertTitle')}
                </p>
                <p className="text-sm text-red-700 mt-1">
                  {t('financial.budget.alertBody', {
                    count: overBudgetCategories.length,
                    categories: overBudgetCategories.map((cat) => cat.name).join(', '),
                  })}
                </p>
              </div>
            </div>
          </div>
        )}
      </div>

      {/* Category Cards */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
        {categories.map((category) => (
          <BudgetCard
            key={category.name}
            category={category.name}
            budgeted={category.budgeted}
            actual={category.actual}
            currency={currency}
            period={String(year)}
          />
        ))}
      </div>
    </div>
  );
}
