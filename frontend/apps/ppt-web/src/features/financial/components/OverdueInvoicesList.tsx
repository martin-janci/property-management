/**
 * OverdueInvoicesList component for displaying overdue invoices.
 */

import type { Invoice } from '@ppt/api-client';
import { useTranslation } from 'react-i18next';
import { formatCurrency, formatDate } from '../utils/formatting';

interface OverdueInvoicesListProps {
  invoices: Invoice[];
  isLoading?: boolean;
  onViewInvoice?: (invoiceId: string) => void;
  onSendReminder?: (invoiceId: string) => void;
}

function getDaysOverdue(dueDate: string): number {
  const due = new Date(dueDate);
  const today = new Date();
  const diffTime = today.getTime() - due.getTime();
  return Math.ceil(diffTime / (1000 * 60 * 60 * 24));
}

function getOverdueStyle(daysOverdue: number): string {
  if (daysOverdue > 60) return 'text-red-700 bg-red-100';
  if (daysOverdue > 30) return 'text-orange-700 bg-orange-100';
  return 'text-yellow-700 bg-yellow-100';
}

export function OverdueInvoicesList({
  invoices,
  isLoading,
  onViewInvoice,
  onSendReminder,
}: OverdueInvoicesListProps) {
  const { t } = useTranslation();
  if (isLoading) {
    return (
      <div className="bg-white rounded-lg shadow p-6">
        <h3 className="text-lg font-medium text-gray-900 mb-4">
          {t('financial.overdueInvoices.title')}
        </h3>
        <div className="animate-pulse space-y-3">
          {[1, 2, 3].map((i) => (
            <div key={i} className="h-16 bg-gray-200 rounded" />
          ))}
        </div>
      </div>
    );
  }

  if (invoices.length === 0) {
    return (
      <div className="bg-white rounded-lg shadow p-6">
        <h3 className="text-lg font-medium text-gray-900 mb-4">
          {t('financial.overdueInvoices.title')}
        </h3>
        <div className="text-center py-8">
          <div className="text-green-500 text-4xl mb-2">✓</div>
          <p className="text-gray-500">{t('financial.overdueInvoices.empty')}</p>
        </div>
      </div>
    );
  }

  return (
    <div className="bg-white rounded-lg shadow">
      <div className="px-6 py-4 border-b border-gray-200 flex items-center justify-between">
        <h3 className="text-lg font-medium text-gray-900">
          {t('financial.overdueInvoices.title')}
          <span className="ml-2 px-2 py-0.5 text-sm font-medium bg-red-100 text-red-800 rounded-full">
            {invoices.length}
          </span>
        </h3>
      </div>

      <ul className="divide-y divide-gray-200">
        {invoices.map((invoice) => {
          const daysOverdue = getDaysOverdue(invoice.due_date);
          return (
            <li key={invoice.id} className="px-6 py-4">
              <div className="flex items-center justify-between">
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2">
                    <p className="text-sm font-medium text-gray-900">{invoice.invoice_number}</p>
                    <span
                      className={`px-2 py-0.5 text-xs font-medium rounded-full ${getOverdueStyle(daysOverdue)}`}
                    >
                      {t('financial.overdueInvoices.daysOverdue', { count: daysOverdue })}
                    </span>
                  </div>
                  <p className="text-sm text-gray-500">
                    {t('financial.overdueInvoices.dueAndBalance', {
                      date: formatDate(invoice.due_date),
                      amount: formatCurrency(invoice.balance_due, invoice.currency),
                    })}
                  </p>
                </div>
                <div className="ml-4 flex items-center gap-2">
                  {onSendReminder && (
                    <button
                      type="button"
                      onClick={() => onSendReminder(invoice.id)}
                      className="text-sm text-blue-600 hover:text-blue-800"
                    >
                      {t('financial.overdueInvoices.sendReminder')}
                    </button>
                  )}
                  {onViewInvoice && (
                    <button
                      type="button"
                      onClick={() => onViewInvoice(invoice.id)}
                      className="text-sm text-gray-600 hover:text-gray-800"
                    >
                      {t('financial.view')}
                    </button>
                  )}
                </div>
              </div>
            </li>
          );
        })}
      </ul>
    </div>
  );
}
