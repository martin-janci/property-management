/**
 * LeasesListPage - List all leases with filters.
 * Epic 19: Lease Management & Tenant Screening
 */

import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { LeaseCard } from '../components/LeaseCard';
import type { LeaseStatus, LeaseSummary } from '../types';

export interface LeaseListParams {
  status?: LeaseStatus;
  buildingId?: string;
  search?: string;
  expiringWithinDays?: number;
  page: number;
  pageSize: number;
}

interface Building {
  id: string;
  name: string;
}

interface LeasesListPageProps {
  leases: LeaseSummary[];
  total: number;
  buildings: Building[];
  isLoading?: boolean;
  /** The leases query failed — render the error state instead of the list. */
  isError?: boolean;
  /** Retry the failed leases query (refetch). */
  onRetry?: () => void;
  onNavigateToCreate: () => void;
  onNavigateToView: (id: string) => void;
  onRenewLease: (id: string) => void;
  onTerminateLease: (id: string) => void;
  onFilterChange: (params: LeaseListParams) => void;
  /** Initial filter state (e.g. restored from the URL). */
  initialFilters?: Omit<LeaseListParams, 'page' | 'pageSize'>;
  /** Initial 1-based page (e.g. restored from the URL). */
  initialPage?: number;
  /** Rows per page; must match the `limit` the caller fetches with. */
  pageSize?: number;
  /** Hide the free-text search when the backend cannot search (default: shown). */
  showSearch?: boolean;
  /** Hide the building filter when the backend cannot filter by it (default: shown). */
  showBuildingFilter?: boolean;
}

const STATUS_OPTIONS: LeaseStatus[] = [
  'active',
  'pending_signatures',
  'expiring_soon',
  'expired',
  'terminated',
  'renewed',
  'draft',
];

const EXPIRY_OPTIONS = [{ value: 30 }, { value: 60 }, { value: 90 }];

