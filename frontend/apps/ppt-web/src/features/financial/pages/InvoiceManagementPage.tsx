/**
 * InvoiceManagementPage - Story 52.2
 *
 * Manage invoices: list, create, view details, and send.
 */

import type { Invoice, InvoiceStatus } from '@ppt/api-client';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { BuildingFilter, InvoiceList } from '../components';

interface Building {
  id: string;
  name: string;
}

export interface InvoiceManagementPageProps {
  buildings: Building[];
  invoices: Invoice[];
  total: number;
  isLoading?: boolean;
  onNavigateToCreate: () => void;
  onNavigateToDetail: (invoiceId: string) => void;
  onSendInvoice: (invoiceId: string) => void;
  onDownloadPdf?: (invoiceId: string) => void;
  /** Id of the invoice whose PDF is currently downloading (drives the busy/disabled state). */
  downloadingPdfId?: string | null;
  onFilterChange: (params: {
    page: number;
    pageSize: number;
    buildingId?: string;
    status?: InvoiceStatus;
    search?: string;
  }) => void;
}

export function InvoiceManagementPage({
  buildings,
  invoices,
  total,
  isLoading,
  onNavigateToCreate,
  onNavigateToDetail,
  onSendInvoice,
  onDownloadPdf,
  downloadingPdfId,
  onFilterChange,
}: InvoiceManagementPageProps) {
  const { t } = useTranslation();
  const [page, setPage] = useState(1);
  const [pageSize] = useState(10);
  const [selectedBuildingId, setSelectedBuildingId] = useState<string>();
  const [statusFilter, setStatusFilter] = useState<InvoiceStatus>();
  const [searchQuery, setSearchQuery] = useState('');

  const handlePageChange = (newPage: number) => {
    setPage(newPage);
    onFilterChange({
      page: newPage,
      pageSize,
      buildingId: selectedBuildingId,
      status: statusFilter,
      search: searchQuery,
    });
  };

  const handleBuildingChange = (buildingId?: string) => {
    setSelectedBuildingId(buildingId);
    setPage(1);
    onFilterChange({
      page: 1,
      pageSize,
      buildingId,
      status: statusFilter,
      search: searchQuery,
    });
  };

  const handleStatusFilter = (status?: InvoiceStatus) => {
    setStatusFilter(status);
    setPage(1);
    onFilterChange({
      page: 1,
      pageSize,
      buildingId: selectedBuildingId,
      status,
      search: searchQuery,
    });
  };

  const handleSearch = (query: string) => {
    setSearchQuery(query);
    setPage(1);
    onFilterChange({
      page: 1,
      pageSize,
      buildingId: selectedBuildingId,
      status: statusFilter,
      search: query,
    });
  };

  return (
    <div className="min-h-screen bg-gray-100">
      {/* Header */}
      <div className="bg-white shadow">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-6">
          <div className="flex flex-col lg:flex-row lg:items-center lg:justify-between gap-4">
            <div>
              <h1 className="text-2xl font-bold text-gray-900">{t('financial.invoices.title')}</h1>
              <p className="mt-1 text-sm text-gray-500">{t('financial.invoices.subtitle')}</p>
            </div>
            <div className="flex items-center gap-4">
              <BuildingFilter
                buildings={buildings}
                selectedBuildingId={selectedBuildingId}
                onChange={handleBuildingChange}
                isLoading={isLoading}
              />
              <button
                type="button"
                onClick={onNavigateToCreate}
                className="px-4 py-2 text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 rounded-md"
              >
                {t('financial.invoices.createNew')}
              </button>
            </div>
          </div>
        </div>
      </div>

      {/* Main Content */}
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        <InvoiceList
          invoices={invoices}
          total={total}
          page={page}
          pageSize={pageSize}
          isLoading={isLoading}
          onPageChange={handlePageChange}
          onViewInvoice={onNavigateToDetail}
          onSendInvoice={onSendInvoice}
          onDownloadPdf={onDownloadPdf}
          downloadingPdfId={downloadingPdfId}
          onStatusFilter={handleStatusFilter}
          onSearch={handleSearch}
        />
      </div>
    </div>
  );
}
