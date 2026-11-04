/**
 * InvoiceList component for displaying and managing invoices.
 */

import type { Invoice, InvoiceStatus } from '@ppt/api-client';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { INVOICE_STATUS_STYLES, INVOICE_STATUSES, invoiceStatusKey } from '../utils/constants';
import { formatCurrency, formatDate } from '../utils/formatting';

interface InvoiceListProps {
  invoices: Invoice[];
  total: number;
  page: number;
  pageSize: number;
  isLoading?: boolean;
  onPageChange: (page: number) => void;
  onViewInvoice: (invoiceId: string) => void;
  onSendInvoice?: (invoiceId: string) => void;
  onDownloadPdf?: (invoiceId: string) => void;
  /** Id of the invoice whose PDF is currently downloading; disables its PDF button. */
  downloadingPdfId?: string | null;
  onStatusFilter?: (status?: InvoiceStatus) => void;
  onSearch?: (query: string) => void;
}

export function InvoiceList({
  invoices,
  total,
  page,
  pageSize,
  isLoading,
  onPageChange,
  onViewInvoice,
  onSendInvoice,
  onDownloadPdf,
  downloadingPdfId,
  onStatusFilter,
  onSearch,
}: InvoiceListProps) {
  const { t } = useTranslation();
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedStatus, setSelectedStatus] = useState<InvoiceStatus>();

  const totalPages = Math.ceil(total / pageSize);

  const handleSearchChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    setSearchQuery(e.target.value);
    onSearch?.(e.target.value);
  };

  const handleStatusChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
    const status = e.target.value as InvoiceStatus | '';
    setSelectedStatus(status || undefined);
    onStatusFilter?.(status || undefined);
  };

  if (isLoading) {
    return (
      <div className="bg-white rounded-lg shadow">
        <div className="p-6">
          <div className="animate-pulse space-y-4">
            <div className="h-10 bg-gray-200 rounded w-full" />
            {[1, 2, 3, 4, 5].map((i) => (
              <div key={i} className="h-16 bg-gray-200 rounded" />
            ))}
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="bg-white rounded-lg shadow">
      {/* Filters */}
      <div className="px-6 py-4 border-b border-gray-200">
        <div className="flex flex-col sm:flex-row gap-4">
          <div className="flex-1">
            <input
              type="text"
              placeholder={t('financial.invoices.searchPlaceholder')}
              aria-label={t('financial.invoices.searchPlaceholder')}
              value={searchQuery}
              onChange={handleSearchChange}
              className="w-full px-4 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
            />
          </div>
          <div className="w-48">
            <select
              value={selectedStatus || ''}
              onChange={handleStatusChange}
              aria-label={t('financial.invoices.columns.status')}
              className="w-full px-4 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
            >
              <option value="">{t('financial.invoices.allStatuses')}</option>
              {INVOICE_STATUSES.map((value) => (
                <option key={value} value={value}>
                  {t(invoiceStatusKey(value))}
                </option>
              ))}
            </select>
          </div>
        </div>
      </div>

      {/* Table */}
      {invoices.length === 0 ? (
        <div className="p-8 text-center">
          <p className="text-gray-500">{t('financial.invoices.empty')}</p>
        </div>
      ) : (
        <div className="overflow-x-auto">
          <table className="min-w-full divide-y divide-gray-200">
            <thead className="bg-gray-50">
              <tr>
                <th
                  scope="col"
                  className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider"
                >
                  {t('financial.invoices.columns.number')}
                </th>
                <th
                  scope="col"
                  className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider"
                >
                  {t('financial.invoices.columns.issueDate')}
                </th>
                <th
                  scope="col"
                  className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider"
                >
                  {t('financial.invoices.columns.dueDate')}
                </th>
                <th
                  scope="col"
                  className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider"
                >
                  {t('financial.invoices.columns.status')}
                </th>
                <th
                  scope="col"
                  className="px-6 py-3 text-right text-xs font-medium text-gray-500 uppercase tracking-wider"
                >
                  {t('financial.invoices.columns.total')}
                </th>
                <th
                  scope="col"
                  className="px-6 py-3 text-right text-xs font-medium text-gray-500 uppercase tracking-wider"
                >
                  {t('financial.invoices.columns.balanceDue')}
                </th>
                <th
                  scope="col"
                  className="px-6 py-3 text-right text-xs font-medium text-gray-500 uppercase tracking-wider"
                >
                  {t('common.actions')}
                </th>
              </tr>
            </thead>
            <tbody className="bg-white divide-y divide-gray-200">
              {invoices.map((invoice) => (
                <tr
                  key={invoice.id}
                  className="hover:bg-gray-50 cursor-pointer"
                  onClick={() => onViewInvoice(invoice.id)}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter' || e.key === ' ') {
                      e.preventDefault();
                      onViewInvoice(invoice.id);
                    }
                  }}
                  tabIndex={0}
                  aria-label={t('financial.invoices.viewAria', { number: invoice.invoice_number })}
                >
                  <td className="px-6 py-4 whitespace-nowrap text-sm font-medium text-gray-900">
                    {invoice.invoice_number}
                  </td>
                  <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-500">
                    {formatDate(invoice.issue_date)}
                  </td>
                  <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-500">
                    {formatDate(invoice.due_date)}
                  </td>
                  <td className="px-6 py-4 whitespace-nowrap">
                    <span
                      className={`px-2.5 py-0.5 rounded-full text-xs font-medium ${INVOICE_STATUS_STYLES[invoice.status]}`}
                    >
                      {t(invoiceStatusKey(invoice.status), { defaultValue: invoice.status })}
                    </span>
                  </td>
                  <td className="px-6 py-4 whitespace-nowrap text-sm text-right text-gray-900">
                    {formatCurrency(invoice.total, invoice.currency)}
                  </td>
                  <td
                    className={`px-6 py-4 whitespace-nowrap text-sm text-right font-medium ${
                      invoice.balance_due > 0 ? 'text-red-600' : 'text-green-600'
                    }`}
                  >
                    {formatCurrency(invoice.balance_due, invoice.currency)}
                  </td>
                  <td className="px-6 py-4 whitespace-nowrap text-sm text-right">
                    <div className="flex justify-end gap-2">
                      {invoice.status === 'draft' && onSendInvoice && (
                        <button
                          type="button"
                          onClick={(e) => {
                            e.stopPropagation();
                            onSendInvoice(invoice.id);
                          }}
                          className="text-blue-600 hover:text-blue-800"
                        >
                          {t('financial.invoices.send')}
                        </button>
                      )}
                      {onDownloadPdf &&
                        (() => {
                          const isDownloading = downloadingPdfId === invoice.id;
                          return (
                            <button
                              type="button"
                              onClick={(e) => {
                                e.stopPropagation();
                                onDownloadPdf(invoice.id);
                              }}
                              disabled={isDownloading}
                              aria-busy={isDownloading}
                              className="text-gray-600 hover:text-gray-800 disabled:opacity-50 disabled:cursor-not-allowed"
                            >
                              {isDownloading
                                ? t('financial.invoices.pdfDownloading')
                                : t('financial.invoices.pdf')}
                            </button>
                          );
                        })()}
                      <button
                        type="button"
                        onClick={(e) => {
                          e.stopPropagation();
                          onViewInvoice(invoice.id);
                        }}
                        className="text-gray-600 hover:text-gray-800"
                      >
                        {t('financial.view')}
                      </button>
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {/* Pagination */}
      {totalPages > 1 && (
        <div className="px-6 py-4 border-t border-gray-200 flex items-center justify-between">
          <div className="text-sm text-gray-500">
            {t('financial.invoices.pagination', {
              from: (page - 1) * pageSize + 1,
              to: Math.min(page * pageSize, total),
              total,
            })}
          </div>
          <div className="flex gap-2">
            <button
              type="button"
              onClick={() => onPageChange(page - 1)}
              disabled={page === 1}
              className="px-3 py-1 border border-gray-300 rounded-md text-sm disabled:opacity-50 disabled:cursor-not-allowed hover:bg-gray-50"
            >
              {t('common.previous')}
            </button>
            <button
              type="button"
              onClick={() => onPageChange(page + 1)}
              disabled={page === totalPages}
              className="px-3 py-1 border border-gray-300 rounded-md text-sm disabled:opacity-50 disabled:cursor-not-allowed hover:bg-gray-50"
            >
              {t('common.next')}
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
