/**
 * OutagesPage - List view for outages.
 * UC-12: Utility Outages
 */

import { useState } from 'react';
import {
  type OutageCommodity,
  OutageList,
  type OutageSeverity,
  type OutageStatus,
  type OutageSummary,
} from '../components';

export interface ListOutagesParams {
  page: number;
  pageSize: number;
  status?: OutageStatus;
  commodity?: OutageCommodity;
  severity?: OutageSeverity;
}

interface OutagesPageProps {
  outages: OutageSummary[];
  total: number;
  isLoading?: boolean;
  /** Managers only; omitted for residents so the create button is hidden. */
  onNavigateToCreate?: () => void;
  onNavigateToView: (id: string) => void;
  /** Managers only; omitted for residents so the cards hide Edit. */
  onNavigateToEdit?: (id: string) => void;
  onFilterChange: (params: ListOutagesParams) => void;
}

export function OutagesPage({
  outages,
  total,
  isLoading,
  onNavigateToCreate,
  onNavigateToView,
  onNavigateToEdit,
  onFilterChange,
}: OutagesPageProps) {
  const [page, setPage] = useState(1);
  const [pageSize] = useState(10);
  const [filters, setFilters] = useState<Omit<ListOutagesParams, 'page' | 'pageSize'>>({});

  const handlePageChange = (newPage: number) => {
    setPage(newPage);
    onFilterChange({ ...filters, page: newPage, pageSize });
  };

  const handleStatusFilter = (status?: OutageStatus) => {
    const newFilters = { ...filters, status };
    setFilters(newFilters);
    setPage(1);
    onFilterChange({ ...newFilters, page: 1, pageSize });
  };

  const handleCommodityFilter = (commodity?: OutageCommodity) => {
    const newFilters = { ...filters, commodity };
    setFilters(newFilters);
    setPage(1);
    onFilterChange({ ...newFilters, page: 1, pageSize });
  };

  const handleSeverityFilter = (severity?: OutageSeverity) => {
    const newFilters = { ...filters, severity };
    setFilters(newFilters);
    setPage(1);
    onFilterChange({ ...newFilters, page: 1, pageSize });
  };

  return (
    <div className="max-w-4xl mx-auto px-4 py-8">
      <OutageList
        outages={outages}
        total={total}
        page={page}
        pageSize={pageSize}
        isLoading={isLoading}
        onPageChange={handlePageChange}
        onStatusFilter={handleStatusFilter}
        onCommodityFilter={handleCommodityFilter}
        onSeverityFilter={handleSeverityFilter}
        onView={onNavigateToView}
        onEdit={onNavigateToEdit}
        onCreate={onNavigateToCreate}
      />
    </div>
  );
}
