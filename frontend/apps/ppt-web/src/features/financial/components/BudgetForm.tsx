/**
 * BudgetForm component for creating and editing budgets.
 */

import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { formatCurrency } from '../utils/formatting';

interface BudgetCategory {
  name: string;
  amount: number;
}

interface BudgetFormProps {
  initialCategories?: BudgetCategory[];
  year: number;
  currency?: string;
  onSubmit: (categories: BudgetCategory[], year: number) => void;
  onCancel: () => void;
  isSubmitting?: boolean;
}

/** Default category name keys (`financial.budgetForm.defaultCategories.<key>`). */
const DEFAULT_CATEGORY_KEYS = [
  'maintenance',
  'utilities',
  'insurance',
  'managementFees',
  'repairs',
  'cleaning',
  'security',
  'landscaping',
  'reserveFund',
  'other',
] as const;

interface FormErrors {
  categories?: string;
}

export function BudgetForm({
  initialCategories,
  year,
  currency = 'EUR',
  onSubmit,
  onCancel,
  isSubmitting,
}: BudgetFormProps) {
  const { t } = useTranslation();
  const [budgetYear, setBudgetYear] = useState(year);
  const [categories, setCategories] = useState<BudgetCategory[]>(
    () =>
      initialCategories ||
      DEFAULT_CATEGORY_KEYS.map((key) => ({
        name: t(`financial.budgetForm.defaultCategories.${key}`),
        amount: 0,
      }))
  );
  const [errors, setErrors] = useState<FormErrors>({});

  const validate = (): boolean => {
    const newErrors: FormErrors = {};

    const hasValidCategory = categories.some((cat) => cat.name.trim() && cat.amount > 0);
    if (!hasValidCategory) {
      newErrors.categories = t('financial.budgetForm.categoriesRequired');
    }

    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();

    if (!validate()) {
      return;
    }

    const validCategories = categories.filter((cat) => cat.name.trim() && cat.amount > 0);
    onSubmit(validCategories, budgetYear);
  };

  const updateCategory = (index: number, field: 'name' | 'amount', value: string | number) => {
    const newCategories = [...categories];
    newCategories[index] = { ...newCategories[index], [field]: value };
    setCategories(newCategories);
  };

  const addCategory = () => {
    setCategories([...categories, { name: '', amount: 0 }]);
  };

  const removeCategory = (index: number) => {
    setCategories(categories.filter((_, i) => i !== index));
  };

  const totalBudget = categories.reduce((sum, cat) => sum + (cat.amount || 0), 0);

  return (
    <form onSubmit={handleSubmit} className="space-y-6">
      {/* Year Selection */}
      <div>
        <label htmlFor="budget-year" className="block text-sm font-medium text-gray-700">
          {t('financial.budgetForm.year')}
        </label>
        <select
          id="budget-year"
          value={budgetYear}
          onChange={(e) => setBudgetYear(Number.parseInt(e.target.value, 10))}
          className="mt-1 block w-48 px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500"
        >
          {[year - 1, year, year + 1, year + 2].map((y) => (
            <option key={y} value={y}>
              {y}
            </option>
          ))}
        </select>
      </div>

      {/* Budget Categories */}
      <div>
        <div className="flex items-center justify-between mb-3">
          <span className="block text-sm font-medium text-gray-700">
            {t('financial.budgetForm.categories')}
          </span>
          <button
            type="button"
            onClick={addCategory}
            className="text-sm text-blue-600 hover:text-blue-800"
          >
            + {t('financial.budgetForm.addCategory')}
          </button>
        </div>
        {errors.categories && <p className="mb-2 text-sm text-red-600">{errors.categories}</p>}

        <div className="space-y-2">
          {categories.map((category, index) => (
            <div key={index} className="flex items-center gap-3">
              <input
                type="text"
                value={category.name}
                onChange={(e) => updateCategory(index, 'name', e.target.value)}
                placeholder={t('financial.budgetForm.categoryNamePlaceholder')}
                aria-label={t('financial.budgetForm.categoryNamePlaceholder')}
                className="flex-1 px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500"
              />
              <div className="relative w-40">
                <span className="absolute left-3 top-1/2 -translate-y-1/2 text-gray-500 text-sm">
                  {currency}
                </span>
                <input
                  type="number"
                  value={category.amount || ''}
                  onChange={(e) =>
                    updateCategory(index, 'amount', Number.parseFloat(e.target.value) || 0)
                  }
                  min="0"
                  step="0.01"
                  placeholder="0.00"
                  aria-label={t('financial.amount')}
                  className="w-full pl-12 pr-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500"
                />
              </div>
              <button
                type="button"
                onClick={() => removeCategory(index)}
                className="text-red-600 hover:text-red-800 px-2"
                aria-label={t('financial.form.removeRow')}
                title={t('financial.form.removeRow')}
              >
                ✕
              </button>
            </div>
          ))}
        </div>

        {/* Total */}
        <div className="mt-4 pt-4 border-t flex justify-end">
          <div className="text-right">
            <p className="text-sm text-gray-500">{t('financial.budget.totalBudget')}</p>
            <p className="text-xl font-bold text-gray-900">
              {formatCurrency(totalBudget, currency)}
            </p>
          </div>
        </div>
      </div>

      {/* Actions */}
      <div className="flex justify-end gap-3 pt-4 border-t">
        <button
          type="button"
          onClick={onCancel}
          disabled={isSubmitting}
          className="px-4 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-md hover:bg-gray-50 disabled:opacity-50"
        >
          {t('common.cancel')}
        </button>
        <button
          type="submit"
          disabled={isSubmitting}
          className="px-4 py-2 text-sm font-medium text-white bg-blue-600 border border-transparent rounded-md hover:bg-blue-700 disabled:opacity-50"
        >
          {isSubmitting ? t('financial.form.saving') : t('financial.budgetForm.save')}
        </button>
      </div>
    </form>
  );
}
