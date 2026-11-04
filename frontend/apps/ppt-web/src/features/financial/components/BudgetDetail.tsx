/**
 * BudgetDetail — one budget with its lines and lifecycle actions (Epic 24).
 *
 * Presentational: the route wrapper loads the data and performs the calls.
 * What is offered follows the api-server workflow
 * (draft → pending_approval → approved → active → closed):
 *
 *   draft            add / remove lines, submit for approval, delete budget
 *   pending_approval approve
 *   approved         activate
 *   active           record actual spend against a line, close
 *   closed           read-only
 */

import type { Budget, BudgetCategory, BudgetItem, BudgetSummary } from '@ppt/api-client';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { BUDGET_STATUS_STYLES } from '../utils/constants';
import { formatCurrency } from '../utils/formatting';

export type BudgetDetailAction = 'submit' | 'approve' | 'activate' | 'close' | 'delete';

export interface NewBudgetItemValues {
  /** Existing category id, or null when `newCategoryName` should be created first. */
  categoryId: string | null;
  newCategoryName: string | null;
  name: string;
  budgetedAmount: number;
  description: string | null;
}

export interface BudgetActualValues {
  amount: number;
  transactionDate: string;
  description: string | null;
}

export interface BudgetDetailProps {
  budget: Budget;
  buildingName?: string;
  items: BudgetItem[];
  categories: BudgetCategory[];
  summary?: BudgetSummary;
  currency?: string;
  /** The lifecycle action currently in flight (disables the action buttons). */
  pendingAction?: BudgetDetailAction | null;
  onAction: (action: BudgetDetailAction) => void;
  onAddItem: (values: NewBudgetItemValues) => void;
  isAddingItem?: boolean;
  onDeleteItem: (itemId: string) => void;
  onRecordActual: (itemId: string, values: BudgetActualValues) => void;
  isRecordingActual?: boolean;
  onBack: () => void;
}

const NEW_CATEGORY = '__new__';

const inputClass =
  'block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 sm:text-sm';

function todayIso(): string {
  return new Date().toISOString().split('T')[0];
}

