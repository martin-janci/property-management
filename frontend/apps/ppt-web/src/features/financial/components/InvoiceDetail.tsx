/**
 * InvoiceDetail component for displaying invoice details.
 */

import type { InvoiceResponse } from '@ppt/api-client';
import { useTranslation } from 'react-i18next';
import { INVOICE_STATUS_STYLES } from '../utils/constants';
import { formatCurrency, formatDetailedDate } from '../utils/formatting';

interface InvoiceDetailProps {
  invoice: InvoiceResponse;
  onSend?: () => void;
  onDownloadPdf?: () => void;
  onRecordPayment?: () => void;
  onClose: () => void;
  isSending?: boolean;
}

export function InvoiceDetail({
  invoice,
  onSend,
  onDownloadPdf,
  onRecordPayment,
  onClose,
  isSending,
}: InvoiceDetailProps) {
  const { t } = useTranslation();
  const { invoice: inv, items, payments } = invoice;

  return (
    <div className="fixed inset-0 z-50 overflow-y-auto">
      <div className="flex min-h-screen items-center justify-center p-4">
        {/* Backdrop */}
        <div
          className="fixed inset-0 bg-black bg-opacity-50 transition-opacity"
          onClick={onClose}
          onKeyDown={(e) => {
            if (e.key === 'Escape' || e.key === 'Enter' || e.key === ' ') {
              onClose();
            }
          }}
          role="button"
          tabIndex={0}
          aria-label={t('aria.closeModal')}
        />

        {/* Modal */}
        <div className="relative bg-white rounded-lg shadow-xl max-w-3xl w-full max-h-[90vh] overflow-y-auto">
          {/* Header */}
          <div className="px-6 py-4 border-b border-gray-200 flex items-center justify-between">
            <div>
              <h2 className="text-xl font-semibold text-gray-900">Invoice {inv.invoice_number}</h2>
              <span
                className={`mt-1 inline-block px-2.5 py-0.5 rounded-full text-xs font-medium ${INVOICE_STATUS_STYLES[inv.status]}`}
              >
                {inv.status.toUpperCase()}
              </span>
            </div>
            <button
              type="button"
              onClick={onClose}
              className="text-gray-400 hover:text-gray-600"
              aria-label={t('aria.closeDialog')}
            >
              <svg
                className="w-6 h-6"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
                aria-hidden="true"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M6 18L18 6M6 6l12 12"
                />
              </svg>
            </button>
          </div>

          {/* Content */}
          <div className="px-6 py-4 space-y-6">
            {/* Dates */}
            <div className="grid grid-cols-3 gap-4">
              <div>
                <p className="text-sm text-gray-500">Issue Date</p>
                <p className="font-medium">{formatDetailedDate(inv.issue_date)}</p>
              </div>
              <div>
                <p className="text-sm text-gray-500">Due Date</p>
                <p className={`font-medium ${inv.status === 'overdue' ? 'text-red-600' : ''}`}>
                  {formatDetailedDate(inv.due_date)}
                </p>
              </div>
              {inv.billing_period_start && inv.billing_period_end && (
                <div>
                  <p className="text-sm text-gray-500">Billing Period</p>
                  <p className="font-medium">
                    {formatDetailedDate(inv.billing_period_start)} -{' '}
                    {formatDetailedDate(inv.billing_period_end)}
                  </p>
                </div>
              )}
            </div>

            {/* Line Items */}
            <div>
              <h3 className="text-sm font-medium text-gray-700 mb-3">Line Items</h3>
              <table className="min-w-full divide-y divide-gray-200">
                <thead className="bg-gray-50">
                  <tr>
                    <th className="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase">
                      Description
                    </th>
                    <th className="px-4 py-2 text-right text-xs font-medium text-gray-500 uppercase">
                      Qty
                    </th>
                    <th className="px-4 py-2 text-right text-xs font-medium text-gray-500 uppercase">
                      Unit Price
                    </th>
                    <th className="px-4 py-2 text-right text-xs font-medium text-gray-500 uppercase">
                      Tax
                    </th>
                    <th className="px-4 py-2 text-right text-xs font-medium text-gray-500 uppercase">
                      Amount
                    </th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-gray-200">
                  {items.map((item) => (
                    <tr key={item.id}>
                      <td className="px-4 py-3 text-sm text-gray-900">{item.description}</td>
                      <td className="px-4 py-3 text-sm text-right text-gray-500">
                        {item.quantity}
                      </td>
                      <td className="px-4 py-3 text-sm text-right text-gray-500">
                        {formatCurrency(item.unit_price, inv.currency)}
                      </td>
                      <td className="px-4 py-3 text-sm text-right text-gray-500">
                        {item.tax_rate ? `${item.tax_rate}%` : '-'}
                      </td>
                      <td className="px-4 py-3 text-sm text-right font-medium text-gray-900">
                        {formatCurrency(item.amount, inv.currency)}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>

            {/* Totals */}
            <div className="border-t pt-4">
              <div className="flex flex-col items-end space-y-2">
                <div className="flex justify-between w-64">
                  <span className="text-gray-500">Subtotal:</span>
                  <span className="font-medium">{formatCurrency(inv.subtotal, inv.currency)}</span>
                </div>
                <div className="flex justify-between w-64">
                  <span className="text-gray-500">Tax:</span>
                  <span className="font-medium">
                    {formatCurrency(inv.tax_amount, inv.currency)}
                  </span>
                </div>
                <div className="flex justify-between w-64 text-lg">
                  <span className="font-medium">Total:</span>
                  <span className="font-bold">{formatCurrency(inv.total, inv.currency)}</span>
                </div>
                {inv.amount_paid > 0 && (
                  <div className="flex justify-between w-64 text-green-600">
                    <span>Paid:</span>
                    <span className="font-medium">
                      {formatCurrency(inv.amount_paid, inv.currency)}
                    </span>
                  </div>
                )}
                {inv.balance_due > 0 && (
                  <div className="flex justify-between w-64 text-red-600 text-lg">
                    <span className="font-medium">Balance Due:</span>
                    <span className="font-bold">
                      {formatCurrency(inv.balance_due, inv.currency)}
                    </span>
                  </div>
                )}
              </div>
            </div>

            {/* Payment History */}
            {payments.length > 0 && (
              <div>
                <h3 className="text-sm font-medium text-gray-700 mb-3">Payment History</h3>
                <ul className="divide-y divide-gray-200 border rounded-lg">
                  {payments.map((payment) => (
                    <li key={payment.id} className="px-4 py-3 flex justify-between">
                      <span className="text-sm text-gray-500">
                        {formatDetailedDate(payment.created_at)}
                      </span>
                      <span className="text-sm font-medium text-green-600">
                        {formatCurrency(payment.amount, inv.currency)}
                      </span>
                    </li>
                  ))}
                </ul>
              </div>
            )}

            {/* Notes */}
            {inv.notes && (
              <div>
                <h3 className="text-sm font-medium text-gray-700 mb-2">Notes</h3>
                <p className="text-sm text-gray-600 bg-gray-50 rounded-lg p-3">{inv.notes}</p>
              </div>
            )}
          </div>

          {/* Actions */}
          <div className="px-6 py-4 border-t border-gray-200 flex justify-end gap-3">
            {inv.status === 'draft' && onSend && (
              <button
                type="button"
                onClick={onSend}
                disabled={isSending}
                className="px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700 disabled:opacity-50"
              >
                {isSending ? 'Sending...' : 'Send Invoice'}
              </button>
            )}
            {inv.pdf_file_path && onDownloadPdf && (
              <button
                type="button"
                onClick={onDownloadPdf}
                className="px-4 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-md hover:bg-gray-50"
              >
                Download PDF
              </button>
            )}
            {inv.balance_due > 0 && onRecordPayment && (
              <button
                type="button"
                onClick={onRecordPayment}
                className="px-4 py-2 text-sm font-medium text-white bg-green-600 rounded-md hover:bg-green-700"
              >
                Record Payment
              </button>
            )}
            <button
              type="button"
              onClick={onClose}
              className="px-4 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-md hover:bg-gray-50"
            >
              Close
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
