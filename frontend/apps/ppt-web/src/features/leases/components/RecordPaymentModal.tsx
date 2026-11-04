/**
 * RecordPaymentModal - capture an incoming rent payment against a scheduled
 * lease payment (amount, method, reference, date).
 * Epic 19: Lease Management & Tenant Screening
 */

import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { LeasePayment, PaymentMethod } from '../types';

export interface RecordPaymentFormData {
  paidAmount: number;
  paymentMethod: PaymentMethod;
  paymentReference?: string;
  /** ISO date (YYYY-MM-DD). */
  paidAt: string;
}

interface RecordPaymentModalProps {
  payment: LeasePayment;
  isSubmitting?: boolean;
  onSubmit: (data: RecordPaymentFormData) => void;
  onClose: () => void;
}

const PAYMENT_METHODS: PaymentMethod[] = [
  'bank_transfer',
  'cash',
  'credit_card',
  'debit_card',
  'check',
  'other',
];

function todayIso(): string {
  return new Date().toISOString().split('T')[0];
}

export function RecordPaymentModal({
  payment,
  isSubmitting,
  onSubmit,
  onClose,
}: RecordPaymentModalProps) {
  const { t } = useTranslation();
  const [amount, setAmount] = useState(() => String(payment.amount));
  const [method, setMethod] = useState<PaymentMethod>('bank_transfer');
  const [reference, setReference] = useState('');
  const [paidAt, setPaidAt] = useState(todayIso);
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const value = Number(amount);
    if (!Number.isFinite(value) || value <= 0) {
      setError(t('leases.payments.recordModal.amountInvalid'));
      return;
    }
    setError(null);
    onSubmit({
      paidAmount: value,
      paymentMethod: method,
      paymentReference: reference.trim() || undefined,
      paidAt,
    });
  };

  const inputClass =
    'block w-full border border-gray-300 rounded-md shadow-sm py-2 px-3 focus:outline-none focus:ring-blue-500 focus:border-blue-500 sm:text-sm';

  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
      <div
        role="dialog"
        aria-modal="true"
        aria-labelledby="record-payment-title"
        className="bg-white rounded-lg shadow-lg p-6 max-w-md w-full mx-4"
      >
        <h2 id="record-payment-title" className="text-lg font-medium text-gray-900 mb-1">
          {t('leases.payments.recordModal.title')}
        </h2>
        <p className="text-sm text-gray-500 mb-4">
          {t('leases.payments.recordModal.dueOn', {
            date: new Date(payment.dueDate).toLocaleDateString(),
          })}
        </p>
        <form onSubmit={handleSubmit} noValidate>
          <div className="mb-4">
            <label
              htmlFor="payment-amount"
              className="block text-sm font-medium text-gray-700 mb-1"
            >
              {t('leases.payments.recordModal.amount')} ({payment.currency})
            </label>
            <input
              id="payment-amount"
              type="number"
              min="0"
              step="0.01"
              value={amount}
              onChange={(e) => setAmount(e.target.value)}
              aria-invalid={error ? true : undefined}
              aria-describedby={error ? 'payment-amount-error' : undefined}
              className={inputClass}
            />
            {error && (
              <p id="payment-amount-error" role="alert" className="mt-1 text-sm text-red-600">
                {error}
              </p>
            )}
          </div>
          <div className="mb-4">
            <label
              htmlFor="payment-method"
              className="block text-sm font-medium text-gray-700 mb-1"
            >
              {t('leases.payments.recordModal.method')}
            </label>
            <select
              id="payment-method"
              value={method}
              onChange={(e) => setMethod(e.target.value as PaymentMethod)}
              className={inputClass}
            >
              {PAYMENT_METHODS.map((m) => (
                <option key={m} value={m}>
                  {t(`leases.payments.methods.${m}`)}
                </option>
              ))}
            </select>
          </div>
          <div className="mb-4">
            <label
              htmlFor="payment-reference"
              className="block text-sm font-medium text-gray-700 mb-1"
            >
              {t('leases.payments.reference')}
            </label>
            <input
              id="payment-reference"
              type="text"
              value={reference}
              onChange={(e) => setReference(e.target.value)}
              placeholder={t('leases.payments.recordModal.referencePlaceholder')}
              className={inputClass}
            />
          </div>
          <div className="mb-6">
            <label htmlFor="payment-date" className="block text-sm font-medium text-gray-700 mb-1">
              {t('leases.payments.paidDate')}
            </label>
            <input
              id="payment-date"
              type="date"
              value={paidAt}
              max={todayIso()}
              onChange={(e) => setPaidAt(e.target.value)}
              required
              className={inputClass}
            />
          </div>
          <div className="flex justify-end gap-2">
            <button
              type="button"
              onClick={onClose}
              disabled={isSubmitting}
              className="px-4 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-md hover:bg-gray-50 disabled:opacity-50"
            >
              {t('common.cancel')}
            </button>
            <button
              type="submit"
              disabled={isSubmitting}
              className="px-4 py-2 text-sm font-medium text-white bg-green-600 hover:bg-green-700 rounded-md disabled:opacity-50"
            >
              {isSubmitting ? t('common.processing') : t('leases.payments.recordPayment')}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
