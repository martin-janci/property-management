/**
 * ApplicationsPage - List all tenant applications.
 * Epic 19: Lease Management & Tenant Screening
 */

import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { ApplicationCard } from '../components/ApplicationCard';
import type { ApplicationStatus, ApplicationSummary } from '../types';

export interface ApplicationListParams {
  status?: ApplicationStatus;
  unitId?: string;
  search?: string;
  page: number;
  pageSize: number;
}

interface Unit {
  id: string;
  number: string;
  buildingName: string;
}

interface ApplicationsPageProps {
  applications: ApplicationSummary[];
  total: number;
  units: Unit[];
  isLoading?: boolean;
  onNavigateToView: (id: string) => void;
  onNavigateToReview: (id: string) => void;
  onFilterChange: (params: ApplicationListParams) => void;
  /** Initial filter state (e.g. restored from the URL). */
  initialFilters?: Omit<ApplicationListParams, 'page' | 'pageSize'>;
  /** Initial 1-based page (e.g. restored from the URL). */
  initialPage?: number;
  /** Rows per page; must match the `limit` the caller fetches with. */
  pageSize?: number;
  /** Hide the free-text search when the backend cannot search (default: shown). */
  showSearch?: boolean;
}

const STATUS_OPTIONS: ApplicationStatus[] = [
  'submitted',
  'under_review',
  'screening',
  'approved',
  'rejected',
  'withdrawn',
  'draft',
];

export function ApplicationsPage({
  applications,
  total,
  units,
  isLoading,
  onNavigateToView,
  onNavigateToReview,
  onFilterChange,
  initialFilters,
  initialPage = 1,
  pageSize = 12,
  showSearch = true,
}: ApplicationsPageProps) {
  const { t } = useTranslation();
  const [page, setPage] = useState(initialPage);
  const [filters, setFilters] = useState<Omit<ApplicationListParams, 'page' | 'pageSize'>>(
    initialFilters ?? {}
  );
  const [searchInput, setSearchInput] = useState(initialFilters?.search ?? '');

  const totalPages = Math.ceil(total / pageSize);

  // Count applications by status
  const pendingCount = applications.filter(
    (a) => a.status === 'submitted' || a.status === 'under_review' || a.status === 'screening'
  ).length;

  const handlePageChange = (newPage: number) => {
    setPage(newPage);
    onFilterChange({ ...filters, page: newPage, pageSize });
  };

  const handleStatusFilter = (status?: ApplicationStatus) => {
    const newFilters = { ...filters, status };
    setFilters(newFilters);
    setPage(1);
    onFilterChange({ ...newFilters, page: 1, pageSize });
  };

  const handleUnitFilter = (unitId?: string) => {
    const newFilters = { ...filters, unitId };
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

  const hasActiveFilters = filters.status || filters.unitId || filters.search;

  return (
    <div className="max-w-7xl mx-auto px-4 py-8">
      {/* Header */}
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">{t('leases.applications.title')}</h1>
          <p className="mt-1 text-sm text-gray-500">
            {t('leases.applications.showing', { count: applications.length, total })}
            {pendingCount > 0 && (
              <span className="ml-2 text-blue-600">
                ({t('leases.applications.pendingReview', { count: pendingCount })})
              </span>
            )}
          </p>
        </div>
      </div>

      {/* Stats Cards */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-4 mb-6">
        <div className="bg-white rounded-lg shadow p-4">
          <p className="text-sm text-gray-500">{t('leases.applications.stats.total')}</p>
          <p className="text-2xl font-semibold text-gray-900">{total}</p>
        </div>
        <div className="bg-white rounded-lg shadow p-4 border-l-4 border-blue-500">
          <p className="text-sm text-gray-500">{t('leases.applications.stats.pending')}</p>
          <p className="text-2xl font-semibold text-blue-600">{pendingCount}</p>
        </div>
        <div className="bg-white rounded-lg shadow p-4">
          <p className="text-sm text-gray-500">{t('leases.applications.stats.approved')}</p>
          <p className="text-2xl font-semibold text-green-600">
            {applications.filter((a) => a.status === 'approved').length}
          </p>
        </div>
        <div className="bg-white rounded-lg shadow p-4">
          <p className="text-sm text-gray-500">{t('leases.applications.stats.rejected')}</p>
          <p className="text-2xl font-semibold text-red-600">
            {applications.filter((a) => a.status === 'rejected').length}
          </p>
        </div>
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
                  placeholder={t('leases.applications.searchPlaceholder')}
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
            aria-label={t('leases.applications.filterByStatus')}
            value={filters.status || ''}
            onChange={(e) => handleStatusFilter((e.target.value as ApplicationStatus) || undefined)}
            className="px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
          >
            <option value="">{t('leases.applications.allStatuses')}</option>
            {STATUS_OPTIONS.map((status) => (
              <option key={status} value={status}>
                {t(`leases.applications.status.${status}`)}
              </option>
            ))}
          </select>

          {/* Unit Filter */}
          <select
            aria-label={t('leases.applications.filterByUnit')}
            value={filters.unitId || ''}
            onChange={(e) => handleUnitFilter(e.target.value || undefined)}
            className="px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
          >
            <option value="">{t('leases.applications.allUnits')}</option>
            {units.map((unit) => (
              <option key={unit.id} value={unit.id}>
                {unit.buildingName} - {unit.number}
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
              {t('leases.applications.clearFilters')}
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

      {/* Empty State */}
      {!isLoading && applications.length === 0 && (
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
          <p className="mt-4 text-gray-500">{t('leases.applications.noApplications')}</p>
        </div>
      )}

      {/* Applications Grid */}
      {!isLoading && applications.length > 0 && (
        <>
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {applications.map((application) => (
              <ApplicationCard
                key={application.id}
                application={application}
                onView={onNavigateToView}
                onReview={onNavigateToReview}
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
