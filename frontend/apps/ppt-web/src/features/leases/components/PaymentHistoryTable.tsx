/**
 * PaymentHistoryTable component - displays payment history for a lease.
 * Epic 19: Lease Management & Tenant Screening
 */

import { useTranslation } from 'react-i18next';
import { getActiveLocale } from '../../../lib/locale';
import type { LeasePayment, PaymentStatus } from '../types';

interface PaymentHistoryTableProps {
  payments: LeasePayment[];
  isLoading?: boolean;
  onRecordPayment?: (paymentId: string) => void;
}

const statusColors: Record<PaymentStatus, string> = {
  pending: 'bg-yellow-100 text-yellow-800',
  paid: 'bg-green-100 text-green-800',
  overdue: 'bg-red-100 text-red-800',
  partial: 'bg-orange-100 text-orange-800',
  waived: 'bg-gray-100 text-gray-800',
};

export function PaymentHistoryTable({
  payments,
  isLoading,
  onRecordPayment,
}: PaymentHistoryTableProps) {
  const { t } = useTranslation();

  const formatDate = (dateString?: string) => {
    if (!dateString) return '-';
    return new Date(dateString).toLocaleDateString(getActiveLocale());
  };

  const formatCurrency = (amount: number, currency: string) => {
    return new Intl.NumberFormat(getActiveLocale(), {
      style: 'currency',
      currency,
    }).format(amount);
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center py-8">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600" />
      </div>
    );
  }

  if (payments.length === 0) {
    return (
      <div className="text-center py-8 bg-gray-50 rounded-lg">
        <svg
          className="mx-auto h-12 w-12 text-gray-400"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth={1.5}
            d="M17 9V7a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2m2 4h10a2 2 0 002-2v-6a2 2 0 00-2-2H9a2 2 0 00-2 2v6a2 2 0 002 2zm7-5a2 2 0 11-4 0 2 2 0 014 0z"
          />
        </svg>
        <p className="mt-4 text-gray-500">{t('leases.payments.noPayments')}</p>
      </div>
    );
  }

  return (
    <div className="overflow-x-auto">
      <table className="min-w-full divide-y divide-gray-200">
        <thead className="bg-gray-50">
          <tr>
            <th className="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
              {t('leases.payments.dueDate')}
            </th>
            <th className="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
              {t('leases.payments.amount')}
            </th>
            <th className="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
              {t('leases.payments.status')}
            </th>
            <th className="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
              {t('leases.payments.paidDate')}
            </th>
            <th className="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
              {t('leases.payments.reference')}
            </th>
            {onRecordPayment && (
              <th className="px-4 py-3 text-right text-xs font-medium text-gray-500 uppercase tracking-wider">
                {t('leases.payments.actions')}
              </th>
            )}
          </tr>
        </thead>
        <tbody className="bg-white divide-y divide-gray-200">
          {payments.map((payment) => (
            <tr key={payment.id} className="hover:bg-gray-50">
              <td className="px-4 py-3 whitespace-nowrap text-sm text-gray-900">
                {formatDate(payment.dueDate)}
              </td>
              <td className="px-4 py-3 whitespace-nowrap text-sm text-gray-900">
                <div>
                  {formatCurrency(payment.amount, payment.currency)}
                  {payment.paidAmount !== undefined &&
                    payment.paidAmount > 0 &&
                    payment.paidAmount < payment.amount && (
                      <span className="block text-xs text-gray-500">
                        {t('leases.payments.paid')}:{' '}
                        {formatCurrency(payment.paidAmount, payment.currency)}
                      </span>
                    )}
                </div>
              </td>
              <td className="px-4 py-3 whitespace-nowrap">
                <span
                  className={`px-2 py-1 text-xs font-medium rounded ${statusColors[payment.status]}`}
                >
                  {t(`leases.payments.statusLabels.${payment.status}`)}
                </span>
              </td>
              <td className="px-4 py-3 whitespace-nowrap text-sm text-gray-500">
                {formatDate(payment.paidAt)}
              </td>
              <td className="px-4 py-3 whitespace-nowrap text-sm text-gray-500">
                {payment.reference || '-'}
              </td>
              {onRecordPayment && (
                <td className="px-4 py-3 whitespace-nowrap text-right text-sm">
                  {(payment.status === 'pending' || payment.status === 'overdue') && (
                    <button
                      type="button"
                      onClick={() => onRecordPayment(payment.id)}
                      className="text-blue-600 hover:text-blue-800"
                    >
                      {t('leases.payments.recordPayment')}
                    </button>
                  )}
                </td>
              )}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
