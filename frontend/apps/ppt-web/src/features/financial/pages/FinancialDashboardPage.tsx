/**
 * FinancialDashboardPage - Story 52.1
 *
 * Main financial dashboard showing key metrics, invoice status,
 * AR aging, recent payments, and overdue invoices.
 */

import type { ARReportEntry, ARReportTotals, Invoice, Payment } from '@ppt/api-client';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  ARAgingTable,
  BuildingFilter,
  DateRangeFilter,
  InvoiceStatusChart,
  MetricCard,
  OverdueInvoicesList,
  RecentPaymentsList,
} from '../components';
import { formatCurrency } from '../utils/formatting';

interface Building {
  id: string;
  name: string;
}

interface FinancialDashboardPageProps {
  organizationId: string;
  buildings: Building[];
  metrics: {
    totalBalance: number;
    totalOutstanding: number;
    totalOverdue: number;
    currency: string;
  };
  invoiceCounts: {
    draft: number;
    sent: number;
    overdue: number;
    paid: number;
  };
  recentPayments: Payment[];
  overdueInvoices: Invoice[];
  arReport: {
    entries: ARReportEntry[];
    totals: ARReportTotals;
  };
  isLoading?: boolean;
  onBuildingChange?: (buildingId?: string) => void;
  onDateRangeChange?: (startDate?: string, endDate?: string) => void;
  onViewInvoice?: (invoiceId: string) => void;
  onViewAllPayments?: () => void;
  onSendReminder?: (invoiceId: string) => void;
  onUnitClick?: (unitId: string) => void;
}

export function FinancialDashboardPage({
  buildings,
  metrics,
  invoiceCounts,
  recentPayments,
  overdueInvoices,
  arReport,
  isLoading,
  onBuildingChange,
  onDateRangeChange,
  onViewInvoice,
  onViewAllPayments,
  onSendReminder,
  onUnitClick,
}: FinancialDashboardPageProps) {
  const { t } = useTranslation();
  const [selectedBuildingId, setSelectedBuildingId] = useState<string>();
  const [startDate, setStartDate] = useState<string>();
  const [endDate, setEndDate] = useState<string>();

  const handleBuildingChange = (buildingId?: string) => {
    setSelectedBuildingId(buildingId);
    onBuildingChange?.(buildingId);
  };

  const handleStartDateChange = (date?: string) => {
    setStartDate(date);
    onDateRangeChange?.(date, endDate);
  };

  const handleEndDateChange = (date?: string) => {
    setEndDate(date);
    onDateRangeChange?.(startDate, date);
  };

  return (
    <div className="min-h-screen bg-gray-100">
      {/* Header */}
      <div className="bg-white shadow">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-6">
          <div className="flex flex-col lg:flex-row lg:items-center lg:justify-between gap-4">
            <h1 className="text-2xl font-bold text-gray-900">{t('financial.dashboard.title')}</h1>
            <div className="flex flex-col sm:flex-row gap-4">
              <BuildingFilter
                buildings={buildings}
                selectedBuildingId={selectedBuildingId}
                onChange={handleBuildingChange}
                isLoading={isLoading}
              />
            </div>
          </div>
          <div className="mt-4">
            <DateRangeFilter
              startDate={startDate}
              endDate={endDate}
              onStartDateChange={handleStartDateChange}
              onEndDateChange={handleEndDateChange}
            />
          </div>
        </div>
      </div>

      {/* Main Content */}
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        {/* Metrics Grid */}
        <div className="grid grid-cols-1 md:grid-cols-3 gap-6 mb-8">
          <MetricCard
            title={t('financial.dashboard.totalBalance')}
            value={formatCurrency(metrics.totalBalance, metrics.currency)}
            subtitle={t('financial.dashboard.totalBalanceSubtitle')}
            icon={
              <svg
                className="w-8 h-8"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
                aria-hidden="true"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M12 8c-1.657 0-3 .895-3 2s1.343 2 3 2 3 .895 3 2-1.343 2-3 2m0-8c1.11 0 2.08.402 2.599 1M12 8V7m0 1v8m0 0v1m0-1c-1.11 0-2.08-.402-2.599-1M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
                />
              </svg>
            }
          />
          <MetricCard
            title={t('financial.dashboard.outstanding')}
            value={formatCurrency(metrics.totalOutstanding, metrics.currency)}
            subtitle={t('financial.dashboard.outstandingSubtitle')}
            icon={
              <svg
                className="w-8 h-8"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
                aria-hidden="true"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
                />
              </svg>
            }
          />
          <MetricCard
            title={t('financial.statusOverdue')}
            value={formatCurrency(metrics.totalOverdue, metrics.currency)}
            subtitle={t('financial.dashboard.overdueSubtitle', { count: overdueInvoices.length })}
            className={metrics.totalOverdue > 0 ? 'border-l-4 border-red-500' : ''}
            icon={
              <svg
                className="w-8 h-8 text-red-400"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
                aria-hidden="true"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
                />
              </svg>
            }
          />
        </div>

        {/* Charts Row */}
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6 mb-8">
          <InvoiceStatusChart data={invoiceCounts} />
          <RecentPaymentsList
            payments={recentPayments}
            isLoading={isLoading}
            onViewAll={onViewAllPayments}
          />
        </div>

        {/* AR Aging Table */}
        <div className="mb-8">
          <ARAgingTable
            entries={arReport.entries}
            totals={arReport.totals}
            isLoading={isLoading}
            onRowClick={onUnitClick}
          />
        </div>

        {/* Overdue Invoices */}
        <div className="mb-8">
          <OverdueInvoicesList
            invoices={overdueInvoices}
            isLoading={isLoading}
            onViewInvoice={onViewInvoice}
            onSendReminder={onSendReminder}
          />
        </div>
      </div>
    </div>
  );
}