function AddItemForm({
  categories,
  onSubmit,
  isSubmitting,
}: {
  categories: BudgetCategory[];
  onSubmit: (values: NewBudgetItemValues) => void;
  isSubmitting?: boolean;
}) {
  const { t } = useTranslation();
  const [categoryId, setCategoryId] = useState(categories[0]?.id ?? NEW_CATEGORY);
  const [newCategoryName, setNewCategoryName] = useState('');
  const [name, setName] = useState('');
  const [amount, setAmount] = useState('');
  const [description, setDescription] = useState('');
  const [error, setError] = useState<string | null>(null);

  const creatingCategory = categoryId === NEW_CATEGORY || categories.length === 0;

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const parsed = Number.parseFloat(amount);
    if (creatingCategory && !newCategoryName.trim()) {
      setError(t('financial.budgetPages.items.categoryRequired'));
      return;
    }
    if (!name.trim()) {
      setError(t('financial.budgetPages.items.nameRequired'));
      return;
    }
    if (!Number.isFinite(parsed) || parsed < 0) {
      setError(t('financial.budgetPages.items.amountInvalid'));
      return;
    }
    setError(null);
    onSubmit({
      categoryId: creatingCategory ? null : categoryId,
      newCategoryName: creatingCategory ? newCategoryName.trim() : null,
      name: name.trim(),
      budgetedAmount: parsed,
      description: description.trim() ? description.trim() : null,
    });
    setName('');
    setAmount('');
    setDescription('');
    setNewCategoryName('');
  };

  return (
    <form
      onSubmit={handleSubmit}
      noValidate
      aria-label={t('financial.budgetPages.items.addTitle')}
      className="border-t border-gray-200 p-4 space-y-3 bg-gray-50"
    >
      <h3 className="text-sm font-semibold text-gray-900">
        {t('financial.budgetPages.items.addTitle')}
      </h3>
      <div className="grid grid-cols-1 md:grid-cols-4 gap-3">
        <div>
          <label htmlFor="budget-item-category" className="block text-xs text-gray-600 mb-1">
            {t('financial.budgetPages.items.category')}
          </label>
          {categories.length > 0 ? (
            <select
              id="budget-item-category"
              value={categoryId}
              onChange={(e) => setCategoryId(e.target.value)}
              className={inputClass}
            >
              {categories.map((c) => (
                <option key={c.id} value={c.id}>
                  {c.name}
                </option>
              ))}
              <option value={NEW_CATEGORY}>{t('financial.budgetPages.items.newCategory')}</option>
            </select>
          ) : (
            <p className="text-xs text-gray-500 py-2">
              {t('financial.budgetPages.items.noCategories')}
            </p>
          )}
        </div>
        {creatingCategory && (
          <div>
            <label htmlFor="budget-item-new-category" className="block text-xs text-gray-600 mb-1">
              {t('financial.budgetPages.items.newCategoryName')}
            </label>
            <input
              id="budget-item-new-category"
              type="text"
              value={newCategoryName}
              onChange={(e) => setNewCategoryName(e.target.value)}
              className={inputClass}
            />
          </div>
        )}
        <div>
          <label htmlFor="budget-item-name" className="block text-xs text-gray-600 mb-1">
            {t('financial.budgetPages.items.name')}
          </label>
          <input
            id="budget-item-name"
            type="text"
            value={name}
            onChange={(e) => setName(e.target.value)}
            className={inputClass}
          />
        </div>
        <div>
          <label htmlFor="budget-item-amount" className="block text-xs text-gray-600 mb-1">
            {t('financial.budgetPages.items.amount')}
          </label>
          <input
            id="budget-item-amount"
            type="number"
            min="0"
            step="0.01"
            inputMode="decimal"
            value={amount}
            onChange={(e) => setAmount(e.target.value)}
            className={inputClass}
          />
        </div>
        <div className={creatingCategory ? 'md:col-span-4' : ''}>
          <label htmlFor="budget-item-description" className="block text-xs text-gray-600 mb-1">
            {t('financial.budgetPages.items.description')}
          </label>
          <input
            id="budget-item-description"
            type="text"
            value={description}
            onChange={(e) => setDescription(e.target.value)}
            className={inputClass}
          />
        </div>
      </div>
      {error && (
        <p className="text-sm text-red-600" role="alert">
          {error}
        </p>
      )}
      <div className="flex justify-end">
        <button
          type="submit"
          disabled={isSubmitting}
          className="px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700 disabled:opacity-50"
        >
          {isSubmitting ? t('financial.budgetPages.saving') : t('financial.budgetPages.items.add')}
        </button>
      </div>
    </form>
  );
}

