/**
 * BudgetManagementPage - Story 52.4
 *
 * Manage budgets: create, track, and compare.
 */

import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { BuildingFilter } from '../components';
import { formatCurrency } from '../utils/formatting';

interface Building {
  id: string;
  name: string;
}

interface Budget {
  id: string;
  name: string;
  year: number;
  totalBudget: number;
  totalActual: number;
  totalVariance: number;
  currency: string;
  status: 'draft' | 'approved' | 'active' | 'closed';
}

export interface BudgetManagementPageProps {
  buildings: Building[];
  budgets: Budget[];
  currentYear: number;
  summary: {
    totalBudget: number;
    totalActual: number;
    overallVariance: number;
    currency: string;
  };
  isLoading?: boolean;
  /**
   * Create / open a budget. Optional: the budget create & detail screens are
   * not wired yet, so the route omits them and the related buttons are hidden
   * (or inert) instead of leading to a not-found page.
   */
  onNavigateToCreate?: () => void;
  onNavigateToDetail?: (budgetId: string) => void;
  onYearChange: (year: number) => void;
  onBuildingChange: (buildingId?: string) => void;
}

const statusColors: Record<string, string> = {
  draft: 'bg-gray-100 text-gray-800',
  approved: 'bg-blue-100 text-blue-800',
  active: 'bg-green-100 text-green-800',
  closed: 'bg-purple-100 text-purple-800',
};

export function BudgetManagementPage({
  buildings,
  budgets,
  currentYear,
  summary,
  isLoading,
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

  if (isLoading) {
    return (
      <div className="flex items-center justify-center py-12">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600" />
      </div>
    );
  }

  const varianceColor = summary.overallVariance >= 0 ? 'text-green-600' : 'text-red-600';

  return (
    <div className="min-h-screen bg-gray-100">
      {/* Header */}
      <div className="bg-white shadow">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-6">
          <div className="flex flex-col lg:flex-row lg:items-center lg:justify-between gap-4">
            <div>
              <h1 className="text-2xl font-bold text-gray-900">{t('financial.budgets.title')}</h1>
              <p className="mt-1 text-sm text-gray-500">{t('financial.budgets.subtitle')}</p>
            </div>
            <div className="flex items-center gap-4">
              <BuildingFilter
                buildings={buildings}
                selectedBuildingId={selectedBuildingId}
                onChange={handleBuildingChange}
              />
              <select
                value={selectedYear}
                onChange={(e) => handleYearChange(Number(e.target.value))}
                className="px-3 py-2 border border-gray-300 rounded-md focus:ring-blue-500 focus:border-blue-500"
              >
                {years.map((year) => (
                  <option key={year} value={year}>
                    {year}
                  </option>
                ))}
              </select>
              {onNavigateToCreate && (
                <button
                  type="button"
                  onClick={onNavigateToCreate}
                  className="px-4 py-2 text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 rounded-md"
                >
                  {t('financial.budgets.createNew')}
                </button>
              )}
            </div>
          </div>
        </div>
      </div>

      {/* Main Content */}
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        {/* Summary */}
        <div className="grid grid-cols-1 md:grid-cols-3 gap-6 mb-8">
          <div className="bg-white rounded-lg shadow p-6">
            <p className="text-sm text-gray-500">Total Budget</p>
            <p className="mt-1 text-2xl font-semibold text-gray-900">
              {formatCurrency(summary.totalBudget, summary.currency)}
            </p>
          </div>
          <div className="bg-white rounded-lg shadow p-6">
            <p className="text-sm text-gray-500">Total Actual</p>
            <p className="mt-1 text-2xl font-semibold text-gray-900">
              {formatCurrency(summary.totalActual, summary.currency)}
            </p>
          </div>
          <div className="bg-white rounded-lg shadow p-6">
            <p className="text-sm text-gray-500">Variance</p>
            <p className={`mt-1 text-2xl font-semibold ${varianceColor}`}>
              {formatCurrency(summary.overallVariance, summary.currency)}
            </p>
          </div>
        </div>

        {/* Budgets Grid */}
        <div>
          <h2 className="text-lg font-semibold text-gray-900 mb-4">
            {t('financial.budgets.budgetsList')}
          </h2>

          {budgets.length === 0 ? (
            <div className="bg-white rounded-lg shadow p-8 text-center">
              <svg
                className="mx-auto h-12 w-12 text-gray-400"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={1.5}
                  d="M9 7h6m0 10v-3m-3 3h.01M9 17h.01M9 14h.01M12 14h.01M15 11h.01M12 11h.01M9 11h.01M7 21h10a2 2 0 002-2V5a2 2 0 00-2-2H7a2 2 0 00-2 2v14a2 2 0 002 2z"
                />
              </svg>
              <p className="mt-4 text-gray-500">{t('financial.budgets.noBudgets')}</p>
              {onNavigateToCreate && (
                <button
                  type="button"
                  onClick={onNavigateToCreate}
                  className="mt-4 px-4 py-2 text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 rounded-md"
                >
                  {t('financial.budgets.createFirst')}
                </button>
              )}
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
                  <div className="flex items-start justify-between">
                    <div>
                      <h3 className="font-semibold text-gray-900">{budget.name}</h3>
                      <p className="text-sm text-gray-500">{budget.year}</p>
                    </div>
                    <span
                      className={`px-2 py-1 text-xs font-medium rounded ${statusColors[budget.status]}`}
                    >
                      {budget.status}
                    </span>
                  </div>
                  <div className="mt-4 space-y-2">
                    <div className="flex justify-between text-sm">
                      <span className="text-gray-500">Budget</span>
                      <span className="font-medium">
                        {formatCurrency(budget.totalBudget, budget.currency)}
                      </span>
                    </div>
                    <div className="flex justify-between text-sm">
                      <span className="text-gray-500">Actual</span>
                      <span className="font-medium">
                        {formatCurrency(budget.totalActual, budget.currency)}
                      </span>
                    </div>
                    <div className="flex justify-between text-sm">
                      <span className="text-gray-500">Variance</span>
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
      </div>
    </div>
  );
}
