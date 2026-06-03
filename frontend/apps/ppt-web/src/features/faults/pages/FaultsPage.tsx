/**
 * FaultsPage - main page listing all faults.
 * Epic 4: Fault Reporting & Resolution (UC-03.2, UC-03.3, UC-03.4)
 */

import { useState } from 'react';
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

interface FaultsPageProps {
  faults: FaultSummary[];
  total: number;
  isLoading?: boolean;
  /** True when the faults list query failed (surfaces an inline error state). */
  isError?: boolean;
  /** Retry the failed faults list query. */
  onRetry?: () => void;
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
  onNavigateToCreate,
  onNavigateToView,
  onNavigateToEdit,
  onNavigateToTriage,
  onFilterChange,
}: FaultsPageProps) {
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
        onCreate={onNavigateToCreate}
      />
    </div>
  );
}