function RecordActualForm({
  itemName,
  onSubmit,
  onCancel,
  isSubmitting,
}: {
  itemName: string;
  onSubmit: (values: BudgetActualValues) => void;
  onCancel: () => void;
  isSubmitting?: boolean;
}) {
  const { t } = useTranslation();
  const [amount, setAmount] = useState('');
  const [date, setDate] = useState(todayIso());
  const [description, setDescription] = useState('');
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const parsed = Number.parseFloat(amount);
    if (!Number.isFinite(parsed) || parsed <= 0) {
      setError(t('financial.budgetPages.items.amountInvalid'));
      return;
    }
    if (!date) {
      setError(t('financial.budgetPages.actuals.dateRequired'));
      return;
    }
    setError(null);
    onSubmit({
      amount: parsed,
      transactionDate: date,
      description: description.trim() ? description.trim() : null,
    });
  };

  return (
    <form
      onSubmit={handleSubmit}
      noValidate
      aria-label={t('financial.budgetPages.actuals.title', { item: itemName })}
      className="grid grid-cols-1 md:grid-cols-4 gap-3 items-end"
    >
      <div>
        <label htmlFor="budget-actual-amount" className="block text-xs text-gray-600 mb-1">
          {t('financial.budgetPages.actuals.amount')}
        </label>
        <input
          id="budget-actual-amount"
          type="number"
          min="0"
          step="0.01"
          inputMode="decimal"
          value={amount}
          onChange={(e) => setAmount(e.target.value)}
          className={inputClass}
        />
      </div>
      <div>
        <label htmlFor="budget-actual-date" className="block text-xs text-gray-600 mb-1">
          {t('financial.budgetPages.actuals.date')}
        </label>
        <input
          id="budget-actual-date"
          type="date"
          value={date}
          onChange={(e) => setDate(e.target.value)}
          className={inputClass}
        />
      </div>
      <div>
        <label htmlFor="budget-actual-description" className="block text-xs text-gray-600 mb-1">
          {t('financial.budgetPages.items.description')}
        </label>
        <input
          id="budget-actual-description"
          type="text"
          value={description}
          onChange={(e) => setDescription(e.target.value)}
          className={inputClass}
        />
      </div>
      <div className="flex gap-2 justify-end">
        <button
          type="button"
          onClick={onCancel}
          className="px-3 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-md hover:bg-gray-50"
        >
          {t('financial.budgetPages.cancel')}
        </button>
        <button
          type="submit"
          disabled={isSubmitting}
          className="px-3 py-2 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700 disabled:opacity-50"
        >
          {t('financial.budgetPages.actuals.save')}
        </button>
      </div>
      {error && (
        <p className="md:col-span-4 text-sm text-red-600" role="alert">
          {error}
        </p>
      )}
    </form>
  );
}

