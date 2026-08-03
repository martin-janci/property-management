/**
 * FaultsPage - main page listing all faults.
 * Epic 4: Fault Reporting & Resolution (UC-03.2, UC-03.3, UC-03.4)
 *
 * Wired to @ppt/api-client hooks via the FaultsPageRoute wrapper:
 *   useFaults           — list + pagination + filters
 *   useDeleteFault      — DELETE /api/v1/faults/:id (new-status faults only)
 *   useFaultStatistics  — GET /api/v1/faults/statistics (summary bar above list)
 */

import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import type {
  FaultCategory,
  FaultPriority,
  FaultStatus,
  FaultSummary,
} from '../components/FaultCard';
import { FaultList } from '../components/FaultList';

interface FaultListParams {
  status?: FaultStatus;
  priority?: FaultPriority;
  category?: FaultCategory;
  search?: string;
  page: number;
  pageSize: number;
}

/** Subset of FaultStatistics that the page summarises in the stats bar. */
export interface FaultStatsSummary {
  total_count: number;
  open_count: number;
  closed_count: number;
}

interface FaultsPageProps {
  faults: FaultSummary[];
  total: number;
  isLoading?: boolean;
  /** True when the faults list query failed (surfaces an inline error state). */
  isError?: boolean;
  /** Retry the failed faults list query. */
  onRetry?: () => void;
  /** Delete a fault (new-status only). Wired to useDeleteFault in route wrapper. */
  onDelete?: (id: string) => void;
  /** Optional statistics summary from useFaultStatistics for the summary bar. */
  stats?: FaultStatsSummary;
  /** Navigate to the fault reports/analytics page (managers only; omitted otherwise). */
  onNavigateToReports?: () => void;
  onNavigateToCreate: () => void;
  onNavigateToView: (id: string) => void;
  onNavigateToEdit: (id: string) => void;
  onNavigateToTriage: (id: string) => void;
  onFilterChange: (params: FaultListParams) => void;
}

export function FaultsPage({
  faults,
  total,
  isLoading,
  isError,
  onRetry,
  onDelete,
  stats,
  onNavigateToReports,
  onNavigateToCreate,
  onNavigateToView,
  onNavigateToEdit,
  onNavigateToTriage,
  onFilterChange,
}: FaultsPageProps) {
  const { t } = useTranslation();
  const [page, setPage] = useState(1);
  const [pageSize] = useState(10);
  const [filters, setFilters] = useState<Omit<FaultListParams, 'page' | 'pageSize'>>({});

  const handlePageChange = (newPage: number) => {
    setPage(newPage);
    onFilterChange({ ...filters, page: newPage, pageSize });
  };

  const handleStatusFilter = (status?: FaultStatus) => {
    const newFilters = { ...filters, status };
    setFilters(newFilters);
    setPage(1);
    onFilterChange({ ...newFilters, page: 1, pageSize });
  };

  const handlePriorityFilter = (priority?: FaultPriority) => {
    const newFilters = { ...filters, priority };
    setFilters(newFilters);
    setPage(1);
    onFilterChange({ ...newFilters, page: 1, pageSize });
  };

  const handleCategoryFilter = (category?: FaultCategory) => {
    const newFilters = { ...filters, category };
    setFilters(newFilters);
    setPage(1);
    onFilterChange({ ...newFilters, page: 1, pageSize });
  };

  const handleSearch = (search: string) => {
    const newFilters = { ...filters, search: search || undefined };
    setFilters(newFilters);
    setPage(1);
    onFilterChange({ ...newFilters, page: 1, pageSize });
  };

  return (
    <div className="max-w-4xl mx-auto px-4 py-8">
      {onNavigateToReports && (
        <div className="mb-4 flex justify-end">
          <button
            type="button"
            onClick={onNavigateToReports}
            className="text-sm font-medium text-blue-600 hover:text-blue-700 underline"
          >
            View reports →
          </button>
        </div>
      )}

      {/* Statistics summary bar — populated by useFaultStatistics in route wrapper */}
      {stats && (
        <div
          className="mb-6 flex gap-6 p-4 bg-gray-50 border border-gray-200 rounded-lg"
          aria-label={t('aria.faultStatistics')}
        >
          <div className="text-center">
            <p className="text-2xl font-bold text-gray-900">{stats.total_count}</p>
            <p className="text-sm text-gray-500">Total</p>
          </div>
          <div className="text-center">
            <p className="text-2xl font-bold text-blue-600">{stats.open_count}</p>
            <p className="text-sm text-gray-500">Open</p>
          </div>
          <div className="text-center">
            <p className="text-2xl font-bold text-gray-400">{stats.closed_count}</p>
            <p className="text-sm text-gray-500">Closed</p>
          </div>
        </div>
      )}

      <FaultList
        faults={faults}
        total={total}
        page={page}
        pageSize={pageSize}
        isLoading={isLoading}
        isError={isError}
        onRetry={onRetry}
        onPageChange={handlePageChange}
        onStatusFilter={handleStatusFilter}
        onPriorityFilter={handlePriorityFilter}
        onCategoryFilter={handleCategoryFilter}
        onSearch={handleSearch}
        onView={onNavigateToView}
        onEdit={onNavigateToEdit}
        onTriage={onNavigateToTriage}
        onDelete={onDelete}
        onCreate={onNavigateToCreate}
      />
    </div>
  );
}
