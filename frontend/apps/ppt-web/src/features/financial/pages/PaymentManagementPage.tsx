/**
 * PaymentManagementPage - Story 52.3
 *
 * Manage payments: view, record, and reconcile.
 */

import type { Invoice, Payment } from '@ppt/api-client';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { BuildingFilter, RecentPaymentsList, ReconciliationTable } from '../components';
import { formatCurrency } from '../utils/formatting';

interface Building {
  id: string;
  name: string;
}

export interface PaymentManagementPageProps {
  buildings: Building[];
  payments: Payment[];
  total: number;
  unallocatedPayments: Payment[];
  unpaidInvoices: Invoice[];
  metrics: {
    totalReceived: number;
    pendingReconciliation: number;
    currency: string;
  };
  isLoading?: boolean;
  /** Managers only; omitted for residents so the record button is hidden. */
  onNavigateToRecord?: () => void;
  onNavigateToDetail: (paymentId: string) => void;
  onMatch: (paymentId: string, invoiceId: string, amount: number) => void;
  onAutoMatch: () => void;
  onFilterChange: (params: {
    page: number;
    pageSize: number;
    buildingId?: string;
    startDate?: string;
    endDate?: string;
  }) => void;
}

export function PaymentManagementPage({
  buildings,
  payments,
  total,
  unallocatedPayments,
  unpaidInvoices,
  metrics,
  isLoading,
  onNavigateToRecord,
  onNavigateToDetail,
  onMatch,
  onAutoMatch,
  onFilterChange,
}: PaymentManagementPageProps) {
  const { t } = useTranslation();
  const [selectedBuildingId, setSelectedBuildingId] = useState<string>();
  const [startDate, setStartDate] = useState<string>();
  const [endDate, setEndDate] = useState<string>();

  const handleBuildingChange = (buildingId?: string) => {
    setSelectedBuildingId(buildingId);
    onFilterChange({
      page: 1,
      pageSize: 10,
      buildingId,
      startDate,
      endDate,
    });
  };

  const handleDateRangeChange = (start?: string, end?: string) => {
    setStartDate(start);
    setEndDate(end);
    onFilterChange({
      page: 1,
      pageSize: 10,
      buildingId: selectedBuildingId,
      startDate: start,
      endDate: end,
    });
  };

  return (
    <div className="min-h-screen bg-gray-100">
      {/* Header */}
      <div className="bg-white shadow">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-6">
          <div className="flex flex-col lg:flex-row lg:items-center lg:justify-between gap-4">
            <div>
              <h1 className="text-2xl font-bold text-gray-900">{t('financial.payments.title')}</h1>
              <p className="mt-1 text-sm text-gray-500">{t('financial.payments.subtitle')}</p>
            </div>
            <div className="flex items-center gap-4">
              <BuildingFilter
                buildings={buildings}
                selectedBuildingId={selectedBuildingId}
                onChange={handleBuildingChange}
                isLoading={isLoading}
              />
              {onNavigateToRecord && (
                <button
                  type="button"
                  onClick={onNavigateToRecord}
                  className="px-4 py-2 text-sm font-medium text-white bg-green-600 hover:bg-green-700 rounded-md"
                >
                  {t('financial.payments.recordPayment')}
                </button>
              )}
            </div>
          </div>
        </div>
      </div>

      {/* Main Content */}
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        {/* Metrics */}
        <div className="grid grid-cols-1 md:grid-cols-2 gap-6 mb-8">
          <div className="bg-white rounded-lg shadow p-6">
            <p className="text-sm text-gray-500">{t('financial.payments.totalReceived')}</p>
            <p className="mt-1 text-3xl font-semibold text-green-600">
              {formatCurrency(metrics.totalReceived, metrics.currency)}
            </p>
          </div>
          <div className="bg-white rounded-lg shadow p-6">
            <p className="text-sm text-gray-500">{t('financial.payments.pendingReconciliation')}</p>
            <p className="mt-1 text-3xl font-semibold text-orange-600">
              {formatCurrency(metrics.pendingReconciliation, metrics.currency)}
            </p>
          </div>
        </div>

        {/* Date Range Filter */}
        <div className="bg-white rounded-lg shadow p-4 mb-6">
          <div className="flex flex-wrap gap-4 items-center">
            <div>
              <label htmlFor="start-date" className="block text-sm font-medium text-gray-700 mb-1">
                {t('financial.startDate')}
              </label>
              <input
                type="date"
                id="start-date"
                value={startDate || ''}
                onChange={(e) => handleDateRangeChange(e.target.value, endDate)}
                className="px-3 py-2 border border-gray-300 rounded-md focus:ring-blue-500 focus:border-blue-500"
              />
            </div>
            <div>
              <label htmlFor="end-date" className="block text-sm font-medium text-gray-700 mb-1">
                {t('financial.endDate')}
              </label>
              <input
                type="date"
                id="end-date"
                value={endDate || ''}
                onChange={(e) => handleDateRangeChange(startDate, e.target.value)}
                className="px-3 py-2 border border-gray-300 rounded-md focus:ring-blue-500 focus:border-blue-500"
              />
            </div>
          </div>
        </div>

        {/* Pending Reconciliation */}
        {(unallocatedPayments.length > 0 || unpaidInvoices.length > 0) && (
          <div className="mb-8">
            <h2 className="text-lg font-semibold text-gray-900 mb-4">
              {t('financial.payments.pendingReconciliationTitle')}
            </h2>
            <ReconciliationTable
              unallocatedPayments={unallocatedPayments}
              unpaidInvoices={unpaidInvoices}
              isLoading={isLoading}
              onMatch={onMatch}
              onAutoMatch={onAutoMatch}
            />
          </div>
        )}

        {/* Recent Payments */}
        <div>
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-lg font-semibold text-gray-900">
              {t('financial.payments.recentPayments')} ({total})
            </h2>
          </div>
          <RecentPaymentsList
            payments={payments}
            isLoading={isLoading}
            onViewAll={() => onNavigateToDetail(payments[0]?.id || '')}
          />
        </div>
      </div>
    </div>
  );
}
