/**
 * BudgetCard component for displaying budget vs actual spending.
 */

import { useTranslation } from 'react-i18next';
import { formatCurrency } from '../utils/formatting';

interface BudgetCardProps {
  category: string;
  budgeted: number;
  actual: number;
  currency?: string;
  period?: string;
}

export function BudgetCard({
  category,
  budgeted,
  actual,
  currency = 'EUR',
  period,
}: BudgetCardProps) {
  const { t } = useTranslation();
  const variance = budgeted - actual;
  const percentUsed = budgeted > 0 ? (actual / budgeted) * 100 : 0;
  const isOverBudget = actual > budgeted;

  const getProgressColor = () => {
    if (percentUsed >= 100) return 'bg-red-500';
    if (percentUsed >= 80) return 'bg-yellow-500';
    return 'bg-green-500';
  };

  return (
    <div className="bg-white rounded-lg shadow p-4">
      <div className="flex items-start justify-between mb-3">
        <div>
          <h4 className="text-sm font-medium text-gray-900">{category}</h4>
          {period && <p className="text-xs text-gray-500">{period}</p>}
        </div>
        {isOverBudget && (
          <span className="px-2 py-0.5 text-xs font-medium bg-red-100 text-red-800 rounded-full">
            {t('financial.budget.overBudget')}
          </span>
        )}
      </div>

      {/* Progress Bar */}
      <div className="mb-3">
        <div className="h-2 bg-gray-200 rounded-full overflow-hidden">
          <div
            className={`h-full ${getProgressColor()} transition-all duration-300`}
            style={{ width: `${Math.min(percentUsed, 100)}%` }}
          />
        </div>
        <div className="flex justify-between mt-1 text-xs text-gray-500">
          <span>{t('financial.budget.percentUsed', { percent: percentUsed.toFixed(0) })}</span>
          <span>
            {t('financial.budget.budgetAmount', { amount: formatCurrency(budgeted, currency) })}
          </span>
        </div>
      </div>

      {/* Values */}
      <div className="grid grid-cols-3 gap-2 text-center">
        <div>
          <p className="text-xs text-gray-500">{t('financial.budget.budget')}</p>
          <p className="text-sm font-medium text-gray-900">{formatCurrency(budgeted, currency)}</p>
        </div>
        <div>
          <p className="text-xs text-gray-500">{t('financial.budget.actual')}</p>
          <p className="text-sm font-medium text-gray-900">{formatCurrency(actual, currency)}</p>
        </div>
        <div>
          <p className="text-xs text-gray-500">{t('financial.budget.variance')}</p>
          <p className={`text-sm font-medium ${variance >= 0 ? 'text-green-600' : 'text-red-600'}`}>
            {variance >= 0 ? '+' : ''}
            {formatCurrency(variance, currency)}
          </p>
        </div>
      </div>
    </div>
  );
}
