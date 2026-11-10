/**
 * ReconciliationTable component for matching payments to invoices.
 */

import type { Invoice, Payment } from '@ppt/api-client';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { paymentMethodKey } from '../utils/constants';
import { formatCurrency, formatDayMonth } from '../utils/formatting';

interface ReconciliationTableProps {
  unallocatedPayments: Payment[];
  unpaidInvoices: Invoice[];
  onMatch: (paymentId: string, invoiceId: string, amount: number) => void;
  onAutoMatch: () => void;
  isLoading?: boolean;
}

export function ReconciliationTable({
  unallocatedPayments,
  unpaidInvoices,
  onMatch,
  onAutoMatch,
  isLoading,
}: ReconciliationTableProps) {
  const { t } = useTranslation();
  const [selectedPayment, setSelectedPayment] = useState<string | null>(null);
  const [selectedInvoice, setSelectedInvoice] = useState<string | null>(null);
  const [matchAmount, setMatchAmount] = useState('');

  const handleMatch = () => {
    if (selectedPayment && selectedInvoice && matchAmount) {
      onMatch(selectedPayment, selectedInvoice, Number.parseFloat(matchAmount));
      setSelectedPayment(null);
      setSelectedInvoice(null);
      setMatchAmount('');
    }
  };

  const selectedPaymentData = unallocatedPayments.find((p) => p.id === selectedPayment);
  const selectedInvoiceData = unpaidInvoices.find((i) => i.id === selectedInvoice);

  if (isLoading) {
    return (
      <div className="bg-white rounded-lg shadow p-6">
        <div className="animate-pulse space-y-4">
          <div className="h-8 bg-gray-200 rounded w-1/3" />
          <div className="h-64 bg-gray-200 rounded" />
        </div>
      </div>
    );
  }

  if (unallocatedPayments.length === 0 && unpaidInvoices.length === 0) {
    return (
      <div className="bg-white rounded-lg shadow p-6">
        <div className="text-center py-8">
          <div className="text-green-500 text-4xl mb-2">✓</div>
          <h3 className="text-lg font-medium text-gray-900">
            {t('financial.reconciliation.allReconciled')}
          </h3>
          <p className="text-gray-500 mt-1">{t('financial.reconciliation.allReconciledBody')}</p>
        </div>
      </div>
    );
  }

  return (
    <div className="bg-white rounded-lg shadow">
      {/* Header */}
      <div className="px-6 py-4 border-b border-gray-200 flex items-center justify-between">
        <h3 className="text-lg font-medium text-gray-900">{t('financial.reconciliation.title')}</h3>
        <button
          type="button"
          onClick={onAutoMatch}
          className="px-4 py-2 text-sm font-medium text-blue-600 bg-blue-50 rounded-md hover:bg-blue-100"
        >
          {t('financial.reconciliation.autoMatchAll')}
        </button>
      </div>

      <div className="grid grid-cols-2 divide-x">
        {/* Unallocated Payments */}
        <div className="p-4">
          <h4 className="text-sm font-medium text-gray-700 mb-3">
            {t('financial.reconciliation.unallocatedPayments', {
              count: unallocatedPayments.length,
            })}
          </h4>
          <div className="space-y-2 max-h-96 overflow-y-auto">
            {unallocatedPayments.length === 0 ? (
              <p className="text-sm text-gray-500 text-center py-4">
                {t('financial.reconciliation.noUnallocated')}
              </p>
            ) : (
              unallocatedPayments.map((payment) => (
                <button
                  key={payment.id}
                  type="button"
                  onClick={() => {
                    setSelectedPayment(payment.id);
                    if (!matchAmount && selectedInvoiceData) {
                      setMatchAmount(
                        Math.min(payment.amount, selectedInvoiceData.balance_due).toFixed(2)
                      );
                    }
                  }}
                  className={`w-full text-left p-3 rounded-lg border transition-colors ${
                    selectedPayment === payment.id
                      ? 'border-blue-500 bg-blue-50'
                      : 'border-gray-200 hover:border-gray-300'
                  }`}
                >
                  <div className="flex justify-between items-start">
                    <div>
                      <p className="text-sm font-medium text-gray-900">
                        {formatCurrency(payment.amount, payment.currency)}
                      </p>
                      <p className="text-xs text-gray-500">
                        {formatDayMonth(payment.payment_date)}
                        {payment.reference && ` • ${payment.reference}`}
                      </p>
                    </div>
                    <span className="text-xs px-2 py-0.5 bg-gray-100 text-gray-600 rounded">
                      {t(paymentMethodKey(payment.payment_method), {
                        defaultValue: payment.payment_method.replace('_', ' '),
                      })}
                    </span>
                  </div>
                </button>
              ))
            )}
          </div>
        </div>

        {/* Unpaid Invoices */}
        <div className="p-4">
          <h4 className="text-sm font-medium text-gray-700 mb-3">
            {t('financial.reconciliation.unpaidInvoices', { count: unpaidInvoices.length })}
          </h4>
          <div className="space-y-2 max-h-96 overflow-y-auto">
            {unpaidInvoices.length === 0 ? (
              <p className="text-sm text-gray-500 text-center py-4">
                {t('financial.reconciliation.noUnpaid')}
              </p>
            ) : (
              unpaidInvoices.map((invoice) => (
                <button
                  key={invoice.id}
                  type="button"
                  onClick={() => {
                    setSelectedInvoice(invoice.id);
                    if (!matchAmount && selectedPaymentData) {
                      setMatchAmount(
                        Math.min(selectedPaymentData.amount, invoice.balance_due).toFixed(2)
                      );
                    }
                  }}
                  className={`w-full text-left p-3 rounded-lg border transition-colors ${
                    selectedInvoice === invoice.id
                      ? 'border-green-500 bg-green-50'
                      : 'border-gray-200 hover:border-gray-300'
                  }`}
                >
                  <div className="flex justify-between items-start">
                    <div>
                      <p className="text-sm font-medium text-gray-900">{invoice.invoice_number}</p>
                      <p className="text-xs text-gray-500">
                        {t('financial.reconciliation.due', {
                          date: formatDayMonth(invoice.due_date),
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
                  </div>
                </button>
              ))
            )}
          </div>
        </div>
      </div>

      {/* Match Action */}
      {(selectedPayment || selectedInvoice) && (
        <div className="px-6 py-4 border-t border-gray-200 bg-gray-50">
          <div className="flex items-center gap-4">
            <div className="flex-1">
              {selectedPaymentData && (
                <span className="text-sm">
                  {t('financial.reconciliation.payment')}{' '}
                  <strong>
                    {formatCurrency(selectedPaymentData.amount, selectedPaymentData.currency)}
                  </strong>
                </span>
              )}
            </div>
            <div className="flex items-center gap-2">
              <label htmlFor="match-amount" className="text-sm text-gray-600">
                {t('financial.reconciliation.amount')}
              </label>
              <input
                id="match-amount"
                type="number"
                value={matchAmount}
                onChange={(e) => setMatchAmount(e.target.value)}
                min="0"
                step="0.01"
                className="w-32 px-3 py-1.5 border border-gray-300 rounded-md text-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500"
                placeholder="0.00"
              />
            </div>
            <div className="flex-1 text-right">
              {selectedInvoiceData && (
                <span className="text-sm">
                  {t('financial.reconciliation.invoice')}{' '}
                  <strong>{selectedInvoiceData.invoice_number}</strong>{' '}
                  {t('financial.reconciliation.amountDue', {
                    amount: formatCurrency(
                      selectedInvoiceData.balance_due,
                      selectedInvoiceData.currency
                    ),
                  })}
                </span>
              )}
            </div>
            <button
              type="button"
              onClick={handleMatch}
              disabled={!selectedPayment || !selectedInvoice || !matchAmount}
              className="px-4 py-2 text-sm font-medium text-white bg-green-600 rounded-md hover:bg-green-700 disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {t('financial.reconciliation.match')}
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
