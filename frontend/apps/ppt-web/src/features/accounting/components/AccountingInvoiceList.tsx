/**
 * AccountingInvoiceList component for displaying and managing native issued invoices.
 */

import type { AccountingInvoice, AccountingInvoiceStatus } from '@ppt/api-client';
import { format } from 'date-fns';

const INVOICE_STATUS_LABELS: Record<AccountingInvoiceStatus, string> = {
  draft: 'Draft',
  issued: 'Issued',
  paid: 'Paid',
  partially_paid: 'Partially Paid',
  overdue: 'Overdue',
};

const INVOICE_STATUS_STYLES: Record<AccountingInvoiceStatus, string> = {
  draft: 'bg-gray-100 text-gray-800',
  issued: 'bg-blue-100 text-blue-800',
  paid: 'bg-green-100 text-green-800',
  partially_paid: 'bg-yellow-100 text-yellow-800',
  overdue: 'bg-red-100 text-red-800',
};

interface AccountingInvoiceListProps {
  invoices: AccountingInvoice[];
  isLoading?: boolean;
  onViewInvoice: (id: string) => void;
  onEditInvoice?: (id: string) => void;
  onDeleteInvoice?: (id: string) => void;
}

export function AccountingInvoiceList({
  invoices,
  isLoading,
  onViewInvoice,
  onEditInvoice,
  onDeleteInvoice,
}: AccountingInvoiceListProps) {
  if (isLoading) {
    return (
      <div className="flex justify-center items-center py-12">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
      </div>
    );
  }

  if (invoices.length === 0) {
    return (
      <div className="bg-white rounded-lg shadow p-12 text-center">
        <div className="mx-auto w-12 h-12 text-gray-400 mb-4">
          <svg fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
            />
          </svg>
        </div>
        <h3 className="text-lg font-medium text-gray-900">No invoices found</h3>
        <p className="mt-1 text-sm text-gray-500">Get started by creating your first invoice.</p>
      </div>
    );
  }

  return (
    <div className="bg-white shadow overflow-hidden sm:rounded-md">
      <ul className="divide-y divide-gray-200">
        {invoices.map((invoice) => (
          <li key={invoice.id}>
            <div
              className="px-4 py-4 sm:px-6 hover:bg-gray-50 transition-colors cursor-pointer"
              onClick={() => onViewInvoice(invoice.id)}
            >
              <div className="flex items-center justify-between">
                <div className="flex flex-col">
                  <p className="text-sm font-medium text-blue-600 truncate">{invoice.number}</p>
                  <p className="text-xs text-gray-500">
                    Due: {format(new Date(invoice.dueDate), 'MMM d, yyyy')}
                  </p>
                </div>
                <div className="ml-2 flex-shrink-0 flex items-center gap-4">
                  <div className="text-right hidden sm:block">
                    <p className="text-sm font-semibold text-gray-900">
                      {invoice.totalAmount.toFixed(2)} {invoice.currency}
                    </p>
                    <p className="text-xs text-gray-500">VAT: {invoice.vatAmount.toFixed(2)}</p>
                  </div>
                  <p
                    className={`px-2 inline-flex text-xs leading-5 font-semibold rounded-full ${INVOICE_STATUS_STYLES[invoice.status]}`}
                  >
                    {INVOICE_STATUS_LABELS[invoice.status]}
                  </p>
                  <div className="flex items-center gap-2">
                    {onEditInvoice && (
                      <button
                        type="button"
                        onClick={(e) => {
                          e.stopPropagation();
                          onEditInvoice(invoice.id);
                        }}
                        className="text-gray-400 hover:text-blue-600"
                      >
                        <svg
                          className="h-5 w-5"
                          fill="none"
                          viewBox="0 0 24 24"
                          stroke="currentColor"
                        >
                          <path
                            strokeLinecap="round"
                            strokeLinejoin="round"
                            strokeWidth={2}
                            d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"
                          />
                        </svg>
                      </button>
                    )}
                    {onDeleteInvoice && (
                      <button
                        type="button"
                        onClick={(e) => {
                          e.stopPropagation();
                          onDeleteInvoice(invoice.id);
                        }}
                        className="text-gray-400 hover:text-red-600"
                      >
                        <svg
                          className="h-5 w-5"
                          fill="none"
                          viewBox="0 0 24 24"
                          stroke="currentColor"
                        >
                          <path
                            strokeLinecap="round"
                            strokeLinejoin="round"
                            strokeWidth={2}
                            d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"
                          />
                        </svg>
                      </button>
                    )}
                  </div>
                </div>
              </div>
            </div>
          </li>
        ))}
      </ul>
    </div>
  );
}
