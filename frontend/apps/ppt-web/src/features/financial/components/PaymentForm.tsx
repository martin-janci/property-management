/**
 * PaymentForm component for recording payments.
 */

import type { Invoice, PaymentMethod, RecordPayment } from '@ppt/api-client';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { PAYMENT_METHODS } from '../utils/constants';
import { formatCurrency } from '../utils/formatting';

interface Unit {
  id: string;
  unit_number: string;
  building_name: string;
}

interface PaymentFormProps {
  units: Unit[];
  outstandingInvoices: Invoice[];
  initialUnitId?: string;
  onSubmit: (data: RecordPayment) => void;
  onCancel: () => void;
  isSubmitting?: boolean;
}

interface FormErrors {
  unit_id?: string;
  amount?: string;
  payment_method?: string;
}

export function PaymentForm({
  units,
  outstandingInvoices,
  initialUnitId,
  onSubmit,
  onCancel,
  isSubmitting,
}: PaymentFormProps) {
  const { t } = useTranslation();
  const [unitId, setUnitId] = useState(initialUnitId || '');
  const [amount, setAmount] = useState('');
  const [currency, setCurrency] = useState('EUR');
  const [paymentMethod, setPaymentMethod] = useState<PaymentMethod | ''>('');
  const [reference, setReference] = useState('');
  const [paymentDate, setPaymentDate] = useState(new Date().toISOString().split('T')[0]);
  const [notes, setNotes] = useState('');
  const [selectedInvoiceIds, setSelectedInvoiceIds] = useState<string[]>([]);
  const [errors, setErrors] = useState<FormErrors>({});

  const filteredInvoices = outstandingInvoices.filter((inv) => !unitId || inv.unit_id === unitId);

  const validate = (): boolean => {
    const newErrors: FormErrors = {};

    if (!unitId) {
      newErrors.unit_id = t('financial.paymentForm.unitRequired');
    }

    if (!amount || Number.parseFloat(amount) <= 0) {
      newErrors.amount = t('financial.paymentForm.amountRequired');
    }

    if (!paymentMethod) {
      newErrors.payment_method = t('financial.paymentForm.paymentMethodRequired');
    }

    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();

    if (!validate()) {
      return;
    }

    onSubmit({
      unit_id: unitId,
      amount: Number.parseFloat(amount),
      currency,
      payment_method: paymentMethod as PaymentMethod,
      reference: reference || undefined,
      payment_date: paymentDate || undefined,
      notes: notes || undefined,
      invoice_ids: selectedInvoiceIds.length > 0 ? selectedInvoiceIds : undefined,
    });
  };

  const toggleInvoice = (invoiceId: string) => {
    setSelectedInvoiceIds((prev) =>
      prev.includes(invoiceId) ? prev.filter((id) => id !== invoiceId) : [...prev, invoiceId]
    );
  };

  const selectedTotal = filteredInvoices
    .filter((inv) => selectedInvoiceIds.includes(inv.id))
    .reduce((sum, inv) => sum + inv.balance_due, 0);

  const handleAutoFillAmount = () => {
    if (selectedTotal > 0) {
      setAmount(selectedTotal.toFixed(2));
    }
  };

  return (
    <form onSubmit={handleSubmit} className="space-y-6">
      {/* Unit Selection */}
      <div>
        <label htmlFor="unit" className="block text-sm font-medium text-gray-700">
          {t('financial.paymentForm.unit')} *
        </label>
        <select
          id="unit"
          value={unitId}
          onChange={(e) => {
            setUnitId(e.target.value);
            setSelectedInvoiceIds([]);
            setErrors({ ...errors, unit_id: undefined });
          }}
          className={`mt-1 block w-full px-3 py-2 border rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 ${
            errors.unit_id ? 'border-red-300' : 'border-gray-300'
          }`}
        >
          <option value="">{t('financial.paymentForm.selectUnit')}</option>
          {units.map((unit) => (
            <option key={unit.id} value={unit.id}>
              {unit.building_name} - {unit.unit_number}
            </option>
          ))}
        </select>
        {errors.unit_id && <p className="mt-1 text-sm text-red-600">{errors.unit_id}</p>}
      </div>

      {/* Outstanding Invoices */}
      {unitId && filteredInvoices.length > 0 && (
        <div>
          <div className="flex items-center justify-between mb-2">
            <span className="block text-sm font-medium text-gray-700">
              {t('financial.paymentForm.allocateToInvoices')}
            </span>
            {selectedInvoiceIds.length > 0 && (
              <button
                type="button"
                onClick={handleAutoFillAmount}
                className="text-sm text-blue-600 hover:text-blue-800"
              >
                {t('financial.paymentForm.autoFillAmount', {
                  amount: formatCurrency(selectedTotal, currency),
                })}
              </button>
            )}
          </div>
          <div className="border rounded-lg divide-y max-h-48 overflow-y-auto">
            {filteredInvoices.map((invoice) => (
              <label
                key={invoice.id}
                className="flex items-center px-4 py-3 hover:bg-gray-50 cursor-pointer"
              >
                <input
                  type="checkbox"
                  checked={selectedInvoiceIds.includes(invoice.id)}
                  onChange={() => toggleInvoice(invoice.id)}
                  className="h-4 w-4 text-blue-600 focus:ring-blue-500 border-gray-300 rounded"
                />
                <div className="ml-3 flex-1">
                  <p className="text-sm font-medium text-gray-900">{invoice.invoice_number}</p>
                  <p className="text-sm text-gray-500">
                    {t('financial.paymentForm.due', {
                      date: new Date(invoice.due_date).toLocaleDateString(),
                    })}
                  </p>
                </div>
                <span
                  className={`text-sm font-medium ${
                    invoice.status === 'overdue' ? 'text-red-600' : 'text-gray-900'
                  }`}
                >
                  {formatCurrency(invoice.balance_due, invoice.currency)}
                </span>
              </label>
            ))}
          </div>
        </div>
      )}

      {/* Amount and Payment Method */}
      <div className="grid grid-cols-2 gap-4">
        <div>
          <label htmlFor="amount" className="block text-sm font-medium text-gray-700">
            {t('financial.paymentForm.amount')} *
          </label>
          <div className="mt-1 relative">
            <input
              type="number"
              id="amount"
              value={amount}
              onChange={(e) => {
                setAmount(e.target.value);
                setErrors({ ...errors, amount: undefined });
              }}
              min="0"
              step="0.01"
              className={`block w-full px-3 py-2 border rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 ${
                errors.amount ? 'border-red-300' : 'border-gray-300'
              }`}
              placeholder="0.00"
            />
          </div>
          {errors.amount && <p className="mt-1 text-sm text-red-600">{errors.amount}</p>}
        </div>
        <div>
          <label htmlFor="currency" className="block text-sm font-medium text-gray-700">
            {t('financial.paymentForm.currency')}
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

      {/* Payment Method */}
      <div>
        <label htmlFor="payment-method" className="block text-sm font-medium text-gray-700">
          {t('financial.paymentForm.paymentMethod')} *
        </label>
        <select
          id="payment-method"
          value={paymentMethod}
          onChange={(e) => {
            setPaymentMethod(e.target.value as PaymentMethod);
            setErrors({ ...errors, payment_method: undefined });
          }}
          className={`mt-1 block w-full px-3 py-2 border rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 ${
            errors.payment_method ? 'border-red-300' : 'border-gray-300'
          }`}
        >
          <option value="">{t('financial.paymentForm.selectPaymentMethod')}</option>
          {PAYMENT_METHODS.map((method) => (
            <option key={method.value} value={method.value}>
              {t(`financial.paymentMethods.${method.value}`)}
            </option>
          ))}
        </select>
        {errors.payment_method && (
          <p className="mt-1 text-sm text-red-600">{errors.payment_method}</p>
        )}
      </div>

      {/* Reference and Date */}
      <div className="grid grid-cols-2 gap-4">
        <div>
          <label htmlFor="reference" className="block text-sm font-medium text-gray-700">
            {t('financial.paymentForm.reference')}
          </label>
          <input
            type="text"
            id="reference"
            value={reference}
            onChange={(e) => setReference(e.target.value)}
            className="mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500"
            placeholder={t('financial.paymentForm.referencePlaceholder')}
          />
        </div>
        <div>
          <label htmlFor="payment-date" className="block text-sm font-medium text-gray-700">
            {t('financial.paymentForm.paymentDate')}
          </label>
          <input
            type="date"
            id="payment-date"
            value={paymentDate}
            onChange={(e) => setPaymentDate(e.target.value)}
            className="mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500"
          />
        </div>
      </div>

      {/* Notes */}
      <div>
        <label htmlFor="notes" className="block text-sm font-medium text-gray-700">
          {t('financial.paymentForm.notes')}
        </label>
        <textarea
          id="notes"
          value={notes}
          onChange={(e) => setNotes(e.target.value)}
          rows={2}
          className="mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500"
          placeholder={t('financial.paymentForm.notesPlaceholder')}
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
          className="px-4 py-2 text-sm font-medium text-white bg-green-600 border border-transparent rounded-md hover:bg-green-700 disabled:opacity-50"
        >
          {isSubmitting ? t('financial.paymentForm.submitting') : t('financial.paymentForm.submit')}
        </button>
      </div>
    </form>
  );
}