export function BudgetDetail({
  budget,
  buildingName,
  items,
  categories,
  summary,
  currency = 'EUR',
  pendingAction,
  onAction,
  onAddItem,
  isAddingItem,
  onDeleteItem,
  onRecordActual,
  isRecordingActual,
  onBack,
}: BudgetDetailProps) {
  const { t } = useTranslation();
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [actualItemId, setActualItemId] = useState<string | null>(null);

  const isDraft = budget.status === 'draft';
  const isActive = budget.status === 'active';
  const busy = !!pendingAction;
  const categoryName = (id: string) => categories.find((c) => c.id === id)?.name ?? '—';

  const totalBudgeted = summary?.totalBudgeted ?? budget.totalAmount;
  const totalActual = summary?.totalActual ?? items.reduce((s, i) => s + i.actualAmount, 0);
  const remaining = totalBudgeted - totalActual;

  const actionButton = (action: BudgetDetailAction, primary = true) => (
    <button
      type="button"
      onClick={() => onAction(action)}
      disabled={busy}
      className={
        primary
          ? 'px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700 disabled:opacity-50'
          : 'px-4 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-md hover:bg-gray-50 disabled:opacity-50'
      }
    >
      {pendingAction === action
        ? t('financial.budgetPages.saving')
        : t(`financial.budgetPages.actions.${action}`)}
    </button>
  );

  return (
    <div className="min-h-screen bg-gray-100">
      <div className="bg-white shadow">
        <div className="max-w-5xl mx-auto px-4 sm:px-6 lg:px-8 py-6">
          <button
            type="button"
            onClick={onBack}
            className="text-sm text-blue-600 hover:text-blue-800"
          >
            &larr; {t('financial.budgetPages.backToBudgets')}
          </button>
          <div className="mt-2 flex flex-col md:flex-row md:items-start md:justify-between gap-4">
            <div>
              <div className="flex items-center gap-3">
                <h1 className="text-2xl font-bold text-gray-900">{budget.name}</h1>
                <span
                  className={`px-2 py-1 text-xs font-medium rounded ${BUDGET_STATUS_STYLES[budget.status] ?? BUDGET_STATUS_STYLES.draft}`}
                >
                  {t(`financial.budgetPages.status.${budget.status}`)}
                </span>
              </div>
              <p className="mt-1 text-sm text-gray-500">
                {t('financial.budgetPages.detail.fiscalYear', { year: budget.fiscalYear })} ·{' '}
                {buildingName ?? t('financial.budgetPages.wholeOrganization')}
              </p>
              {budget.approvedAt && (
                <p className="mt-1 text-sm text-gray-500">
                  {t('financial.budgetPages.detail.approvedOn', {
                    date: new Date(budget.approvedAt).toLocaleDateString(),
                  })}
                </p>
              )}
            </div>
            <div className="flex flex-wrap gap-2">
              {isDraft && actionButton('submit')}
              {budget.status === 'pending_approval' && actionButton('approve')}
              {budget.status === 'approved' && actionButton('activate')}
              {isActive && actionButton('close', false)}
              {isDraft &&
                (confirmDelete ? (
                  <>
                    <button
                      type="button"
                      onClick={() => onAction('delete')}
                      disabled={busy}
                      className="px-4 py-2 text-sm font-medium text-white bg-red-600 rounded-md hover:bg-red-700 disabled:opacity-50"
                    >
                      {t('financial.budgetPages.actions.confirmDelete')}
                    </button>
                    <button
                      type="button"
                      onClick={() => setConfirmDelete(false)}
                      className="px-4 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-md hover:bg-gray-50"
                    >
                      {t('financial.budgetPages.cancel')}
                    </button>
                  </>
                ) : (
                  <button
                    type="button"
                    onClick={() => setConfirmDelete(true)}
                    disabled={busy}
                    className="px-4 py-2 text-sm font-medium text-red-700 bg-white border border-red-300 rounded-md hover:bg-red-50 disabled:opacity-50"
                  >
                    {t('financial.budgetPages.actions.delete')}
                  </button>
                ))}
            </div>
          </div>
          {budget.notes && (
            <p className="mt-4 text-sm text-gray-700 whitespace-pre-line">{budget.notes}</p>
          )}
        </div>
      </div>

      <div className="max-w-5xl mx-auto px-4 sm:px-6 lg:px-8 py-8 space-y-8">
        <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
          <div className="bg-white rounded-lg shadow p-4">
            <p className="text-sm text-gray-500">{t('financial.budgetPages.budgeted')}</p>
            <p className="mt-1 text-xl font-semibold text-gray-900">
              {formatCurrency(totalBudgeted, currency)}
            </p>
          </div>
          <div className="bg-white rounded-lg shadow p-4">
            <p className="text-sm text-gray-500">{t('financial.budgetPages.actual')}</p>
            <p className="mt-1 text-xl font-semibold text-gray-900">
              {formatCurrency(totalActual, currency)}
            </p>
          </div>
          <div className="bg-white rounded-lg shadow p-4">
            <p className="text-sm text-gray-500">{t('financial.budgetPages.remaining')}</p>
            <p
              className={`mt-1 text-xl font-semibold ${remaining >= 0 ? 'text-green-600' : 'text-red-600'}`}
            >
              {formatCurrency(remaining, currency)}
            </p>
          </div>
          <div className="bg-white rounded-lg shadow p-4">
            <p className="text-sm text-gray-500">
              {t('financial.budgetPages.detail.itemsOverBudget')}
            </p>
            <p className="mt-1 text-xl font-semibold text-gray-900">
              {summary?.itemsOverBudget ?? items.filter((i) => i.varianceAmount > 0).length}
            </p>
          </div>
        </div>

        <div className="bg-white rounded-lg shadow overflow-hidden">
          <div className="px-4 py-3 border-b border-gray-200">
            <h2 className="text-lg font-semibold text-gray-900">
              {t('financial.budgetPages.items.title')}
            </h2>
          </div>
          {items.length === 0 ? (
            <p className="p-6 text-sm text-gray-500 text-center">
              {isDraft
                ? t('financial.budgetPages.items.emptyDraft')
                : t('financial.budgetPages.items.empty')}
            </p>
          ) : (
            <div className="overflow-x-auto">
              <table className="min-w-full divide-y divide-gray-200 text-sm">
                <thead className="bg-gray-50">
                  <tr>
                    <th scope="col" className="px-4 py-2 text-left font-medium text-gray-500">
                      {t('financial.budgetPages.items.category')}
                    </th>
                    <th scope="col" className="px-4 py-2 text-left font-medium text-gray-500">
                      {t('financial.budgetPages.items.name')}
                    </th>
                    <th scope="col" className="px-4 py-2 text-right font-medium text-gray-500">
                      {t('financial.budgetPages.budgeted')}
                    </th>
                    <th scope="col" className="px-4 py-2 text-right font-medium text-gray-500">
                      {t('financial.budgetPages.actual')}
                    </th>
                    <th scope="col" className="px-4 py-2 text-right font-medium text-gray-500">
                      {t('financial.budgetPages.remaining')}
                    </th>
                    {(isDraft || isActive) && (
                      <th scope="col" className="px-4 py-2">
                        <span className="sr-only">{t('financial.budgetPages.items.actions')}</span>
                      </th>
                    )}
                  </tr>
                </thead>
                <tbody className="divide-y divide-gray-200">
                  {items.map((item) => {
                    const itemRemaining = item.budgetedAmount - item.actualAmount;
                    return [
                      <tr key={item.id}>
                        <td className="px-4 py-2 text-gray-600">{categoryName(item.categoryId)}</td>
                        <td className="px-4 py-2 text-gray-900">
                          {item.name}
                          {item.description && (
                            <p className="text-xs text-gray-500">{item.description}</p>
                          )}
                        </td>
                        <td className="px-4 py-2 text-right">
                          {formatCurrency(item.budgetedAmount, currency)}
                        </td>
                        <td className="px-4 py-2 text-right">
                          {formatCurrency(item.actualAmount, currency)}
                        </td>
                        <td
                          className={`px-4 py-2 text-right ${itemRemaining >= 0 ? 'text-green-600' : 'text-red-600'}`}
                        >
                          {formatCurrency(itemRemaining, currency)}
                        </td>
                        {(isDraft || isActive) && (
                          <td className="px-4 py-2 text-right whitespace-nowrap">
                            {isDraft && (
                              <button
                                type="button"
                                onClick={() => onDeleteItem(item.id)}
                                aria-label={t('financial.budgetPages.items.removeNamed', {
                                  name: item.name,
                                })}
                                className="text-sm text-red-600 hover:text-red-800"
                              >
                                {t('financial.budgetPages.items.remove')}
                              </button>
                            )}
                            {isActive && (
                              <button
                                type="button"
                                onClick={() =>
                                  setActualItemId(actualItemId === item.id ? null : item.id)
                                }
                                aria-label={t('financial.budgetPages.actuals.recordNamed', {
                                  name: item.name,
                                })}
                                className="text-sm text-blue-600 hover:text-blue-800"
                              >
                                {t('financial.budgetPages.actuals.record')}
                              </button>
                            )}
                          </td>
                        )}
                      </tr>,
                      actualItemId === item.id && (
                        <tr key={`${item.id}-actual`} className="bg-gray-50">
                          <td colSpan={6} className="px-4 py-3">
                            <RecordActualForm
                              itemName={item.name}
                              isSubmitting={isRecordingActual}
                              onCancel={() => setActualItemId(null)}
                              onSubmit={(values) => {
                                onRecordActual(item.id, values);
                                setActualItemId(null);
                              }}
                            />
                          </td>
                        </tr>
                      ),
                    ];
                  })}
                </tbody>
              </table>
            </div>
          )}
          {isDraft && (
            <AddItemForm categories={categories} onSubmit={onAddItem} isSubmitting={isAddingItem} />
          )}
        </div>
      </div>
    </div>
  );
}