export function LeasesListPage({
  leases,
  total,
  buildings,
  isLoading,
  isError,
  onRetry,
  onNavigateToCreate,
  onNavigateToView,
  onRenewLease,
  onTerminateLease,
  onFilterChange,
  initialFilters,
  initialPage = 1,
  pageSize = 12,
  showSearch = true,
  showBuildingFilter = true,
}: LeasesListPageProps) {
  const { t } = useTranslation();
  const [page, setPage] = useState(initialPage);
  const [filters, setFilters] = useState<Omit<LeaseListParams, 'page' | 'pageSize'>>(
    initialFilters ?? {}
  );
  const [searchInput, setSearchInput] = useState(initialFilters?.search ?? '');

  const totalPages = Math.ceil(total / pageSize);

  const handlePageChange = (newPage: number) => {
    setPage(newPage);
    onFilterChange({ ...filters, page: newPage, pageSize });
  };

  const handleStatusFilter = (status?: LeaseStatus) => {
    const newFilters = { ...filters, status };
    setFilters(newFilters);
    setPage(1);
    onFilterChange({ ...newFilters, page: 1, pageSize });
  };

  const handleBuildingFilter = (buildingId?: string) => {
    const newFilters = { ...filters, buildingId };
    setFilters(newFilters);
    setPage(1);
    onFilterChange({ ...newFilters, page: 1, pageSize });
  };

  const handleExpiryFilter = (days?: number) => {
    const newFilters = { ...filters, expiringWithinDays: days };
    setFilters(newFilters);
    setPage(1);
    onFilterChange({ ...newFilters, page: 1, pageSize });
  };

  const handleSearch = (e: React.FormEvent) => {
    e.preventDefault();
    const newFilters = { ...filters, search: searchInput || undefined };
    setFilters(newFilters);
    setPage(1);
    onFilterChange({ ...newFilters, page: 1, pageSize });
  };

  const clearFilters = () => {
    setFilters({});
    setSearchInput('');
    setPage(1);
    onFilterChange({ page: 1, pageSize });
  };

  const hasActiveFilters =
    filters.status || filters.buildingId || filters.search || filters.expiringWithinDays;

  return (
    <div className="max-w-7xl mx-auto px-4 py-8">
      {/* Header */}
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">{t('leases.list.title')}</h1>
          <p className="mt-1 text-sm text-gray-500">
            {t('leases.list.showing', { count: leases.length, total })}
          </p>
        </div>
        <button
          type="button"
          onClick={onNavigateToCreate}
          className="px-4 py-2 text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 rounded-md"
        >
          {t('leases.list.createNew')}
        </button>
      </div>

      {/* Filters */}
      <div className="bg-white rounded-lg shadow p-4 mb-6">
        <div className="flex flex-wrap items-center gap-4">
          {/* Search */}
          {showSearch && (
            <form onSubmit={handleSearch} className="flex-1 min-w-[200px]">
              <div className="relative">
                <input
                  type="text"
                  value={searchInput}
                  onChange={(e) => setSearchInput(e.target.value)}
                  placeholder={t('leases.list.searchPlaceholder')}
                  className="w-full pl-10 pr-4 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                />
                <svg
                  className="absolute left-3 top-2.5 h-5 w-5 text-gray-400"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
                  />
                </svg>
              </div>
            </form>
          )}

          {/* Status Filter */}
          <select
            aria-label={t('leases.list.filterByStatus')}
            value={filters.status || ''}
            onChange={(e) => handleStatusFilter((e.target.value as LeaseStatus) || undefined)}
            className="px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
          >
            <option value="">{t('leases.list.allStatuses')}</option>
            {STATUS_OPTIONS.map((status) => (
              <option key={status} value={status}>
                {t(`leases.status.${status}`)}
              </option>
            ))}
          </select>

          {/* Building Filter */}
          {showBuildingFilter && (
            <select
              aria-label={t('leases.list.filterByBuilding')}
              value={filters.buildingId || ''}
              onChange={(e) => handleBuildingFilter(e.target.value || undefined)}
              className="px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
            >
              <option value="">{t('leases.list.allBuildings')}</option>
              {buildings.map((building) => (
                <option key={building.id} value={building.id}>
                  {building.name}
                </option>
              ))}
            </select>
          )}

          {/* Expiry Filter */}
          <select
            aria-label={t('leases.list.filterByExpiry')}
            value={filters.expiringWithinDays || ''}
            onChange={(e) =>
              handleExpiryFilter(e.target.value ? Number(e.target.value) : undefined)
            }
            className="px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
          >
            <option value="">{t('leases.list.allExpiry')}</option>
            {EXPIRY_OPTIONS.map((option) => (
              <option key={option.value} value={option.value}>
                {t('leases.list.expiringIn', { count: option.value })}
              </option>
            ))}
          </select>

          {/* Clear Filters */}
          {hasActiveFilters && (
            <button
              type="button"
              onClick={clearFilters}
              className="text-sm text-gray-600 hover:text-gray-800"
            >
              {t('leases.list.clearFilters')}
            </button>
          )}
        </div>
      </div>

      {/* Loading State */}
      {isLoading && (
        <div className="flex items-center justify-center py-12">
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600" />
        </div>
      )}

      {/* Error State */}
      {!isLoading && isError && (
        <div
          role="alert"
          className="text-center py-12 border border-red-200 bg-red-50 rounded-lg text-red-700"
        >
          <p className="font-medium">
            {t('leases.list.failedToLoad', { defaultValue: 'Failed to load leases' })}
          </p>
          {onRetry && (
            <button
              type="button"
              onClick={onRetry}
              className="mt-3 px-3 py-1 border border-red-300 rounded text-red-700 hover:bg-red-100"
            >
              {t('common.retry', { defaultValue: 'Retry' })}
            </button>
          )}
        </div>
      )}

      {/* Empty State */}
      {!isLoading && !isError && leases.length === 0 && (
        <div className="text-center py-12 bg-white rounded-lg shadow">
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
              d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
            />
          </svg>
          <p className="mt-4 text-gray-500">{t('leases.list.noLeases')}</p>
          <button
            type="button"
            onClick={onNavigateToCreate}
            className="mt-4 px-4 py-2 text-sm font-medium text-blue-600 hover:text-blue-800"
          >
            {t('leases.list.createFirst')}
          </button>
        </div>
      )}

      {/* Leases Grid */}
      {!isLoading && !isError && leases.length > 0 && (
        <>
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {leases.map((lease) => (
              <LeaseCard
                key={lease.id}
                lease={lease}
                onView={onNavigateToView}
                onRenew={onRenewLease}
                onTerminate={onTerminateLease}
              />
            ))}
          </div>

          {/* Pagination */}
          {totalPages > 1 && (
            <div className="mt-8 flex items-center justify-center gap-2">
              <button
                type="button"
                onClick={() => handlePageChange(page - 1)}
                disabled={page === 1}
                className="px-3 py-1 text-sm border border-gray-300 rounded-md hover:bg-gray-50 disabled:opacity-50 disabled:cursor-not-allowed"
              >
                {t('common.previous')}
              </button>
              <span className="text-sm text-gray-600">
                {t('common.showing')} {page} {t('common.of')} {totalPages}
              </span>
              <button
                type="button"
                onClick={() => handlePageChange(page + 1)}
                disabled={page === totalPages}
                className="px-3 py-1 text-sm border border-gray-300 rounded-md hover:bg-gray-50 disabled:opacity-50 disabled:cursor-not-allowed"
              >
                {t('common.next')}
              </button>
            </div>
          )}
        </>
      )}
    </div>
  );
}
