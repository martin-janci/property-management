/**
 * BillingPage - Invoices and payment history.
 */

import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { InvoiceCard } from '../components';
import type { Invoice, InvoiceStatus } from '../types';

interface BillingPageProps {
  invoices: Invoice[];
  total: number;
  isLoading?: boolean;
  onViewInvoice?: (invoiceId: string) => void;
  onDownloadInvoice?: (invoiceId: string) => void;
  onPayInvoice?: (invoiceId: string) => void;
  onFilterChange?: (params: { page: number; pageSize: number; status?: InvoiceStatus }) => void;
  onBack?: () => void;
}

export function BillingPage({
  invoices,
  total,
  isLoading,
  onViewInvoice,
  onDownloadInvoice,
  onPayInvoice,
  onFilterChange,
  onBack,
}: BillingPageProps) {
  const { t } = useTranslation();
  const [page, setPage] = useState(1);
  const [pageSize] = useState(10);
  const [statusFilter, setStatusFilter] = useState<InvoiceStatus | undefined>();

  const handlePageChange = (newPage: number) => {
    setPage(newPage);
    onFilterChange?.({
      page: newPage,
      pageSize,
      status: statusFilter,
    });
  };

  const handleStatusFilter = (status: InvoiceStatus | undefined) => {
    setStatusFilter(status);
    setPage(1);
    onFilterChange?.({
      page: 1,
      pageSize,
      status,
    });
  };

  const totalPages = Math.ceil(total / pageSize);

  // Calculate summary stats
  const totalPaid = invoices
    .filter((i) => i.status === 'paid')
    .reduce((sum, i) => sum + i.total, 0);
  const totalOutstanding = invoices
    .filter((i) => i.status === 'open')
    .reduce((sum, i) => sum + i.amountDue, 0);
  const currency = invoices[0]?.currency || 'USD';

  function formatCurrency(amount: number, curr: string): string {
    return new Intl.NumberFormat('en-US', {
      style: 'currency',
      currency: curr,
    }).format(amount);
  }

  return (
    <div className="min-h-screen bg-gray-100">
      {/* Header */}
      <div className="bg-white shadow">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-6">
          {onBack && (
            <button
              type="button"
              onClick={onBack}
              className="mb-4 text-blue-600 hover:text-blue-800 flex items-center gap-1"
            >
              <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M15 19l-7-7 7-7"
                />
              </svg>
              {t('common.back')}
            </button>
          )}
          <div className="flex flex-col lg:flex-row lg:items-center lg:justify-between gap-4">
            <div>
              <h1 className="text-2xl font-bold text-gray-900">
                {t('subscription.billing.title')}
              </h1>
              <p className="mt-1 text-sm text-gray-500">{t('subscription.billing.subtitle')}</p>
            </div>
          </div>
        </div>
      </div>

      {/* Main Content */}
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        {/* Summary Cards */}
        <div className="grid grid-cols-1 md:grid-cols-3 gap-6 mb-8">
          <div className="bg-white rounded-lg shadow p-6">
            <p className="text-sm font-medium text-gray-500">
              {t('subscription.billing.totalInvoices')}
            </p>
            <p className="mt-2 text-3xl font-semibold text-gray-900">{total}</p>
          </div>
          <div className="bg-white rounded-lg shadow p-6">
            <p className="text-sm font-medium text-gray-500">
              {t('subscription.billing.totalPaid')}
            </p>
            <p className="mt-2 text-3xl font-semibold text-green-600">
              {formatCurrency(totalPaid, currency)}
            </p>
          </div>
          <div className="bg-white rounded-lg shadow p-6">
            <p className="text-sm font-medium text-gray-500">
              {t('subscription.billing.outstanding')}
            </p>
            <p
              className={`mt-2 text-3xl font-semibold ${totalOutstanding > 0 ? 'text-red-600' : 'text-gray-900'}`}
            >
              {formatCurrency(totalOutstanding, currency)}
            </p>
          </div>
        </div>

        {/* Filters */}
        <div className="bg-white rounded-lg shadow mb-6">
          <div className="p-4 border-b">
            <div className="flex flex-wrap items-center gap-4">
              <span className="text-sm font-medium text-gray-700">
                {t('subscription.billing.filterByStatus')}:
              </span>
              <div className="flex flex-wrap gap-2">
                <button
                  type="button"
                  onClick={() => handleStatusFilter(undefined)}
                  className={`px-3 py-1.5 text-sm rounded-full ${
                    statusFilter === undefined
                      ? 'bg-blue-100 text-blue-800'
                      : 'bg-gray-100 text-gray-600 hover:bg-gray-200'
                  }`}
                >
                  {t('common.all')}
                </button>
                {(['open', 'paid', 'void', 'uncollectible'] as InvoiceStatus[]).map((status) => (
                  <button
                    key={status}
                    type="button"
                    onClick={() => handleStatusFilter(status)}
                    className={`px-3 py-1.5 text-sm rounded-full ${
                      statusFilter === status
                        ? 'bg-blue-100 text-blue-800'
                        : 'bg-gray-100 text-gray-600 hover:bg-gray-200'
                    }`}
                  >
                    {t(`subscription.invoices.status.${status}`)}
                  </button>
                ))}
              </div>
            </div>
          </div>
        </div>

        {/* Invoice List */}
        {isLoading ? (
          <div className="space-y-4">
            {[1, 2, 3].map((i) => (
              <div key={i} className="h-24 bg-gray-200 rounded-lg animate-pulse" />
            ))}
          </div>
        ) : invoices.length === 0 ? (
          <div className="bg-white rounded-lg shadow p-8 text-center">
            <svg
              className="mx-auto h-16 w-16 text-gray-400"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={1.5}
                d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
              />
            </svg>
            <h2 className="mt-4 text-xl font-semibold text-gray-900">
              {t('subscription.invoices.noInvoices')}
            </h2>
            <p className="mt-2 text-gray-500">{t('subscription.invoices.noInvoicesDesc')}</p>
          </div>
        ) : (
          <>
            <div className="space-y-4">
              {invoices.map((invoice) => (
                <InvoiceCard
                  key={invoice.id}
                  invoice={invoice}
                  onView={onViewInvoice}
                  onDownload={onDownloadInvoice}
                  onPay={onPayInvoice}
                  compact
                />
              ))}
            </div>

            {/* Pagination */}
            {totalPages > 1 && (
              <div className="mt-6 flex items-center justify-between">
                <p className="text-sm text-gray-500">
                  {t('common.showing')} {(page - 1) * pageSize + 1} {t('common.to')}{' '}
                  {Math.min(page * pageSize, total)} {t('common.of')} {total}
                </p>
                <div className="flex items-center gap-2">
                  <button
                    type="button"
                    onClick={() => handlePageChange(page - 1)}
                    disabled={page === 1}
                    className="px-3 py-1.5 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-md hover:bg-gray-50 disabled:opacity-50 disabled:cursor-not-allowed"
                  >
                    {t('common.previous')}
                  </button>
                  <span className="text-sm text-gray-600">
                    {page} / {totalPages}
                  </span>
                  <button
                    type="button"
                    onClick={() => handlePageChange(page + 1)}
                    disabled={page === totalPages}
                    className="px-3 py-1.5 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-md hover:bg-gray-50 disabled:opacity-50 disabled:cursor-not-allowed"
                  >
                    {t('common.next')}
                  </button>
                </div>
              </div>
            )}
          </>
        )}
      </div>
    </div>
  );
}
