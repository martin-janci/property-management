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

const VAT_RATE_OPTIONS: { label: string; value: AccountingVatRate }[] = [
  { label: 'Standard (21% CZ / 20% SK)', value: 'standard' },
  { label: 'Reduced (10%)', value: 'reduced' },
  { label: 'First Reduced (15% CZ)', value: 'first_reduced' },
  { label: 'Zero (0%)', value: 'zero' },
];

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
      let rate = item.vatRate;

      if (item.vatRateType) {
        if (item.vatRateType === 'standard') rate = country === 'CZ' ? 21 : 20;
        else if (item.vatRateType === 'reduced') rate = 10;
        else if (item.vatRateType === 'first_reduced') rate = country === 'CZ' ? 15 : 10;
        else if (item.vatRateType === 'zero') rate = 0;
      }

      const v = b * (rate / 100);
      base += b;
      vat += v;
    }

    return { base, vat, total: base + vat };
  }, [items, country]);

  const handleSubmit = (e: FormEvent) => {
    e.preventDefault();

    if (!contactId || !number || !dueDate || items.length === 0) {
      alert('Please fill all required fields');
      return;
    }

    onSubmit({
      contactId,
      number,
      issueDate,
      dueDate,
      currency,
      variableSymbol: variableSymbol || undefined,
      items: items.filter((i) => i.description.trim() !== ''),
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
    value: string | number
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
            <label className="block text-sm font-medium text-gray-700">Contact *</label>
            <select
              value={contactId}
              onChange={(e) => setContactId(e.target.value)}
              className="mt-1 block w-full border border-gray-300 rounded-md shadow-sm p-2"
              required
            >
              <option value="">Select a contact</option>
              {contacts.map((c) => (
                <option key={c.id} value={c.id}>
                  {c.name}
                </option>
              ))}
            </select>
          </div>
          <div>
            <label className="block text-sm font-medium text-gray-700">Invoice Number *</label>
            <input
              type="text"
              value={number}
              onChange={(e) => setNumber(e.target.value)}
              className="mt-1 block w-full border border-gray-300 rounded-md shadow-sm p-2"
              required
            />
          </div>
          <div>
            <label className="block text-sm font-medium text-gray-700">Variable Symbol</label>
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
              <label className="block text-sm font-medium text-gray-700">Issue Date *</label>
              <input
                type="date"
                value={issueDate}
                onChange={(e) => setIssueDate(e.target.value)}
                className="mt-1 block w-full border border-gray-300 rounded-md shadow-sm p-2"
                required
              />
            </div>
            <div>
              <label className="block text-sm font-medium text-gray-700">Due Date *</label>
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
              <label className="block text-sm font-medium text-gray-700">Currency *</label>
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
              <label className="block text-sm font-medium text-gray-700">VAT Country *</label>
              <select
                value={country}
                onChange={(e) => setCountry(e.target.value as 'CZ' | 'SK')}
                className="mt-1 block w-full border border-gray-300 rounded-md shadow-sm p-2"
              >
                <option value="CZ">Czech Republic</option>
                <option value="SK">Slovakia</option>
              </select>
            </div>
          </div>
        </div>
      </div>

      {/* Items */}
      <div className="mt-8">
        <div className="flex justify-between items-center mb-4">
          <h3 className="text-lg font-medium text-gray-900">Line Items</h3>
          <button
            type="button"
            onClick={addItem}
            className="text-sm bg-blue-50 text-blue-600 px-3 py-1 rounded hover:bg-blue-100"
          >
            + Add Item
          </button>
        </div>
        <div className="overflow-x-auto">
          <table className="min-w-full divide-y divide-gray-200">
            <thead className="bg-gray-50">
              <tr>
                <th className="px-3 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                  Description
                </th>
                <th className="px-3 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                  Qty
                </th>
                <th className="px-3 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                  Unit Price
                </th>
                <th className="px-3 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                  VAT Rate
                </th>
                <th className="px-3 py-3 text-right text-xs font-medium text-gray-500 uppercase tracking-wider">
                  Total
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
                      placeholder="Item description"
                    />
                  </td>
                  <td className="px-3 py-4 w-20">
                    <input
                      type="number"
                      value={item.qty}
                      onChange={(e) => updateItem(index, 'qty', parseFloat(e.target.value))}
                      className="block w-full border-gray-300 rounded-md shadow-sm p-1 text-sm"
                    />
                  </td>
                  <td className="px-3 py-4 w-32">
                    <input
                      type="number"
                      value={item.unitPrice}
                      onChange={(e) => updateItem(index, 'unitPrice', parseFloat(e.target.value))}
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
                          {opt.label}
                        </option>
                      ))}
                      <option value="custom">Custom %</option>
                    </select>
                    {!item.vatRateType && (
                      <input
                        type="number"
                        value={item.vatRate}
                        onChange={(e) => updateItem(index, 'vatRate', parseFloat(e.target.value))}
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
                      Remove
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
            <span>Base Amount:</span>
            <span>
              {totals.base.toFixed(2)} {currency}
            </span>
          </div>
          <div className="flex justify-between text-sm text-gray-600">
            <span>VAT Amount:</span>
            <span>
              {totals.vat.toFixed(2)} {currency}
            </span>
          </div>
          <div className="flex justify-between text-lg font-bold text-gray-900 border-t border-gray-100 pt-2">
            <span>Total:</span>
            <span>
              {totals.total.toFixed(2)} {currency}
            </span>
          </div>
        </div>
      </div>

      {/* Actions */}
      <div className="flex justify-end gap-3 mt-8">
        <button
          type="button"
          onClick={onCancel}
          className="px-4 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-md hover:bg-gray-50"
        >
          Cancel
        </button>
        <button
          type="submit"
          disabled={isSubmitting}
          className="px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700 disabled:opacity-50"
        >
          {isSubmitting ? 'Saving...' : initialData ? 'Update Invoice' : 'Create Invoice'}
        </button>
      </div>
    </form>
  );
}
