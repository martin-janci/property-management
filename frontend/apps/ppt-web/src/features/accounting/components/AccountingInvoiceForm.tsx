/**
 * AccountingInvoiceForm component for creating and editing native issued invoices.
 */

import type {
  AccountingContact,
  AccountingCreateInvoiceItem,
  AccountingCreateInvoiceRequest,
  AccountingInvoice,
  AccountingVatRate,
} from '@ppt/api-client';
import { type FormEvent, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';

/**
 * Parse a numeric <input> value, normalizing the empty/invalid case to 0.
 * `parseFloat('')` returns NaN, which would otherwise flow into item state and
 * produce NaN line totals that bypass the submit guard. Non-finite results
 * (NaN, ±Infinity) collapse to 0 so totals stay a real number.
 */
function parseNumericInput(value: string): number {
  const n = Number.parseFloat(value);
  return Number.isFinite(n) ? n : 0;
}

/** VAT rate options, by value + the `accounting.form.vatRate.*` i18n key. */
const VAT_RATE_OPTIONS: { i18nKey: string; value: AccountingVatRate }[] = [
  { i18nKey: 'accounting.form.vatRate.standard', value: 'standard' },
  { i18nKey: 'accounting.form.vatRate.reduced', value: 'reduced' },
  { i18nKey: 'accounting.form.vatRate.firstReduced', value: 'first_reduced' },
  { i18nKey: 'accounting.form.vatRate.zero', value: 'zero' },
];

/**
 * Resolve the effective VAT rate (%) for a line item. A named `vatRateType`
 * derives the rate from the invoice country; absent a named type the custom
 * `vatRate` the user typed is used verbatim. This is the single source of truth
 * for the rate shown in the UI AND persisted in the payload — keeping them in
 * sync so a stored invoice can never drift from what the user saw (e.g. a SK
 * standard line showing 20% must not persist the stale default 21%).
 */
function resolveVatRate(item: AccountingCreateInvoiceItem, country: 'CZ' | 'SK'): number {
  switch (item.vatRateType) {
    case 'standard':
      return country === 'CZ' ? 21 : 20;
    case 'reduced':
      return 10;
    case 'first_reduced':
      return country === 'CZ' ? 15 : 10;
    case 'zero':
      return 0;
    default:
      return item.vatRate;
  }
}

interface AccountingInvoiceFormProps {
  contacts: AccountingContact[];
  initialData?: Partial<AccountingInvoice>;
  onSubmit: (data: AccountingCreateInvoiceRequest) => void;
  onCancel: () => void;
  isSubmitting?: boolean;
}

export function AccountingInvoiceForm({
  contacts,
  initialData,
  onSubmit,
  onCancel,
  isSubmitting,
}: AccountingInvoiceFormProps) {
  const { t } = useTranslation();
  const [formError, setFormError] = useState<string | null>(null);
  const [contactId, setContactId] = useState(initialData?.contactId || '');
  const [number, setNumber] = useState(initialData?.number || '');
  const [issueDate, setIssueDate] = useState(
    initialData?.issueDate || new Date().toISOString().split('T')[0]
  );
  const [dueDate, setDueDate] = useState(initialData?.dueDate || '');
  const [currency, setCurrency] = useState(initialData?.currency || 'CZK');
  const [variableSymbol, setVariableSymbol] = useState(initialData?.variableSymbol || '');
  const [items, setItems] = useState<AccountingCreateInvoiceItem[]>([
    { description: '', qty: 1, unitPrice: 0, vatRate: 21, vatRateType: 'standard' },
  ]);
  const [country, setCountry] = useState<'CZ' | 'SK'>('CZ');

  const totals = useMemo(() => {
    let base = 0;
    let vat = 0;

    for (const item of items) {
      const b = item.qty * item.unitPrice;
      const rate = resolveVatRate(item, country);
      const v = b * (rate / 100);
      base += b;
      vat += v;
    }

    return { base, vat, total: base + vat };
  }, [items, country]);

  const handleSubmit = (e: FormEvent) => {
    e.preventDefault();

    if (!contactId || !number || !dueDate || items.length === 0) {
      setFormError(t('accounting.form.errors.requiredFields'));
      return;
    }

    // Normalize each item's vatRate to the rate the UI derived and displayed, so
    // the persisted payload matches the totals the user saw (no stale vatRate).
    const submittedItems = items
      .filter((i) => i.description.trim() !== '')
      .map((i) => ({ ...i, vatRate: resolveVatRate(i, country) }));

    // Defense in depth: never persist a line with a non-finite qty/unitPrice
    // (e.g. a cleared field left NaN in state). parseNumericInput normalizes on
    // entry, but guard the submit path too so NaN totals can never be sent.
    const hasNonFinite = submittedItems.some(
      (i) => !Number.isFinite(i.qty) || !Number.isFinite(i.unitPrice)
    );
    if (hasNonFinite) {
      setFormError(t('accounting.form.errors.invalidNumbers'));
      return;
    }

    setFormError(null);
    onSubmit({
      contactId,
      number,
      issueDate,
      dueDate,
      currency,
      variableSymbol: variableSymbol || undefined,
      items: submittedItems,
      country,
    });
  };

  const addItem = () => {
    setItems([
      ...items,
      { description: '', qty: 1, unitPrice: 0, vatRate: 21, vatRateType: 'standard' },
    ]);
  };

  const removeItem = (index: number) => {
    if (items.length > 1) {
      setItems(items.filter((_, i) => i !== index));
    }
  };

  const updateItem = (
    index: number,
    field: keyof AccountingCreateInvoiceItem,
    value: string | number | undefined
  ) => {
    const newItems = [...items];
    newItems[index] = { ...newItems[index], [field]: value };
    setItems(newItems);
  };

  return (
    <form onSubmit={handleSubmit} className="space-y-6 bg-white p-6 shadow rounded-lg">
      <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
        {/* Basic Info */}
        <div className="space-y-4">
          <div>
            <label className="block text-sm font-medium text-gray-700">
              {t('accounting.form.contact')} *
            </label>
            <select
              value={contactId}
              onChange={(e) => setContactId(e.target.value)}
              className="mt-1 block w-full border border-gray-300 rounded-md shadow-sm p-2"
              required
            >
              <option value="">{t('accounting.form.selectContact')}</option>
              {contacts.map((c) => (
                <option key={c.id} value={c.id}>
                  {c.name}
                </option>
              ))}
            </select>
          </div>
          <div>
            <label className="block text-sm font-medium text-gray-700">
              {t('accounting.form.invoiceNumber')} *
            </label>
            <input
              type="text"
              value={number}
              onChange={(e) => setNumber(e.target.value)}
              className="mt-1 block w-full border border-gray-300 rounded-md shadow-sm p-2"
              required
            />
          </div>
          <div>
            <label className="block text-sm font-medium text-gray-700">
              {t('accounting.form.variableSymbol')}
            </label>
            <input
              type="text"
              value={variableSymbol}
              onChange={(e) => setVariableSymbol(e.target.value)}
              className="mt-1 block w-full border border-gray-300 rounded-md shadow-sm p-2"
            />
          </div>
        </div>

        <div className="space-y-4">
          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="block text-sm font-medium text-gray-700">
                {t('accounting.form.issueDate')} *
              </label>
              <input
                type="date"
                value={issueDate}
                onChange={(e) => setIssueDate(e.target.value)}
                className="mt-1 block w-full border border-gray-300 rounded-md shadow-sm p-2"
                required
              />
            </div>
            <div>
              <label className="block text-sm font-medium text-gray-700">
                {t('accounting.form.dueDate')} *
              </label>
              <input
                type="date"
                value={dueDate}
                onChange={(e) => setDueDate(e.target.value)}
                className="mt-1 block w-full border border-gray-300 rounded-md shadow-sm p-2"
                required
              />
            </div>
          </div>
          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="block text-sm font-medium text-gray-700">
                {t('accounting.form.currency')} *
              </label>
              <select
                value={currency}
                onChange={(e) => setCurrency(e.target.value)}
                className="mt-1 block w-full border border-gray-300 rounded-md shadow-sm p-2"
              >
                <option value="CZK">CZK</option>
                <option value="EUR">EUR</option>
                <option value="USD">USD</option>
              </select>
            </div>
            <div>
              <label className="block text-sm font-medium text-gray-700">
                {t('accounting.form.vatCountry')} *
              </label>
              <select
                value={country}
                onChange={(e) => setCountry(e.target.value as 'CZ' | 'SK')}
                className="mt-1 block w-full border border-gray-300 rounded-md shadow-sm p-2"
              >
                <option value="CZ">{t('accounting.form.country.cz')}</option>
                <option value="SK">{t('accounting.form.country.sk')}</option>
              </select>
            </div>
          </div>
        </div>
      </div>

      {/* Items */}
      <div className="mt-8">
        <div className="flex justify-between items-center mb-4">
          <h3 className="text-lg font-medium text-gray-900">{t('accounting.form.lineItems')}</h3>
          <button
            type="button"
            onClick={addItem}
            className="text-sm bg-blue-50 text-blue-600 px-3 py-1 rounded hover:bg-blue-100"
          >
            + {t('accounting.form.addItem')}
          </button>
        </div>
        <div className="overflow-x-auto">
          <table className="min-w-full divide-y divide-gray-200">
            <thead className="bg-gray-50">
              <tr>
                <th className="px-3 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                  {t('accounting.form.col.description')}
                </th>
                <th className="px-3 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                  {t('accounting.form.col.qty')}
                </th>
                <th className="px-3 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                  {t('accounting.form.col.unitPrice')}
                </th>
                <th className="px-3 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                  {t('accounting.form.col.vatRate')}
                </th>
                <th className="px-3 py-3 text-right text-xs font-medium text-gray-500 uppercase tracking-wider">
                  {t('accounting.form.col.total')}
                </th>
                <th className="px-3 py-3 text-right text-xs font-medium text-gray-500 uppercase tracking-wider"></th>
              </tr>
            </thead>
            <tbody className="bg-white divide-y divide-gray-200">
              {items.map((item, index) => (
                <tr key={index}>
                  <td className="px-3 py-4">
                    <input
                      type="text"
                      value={item.description}
                      onChange={(e) => updateItem(index, 'description', e.target.value)}
                      className="block w-full border-gray-300 rounded-md shadow-sm p-1 text-sm"
                      placeholder={t('accounting.form.itemDescriptionPlaceholder')}
                    />
                  </td>
                  <td className="px-3 py-4 w-20">
                    <input
                      type="number"
                      value={item.qty}
                      onChange={(e) => updateItem(index, 'qty', parseNumericInput(e.target.value))}
                      className="block w-full border-gray-300 rounded-md shadow-sm p-1 text-sm"
                    />
                  </td>
                  <td className="px-3 py-4 w-32">
                    <input
                      type="number"
                      value={item.unitPrice}
                      onChange={(e) =>
                        updateItem(index, 'unitPrice', parseNumericInput(e.target.value))
                      }
                      className="block w-full border-gray-300 rounded-md shadow-sm p-1 text-sm"
                    />
                  </td>
                  <td className="px-3 py-4 w-48">
                    <select
                      value={item.vatRateType || 'custom'}
                      onChange={(e) => {
                        const val = e.target.value;
                        if (val === 'custom') {
                          updateItem(index, 'vatRateType', undefined);
                        } else {
                          updateItem(index, 'vatRateType', val as AccountingVatRate);
                        }
                      }}
                      className="block w-full border-gray-300 rounded-md shadow-sm p-1 text-sm"
                    >
                      {VAT_RATE_OPTIONS.map((opt) => (
                        <option key={opt.value} value={opt.value}>
                          {t(opt.i18nKey)}
                        </option>
                      ))}
                      <option value="custom">{t('accounting.form.customPercent')}</option>
                    </select>
                    {!item.vatRateType && (
                      <input
                        type="number"
                        value={item.vatRate}
                        onChange={(e) =>
                          updateItem(index, 'vatRate', parseNumericInput(e.target.value))
                        }
                        className="mt-1 block w-full border-gray-300 rounded-md shadow-sm p-1 text-sm"
                        placeholder="%"
                      />
                    )}
                  </td>
                  <td className="px-3 py-4 text-right text-sm font-medium">
                    {(item.qty * item.unitPrice).toFixed(2)}
                  </td>
                  <td className="px-3 py-4 text-right text-sm">
                    <button
                      type="button"
                      onClick={() => removeItem(index)}
                      className="text-red-400 hover:text-red-600"
                    >
                      {t('accounting.form.remove')}
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>

      {/* Totals */}
      <div className="flex justify-end pt-6 border-t border-gray-200">
        <div className="w-64 space-y-2">
          <div className="flex justify-between text-sm text-gray-600">
            <span>{t('accounting.form.baseAmount')}:</span>
            <span>
              {totals.base.toFixed(2)} {currency}
            </span>
          </div>
          <div className="flex justify-between text-sm text-gray-600">
            <span>{t('accounting.form.vatAmount')}:</span>
            <span>
              {totals.vat.toFixed(2)} {currency}
            </span>
          </div>
          <div className="flex justify-between text-lg font-bold text-gray-900 border-t border-gray-100 pt-2">
            <span>{t('accounting.form.total')}:</span>
            <span>
              {totals.total.toFixed(2)} {currency}
            </span>
          </div>
        </div>
      </div>

      {/* Validation error (inline, replaces the old native alert) */}
      {formError && (
        <div
          role="alert"
          className="rounded-md border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-700"
        >
          {formError}
        </div>
      )}

      {/* Actions */}
      <div className="flex justify-end gap-3 mt-8">
        <button
          type="button"
          onClick={onCancel}
          className="px-4 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-md hover:bg-gray-50"
        >
          {t('common.cancel')}
        </button>
        <button
          type="submit"
          disabled={isSubmitting}
          className="px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700 disabled:opacity-50"
        >
          {isSubmitting
            ? t('common.saving')
            : initialData
              ? t('accounting.form.updateInvoice')
              : t('accounting.createInvoice')}
        </button>
      </div>
    </form>
  );
}
