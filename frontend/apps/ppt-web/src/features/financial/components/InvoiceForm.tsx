/**
 * InvoiceForm component for creating and editing invoices.
 */

import type { CreateInvoice, CreateInvoiceItem } from '@ppt/api-client';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';

interface Unit {
  id: string;
  unit_number: string;
  building_name: string;
}

interface InvoiceFormProps {
  units: Unit[];
  initialData?: Partial<CreateInvoice>;
  onSubmit: (data: CreateInvoice) => void;
  onCancel: () => void;
  isSubmitting?: boolean;
}

interface FormErrors {
  unit_id?: string;
  due_date?: string;
  items?: string;
}

export function InvoiceForm({
  units,
  initialData,
  onSubmit,
  onCancel,
  isSubmitting,
}: InvoiceFormProps) {
  const { t } = useTranslation();
  const [unitId, setUnitId] = useState(initialData?.unit_id || '');
  const [billingPeriodStart, setBillingPeriodStart] = useState(
    initialData?.billing_period_start || ''
  );
  const [billingPeriodEnd, setBillingPeriodEnd] = useState(initialData?.billing_period_end || '');
  const [dueDate, setDueDate] = useState(initialData?.due_date || '');
  const [currency, setCurrency] = useState(initialData?.currency || 'EUR');
  const [notes, setNotes] = useState(initialData?.notes || '');
  const [items, setItems] = useState<CreateInvoiceItem[]>(
    initialData?.items || [{ description: '', quantity: 1, unit_price: 0 }]
  );
  const [errors, setErrors] = useState<FormErrors>({});

  const validate = (): boolean => {
    const newErrors: FormErrors = {};

    if (!unitId) {
      newErrors.unit_id = t('financial.invoiceForm.unitRequired');
    }

    if (!dueDate) {
      newErrors.due_date = t('financial.invoiceForm.dueDateRequired');
    }

    if (items.length === 0 || items.every((item) => !item.description)) {
      newErrors.items = t('financial.invoiceForm.itemsRequired');
    }

    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();

    if (!validate()) {
      return;
    }

    const validItems = items.filter((item) => item.description.trim());

    onSubmit({
      unit_id: unitId,
      billing_period_start: billingPeriodStart || undefined,
      billing_period_end: billingPeriodEnd || undefined,
      due_date: dueDate,
      currency,
      notes: notes || undefined,
      items: validItems,
    });
  };

  const addItem = () => {
    setItems([...items, { description: '', quantity: 1, unit_price: 0 }]);
  };

  const removeItem = (index: number) => {
    if (items.length > 1) {
      setItems(items.filter((_, i) => i !== index));
    }
  };

  const updateItem = (index: number, field: keyof CreateInvoiceItem, value: string | number) => {
    const newItems = [...items];
    newItems[index] = { ...newItems[index], [field]: value };
    setItems(newItems);
  };

  const calculateSubtotal = () => {
    return items.reduce((sum, item) => {
      const quantity = item.quantity || 1;
      const price = item.unit_price || 0;
      return sum + quantity * price;
    }, 0);
  };

  const calculateTax = () => {
    return items.reduce((sum, item) => {
      const quantity = item.quantity || 1;
      const price = item.unit_price || 0;
      const taxRate = item.tax_rate || 0;
      return sum + quantity * price * (taxRate / 100);
    }, 0);
  };

  return (
    <form onSubmit={handleSubmit} className="space-y-6">
      {/* Unit Selection */}
      <div>
        <label htmlFor="unit" className="block text-sm font-medium text-gray-700">
          {t('financial.invoiceForm.unit')} *
        </label>
        <select
          id="unit"
          value={unitId}
          onChange={(e) => {
            setUnitId(e.target.value);
            setErrors({ ...errors, unit_id: undefined });
          }}
          className={`mt-1 block w-full px-3 py-2 border rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 ${
            errors.unit_id ? 'border-red-300' : 'border-gray-300'
          }`}
        >
          <option value="">{t('financial.invoiceForm.selectUnit')}</option>
          {units.map((unit) => (
            <option key={unit.id} value={unit.id}>
              {unit.building_name} - {unit.unit_number}
            </option>
          ))}
        </select>
        {errors.unit_id && <p className="mt-1 text-sm text-red-600">{errors.unit_id}</p>}
      </div>

      {/* Billing Period */}
      <div className="grid grid-cols-2 gap-4">
        <div>
          <label htmlFor="billing-period-start" className="block text-sm font-medium text-gray-700">
            {t('financial.invoiceForm.billingPeriodStart')}
          </label>
          <input
            type="date"
            id="billing-period-start"
            value={billingPeriodStart}
            onChange={(e) => setBillingPeriodStart(e.target.value)}
            className="mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500"
          />
        </div>
        <div>
          <label htmlFor="billing-period-end" className="block text-sm font-medium text-gray-700">
            {t('financial.invoiceForm.billingPeriodEnd')}
          </label>
          <input
            type="date"
            id="billing-period-end"
            value={billingPeriodEnd}
            onChange={(e) => setBillingPeriodEnd(e.target.value)}
            className="mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500"
          />
        </div>
      </div>

      {/* Due Date & Currency */}
      <div className="grid grid-cols-2 gap-4">
        <div>
          <label htmlFor="due-date" className="block text-sm font-medium text-gray-700">
            {t('financial.invoiceForm.dueDate')} *
          </label>
          <input
            type="date"
            id="due-date"
            value={dueDate}
            onChange={(e) => {
              setDueDate(e.target.value);
              setErrors({ ...errors, due_date: undefined });
            }}
            className={`mt-1 block w-full px-3 py-2 border rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 ${
              errors.due_date ? 'border-red-300' : 'border-gray-300'
            }`}
          />
          {errors.due_date && <p className="mt-1 text-sm text-red-600">{errors.due_date}</p>}
        </div>
        <div>
          <label htmlFor="currency" className="block text-sm font-medium text-gray-700">
            {t('financial.invoiceForm.currency')}
          </label>
          <select
            id="currency"
            value={currency}
            onChange={(e) => setCurrency(e.target.value)}
            className="mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500"
          >
            <option value="EUR">EUR</option>
            <option value="USD">USD</option>
            <option value="GBP">GBP</option>
            <option value="CZK">CZK</option>
          </select>
        </div>
      </div>

      {/* Line Items */}
      <div>
        <div className="flex items-center justify-between mb-2">
          <span className="block text-sm font-medium text-gray-700">
            {t('financial.invoiceForm.lineItems')} *
          </span>
          <button
            type="button"
            onClick={addItem}
            className="text-sm text-blue-600 hover:text-blue-800"
          >
            + {t('financial.invoiceForm.addItem')}
          </button>
        </div>
        {errors.items && <p className="mb-2 text-sm text-red-600">{errors.items}</p>}
        <div className="space-y-3">
          {items.map((item, index) => (
            <div key={`item-${index}`} className="grid grid-cols-12 gap-2 items-start">
              <div className="col-span-5">
                <input
                  type="text"
                  placeholder={t('financial.invoiceForm.description')}
                  aria-label={t('financial.invoiceForm.description')}
                  value={item.description}
                  onChange={(e) => updateItem(index, 'description', e.target.value)}
                  className="w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500"
                />
              </div>
              <div className="col-span-2">
                <input
                  type="number"
                  placeholder={t('financial.invoiceForm.quantity')}
                  aria-label={t('financial.invoiceForm.quantity')}
                  value={item.quantity || ''}
                  onChange={(e) =>
                    updateItem(index, 'quantity', Number.parseFloat(e.target.value) || 1)
                  }
                  min="0"
                  step="0.01"
                  className="w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500"
                />
              </div>
              <div className="col-span-2">
                <input
                  type="number"
                  placeholder={t('financial.invoiceForm.unitPrice')}
                  aria-label={t('financial.invoiceForm.unitPrice')}
                  value={item.unit_price || ''}
                  onChange={(e) =>
                    updateItem(index, 'unit_price', Number.parseFloat(e.target.value) || 0)
                  }
                  min="0"
                  step="0.01"
                  className="w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500"
                />
              </div>
              <div className="col-span-2">
                <input
                  type="number"
                  placeholder={t('financial.invoiceForm.taxRate')}
                  aria-label={t('financial.invoiceForm.taxRate')}
                  value={item.tax_rate || ''}
                  onChange={(e) =>
                    updateItem(index, 'tax_rate', Number.parseFloat(e.target.value) || 0)
                  }
                  min="0"
                  max="100"
                  step="0.1"
                  className="w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500"
                />
              </div>
              <div className="col-span-1 flex items-center justify-center">
                <button
                  type="button"
                  onClick={() => removeItem(index)}
                  disabled={items.length === 1}
                  aria-label={t('financial.invoiceForm.removeItem')}
                  className="text-red-600 hover:text-red-800 disabled:opacity-50 disabled:cursor-not-allowed"
                >
                  ✕
                </button>
              </div>
            </div>
          ))}
        </div>

        {/* Totals */}
        <div className="mt-4 border-t pt-4">
          <div className="flex justify-end space-x-8 text-sm">
            <div className="text-gray-500">
              {t('financial.invoiceForm.subtotal')}:{' '}
              <span className="font-medium text-gray-900">
                {currency} {calculateSubtotal().toFixed(2)}
              </span>
            </div>
            <div className="text-gray-500">
              {t('financial.invoiceForm.tax')}:{' '}
              <span className="font-medium text-gray-900">
                {currency} {calculateTax().toFixed(2)}
              </span>
            </div>
            <div className="text-gray-700 font-medium">
              {t('financial.invoiceForm.total')}:{' '}
              <span className="text-gray-900">
                {currency} {(calculateSubtotal() + calculateTax()).toFixed(2)}
              </span>
            </div>
          </div>
        </div>
      </div>

      {/* Notes */}
      <div>
        <label htmlFor="notes" className="block text-sm font-medium text-gray-700">
          {t('financial.invoiceForm.notes')}
        </label>
        <textarea
          id="notes"
          value={notes}
          onChange={(e) => setNotes(e.target.value)}
          rows={3}
          className="mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500"
          placeholder={t('financial.invoiceForm.notesPlaceholder')}
        />
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
          {isSubmitting ? t('financial.invoiceForm.submitting') : t('financial.invoiceForm.submit')}
        </button>
      </div>
    </form>
  );
}
