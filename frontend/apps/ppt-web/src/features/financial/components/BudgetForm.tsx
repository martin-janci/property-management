/**
 * BudgetForm — create a budget (Epic 24 / Story 52.4).
 *
 * Collects exactly what `POST /api/v1/budgets` accepts: a name, the fiscal year,
 * an optional building (omitted = organisation-wide budget) and notes. Budget
 * lines are added on the budget detail page, because each line needs a budget
 * category and is created with its own `POST /budgets/{id}/items` call.
 */

import { useState } from 'react';
import { useTranslation } from 'react-i18next';

export interface BudgetFormValues {
  name: string;
  fiscalYear: number;
  buildingId: string | null;
  notes: string | null;
}

interface BudgetFormBuilding {
  id: string;
  name: string;
}

interface BudgetFormProps {
  /** Default fiscal year (typically the current year). */
  year: number;
  buildings: BudgetFormBuilding[];
  onSubmit: (values: BudgetFormValues) => void;
  onCancel: () => void;
  isSubmitting?: boolean;
}

export function BudgetForm({ year, buildings, onSubmit, onCancel, isSubmitting }: BudgetFormProps) {
  const { t } = useTranslation();
  const [name, setName] = useState('');
  const [fiscalYear, setFiscalYear] = useState(year);
  const [buildingId, setBuildingId] = useState('');
  const [notes, setNotes] = useState('');
  const [nameError, setNameError] = useState<string | null>(null);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim()) {
      setNameError(t('financial.budgetPages.form.nameRequired'));
      return;
    }
    setNameError(null);
    onSubmit({
      name: name.trim(),
      fiscalYear,
      buildingId: buildingId || null,
      notes: notes.trim() ? notes.trim() : null,
    });
  };

  const inputClass =
    'mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500';

  return (
    <form onSubmit={handleSubmit} className="space-y-6" noValidate>
      <div>
        <label htmlFor="budget-name" className="block text-sm font-medium text-gray-700">
          {t('financial.budgetPages.form.name')} *
        </label>
        <input
          id="budget-name"
          type="text"
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder={t('financial.budgetPages.form.namePlaceholder')}
          aria-invalid={nameError ? true : undefined}
          aria-describedby={nameError ? 'budget-name-error' : undefined}
          className={inputClass}
        />
        {nameError && (
          <p id="budget-name-error" className="mt-1 text-sm text-red-600">
            {nameError}
          </p>
        )}
      </div>

      <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
        <div>
          <label htmlFor="budget-year" className="block text-sm font-medium text-gray-700">
            {t('financial.budgetPages.form.fiscalYear')} *
          </label>
          <select
            id="budget-year"
            value={fiscalYear}
            onChange={(e) => setFiscalYear(Number.parseInt(e.target.value, 10))}
            className={inputClass}
          >
            {[year - 1, year, year + 1, year + 2].map((y) => (
              <option key={y} value={y}>
                {y}
              </option>
            ))}
          </select>
        </div>

        <div>
          <label htmlFor="budget-building" className="block text-sm font-medium text-gray-700">
            {t('financial.budgetPages.form.building')}
          </label>
          <select
            id="budget-building"
            value={buildingId}
            onChange={(e) => setBuildingId(e.target.value)}
            className={inputClass}
          >
            <option value="">{t('financial.budgetPages.form.wholeOrganization')}</option>
            {buildings.map((b) => (
              <option key={b.id} value={b.id}>
                {b.name}
              </option>
            ))}
          </select>
        </div>
      </div>

      <div>
        <label htmlFor="budget-notes" className="block text-sm font-medium text-gray-700">
          {t('financial.budgetPages.form.notes')}
        </label>
        <textarea
          id="budget-notes"
          value={notes}
          onChange={(e) => setNotes(e.target.value)}
          rows={3}
          className={inputClass}
        />
      </div>

      <p className="text-sm text-gray-500">{t('financial.budgetPages.form.itemsHint')}</p>

      <div className="flex justify-end gap-3 pt-4 border-t">
        <button
          type="button"
          onClick={onCancel}
          disabled={isSubmitting}
          className="px-4 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-md hover:bg-gray-50 disabled:opacity-50"
        >
          {t('financial.budgetPages.cancel')}
        </button>
        <button
          type="submit"
          disabled={isSubmitting}
          className="px-4 py-2 text-sm font-medium text-white bg-blue-600 border border-transparent rounded-md hover:bg-blue-700 disabled:opacity-50"
        >
          {isSubmitting
            ? t('financial.budgetPages.saving')
            : t('financial.budgetPages.form.submit')}
        </button>
      </div>
    </form>
  );
}
