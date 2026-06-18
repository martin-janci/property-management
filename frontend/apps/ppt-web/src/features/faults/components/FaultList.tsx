/**
 * FaultList component - displays a list of faults with filters.
 * Epic 4: Fault Reporting & Resolution (UC-03.2, UC-03.3, UC-03.4)
 */

import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { FaultCategory, FaultPriority, FaultStatus, FaultSummary } from './FaultCard';
import { FaultCard } from './FaultCard';

interface FaultListProps {
  faults: FaultSummary[];
  total: number;
  page: number;
  pageSize: number;
  isLoading?: boolean;
  /** True when the list query failed. Renders an inline error state. */
  isError?: boolean;
  /** Retry the failed list query. */
  onRetry?: () => void;
  onPageChange: (page: number) => void;
  onStatusFilter: (status?: FaultStatus) => void;
  onPriorityFilter: (priority?: FaultPriority) => void;
  onCategoryFilter: (category?: FaultCategory) => void;
  onSearch: (query: string) => void;
  onView: (id: string) => void;
  onEdit: (id: string) => void;
  onTriage: (id: string) => void;
  /** Delete a fault (only shown for 'new' status faults). Wired to useDeleteFault in route wrapper. */
  onDelete?: (id: string) => void;
  onCreate: () => void;
}

const statusOptions: { value: FaultStatus | ''; label: string }[] = [
  { value: '', label: 'All Statuses' },
  { value: 'new', label: 'New' },
  { value: 'triaged', label: 'Triaged' },
  { value: 'in_progress', label: 'In Progress' },
  { value: 'waiting_parts', label: 'Waiting for Parts' },
  { value: 'scheduled', label: 'Scheduled' },
  { value: 'resolved', label: 'Resolved' },
  { value: 'closed', label: 'Closed' },
  { value: 'reopened', label: 'Reopened' },
];

const priorityOptions: { value: FaultPriority | ''; label: string }[] = [
  { value: '', label: 'All Priorities' },
  { value: 'urgent', label: 'Urgent' },
  { value: 'high', label: 'High' },
  { value: 'medium', label: 'Medium' },
  { value: 'low', label: 'Low' },
];

const categoryOptions: { value: FaultCategory | ''; label: string }[] = [
  { value: '', label: 'All Categories' },
  { value: 'plumbing', label: 'Plumbing' },
  { value: 'electrical', label: 'Electrical' },
  { value: 'heating', label: 'Heating' },
  { value: 'structural', label: 'Structural' },
  { value: 'exterior', label: 'Exterior' },
  { value: 'elevator', label: 'Elevator' },
  { value: 'common_area', label: 'Common Area' },
  { value: 'security', label: 'Security' },
  { value: 'cleaning', label: 'Cleaning' },
  { value: 'other', label: 'Other' },
];

export function FaultList({
  faults,
  total,
  page,
  pageSize,
  isLoading,
  isError,
  onRetry,
  onPageChange,
  onStatusFilter,
  onPriorityFilter,
  onCategoryFilter,
  onSearch,
  onView,
  onEdit,
  onTriage,
  onDelete,
  onCreate,
}: FaultListProps) {
  const { t } = useTranslation();
  const [searchQuery, setSearchQuery] = useState('');
  const totalPages = Math.ceil(total / pageSize);

  const handleSearchSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    onSearch(searchQuery);
  };

  return (
    <div className="space-y-4">
      {/* Header */}
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold text-gray-900">Faults</h1>
        <button
          type="button"
          onClick={onCreate}
          className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors"
        >
          Report Fault
        </button>
      </div>

      {/* Filters */}
      <div className="flex flex-wrap gap-3">
        <form onSubmit={handleSearchSubmit} className="flex-1 min-w-[200px]">
          <input
            type="text"
            placeholder="Search faults..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
          />
        </form>
        <select
          onChange={(e) =>
            onStatusFilter(e.target.value ? (e.target.value as FaultStatus) : undefined)
          }
          className="px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
        >
          {statusOptions.map((opt) => (
            <option key={opt.value} value={opt.value}>
              {opt.label}
            </option>
          ))}
        </select>
        <select
          onChange={(e) =>
            onPriorityFilter(e.target.value ? (e.target.value as FaultPriority) : undefined)
          }
          className="px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
        >
          {priorityOptions.map((opt) => (
            <option key={opt.value} value={opt.value}>
              {opt.label}
            </option>
          ))}
        </select>
        <select
          onChange={(e) =>
            onCategoryFilter(e.target.value ? (e.target.value as FaultCategory) : undefined)
          }
          className="px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
        >
          {categoryOptions.map((opt) => (
            <option key={opt.value} value={opt.value}>
              {opt.label}
            </option>
          ))}
        </select>
      </div>

      {/* Error */}
      {!isLoading && isError && (
        <div
          role="alert"
          className="text-center py-8 border border-red-200 bg-red-50 rounded-lg text-red-700"
        >
          <p className="font-medium">
            {t('faults.failedToLoad', { defaultValue: 'Failed to load faults' })}
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

      {/* Loading */}
      {isLoading && (
        <div className="flex justify-center py-8">
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600" />
        </div>
      )}

      {/* List */}
      {!isLoading && !isError && faults.length === 0 && (
        <div className="text-center py-8 text-gray-500">No faults found.</div>
      )}

      {!isLoading && !isError && faults.length > 0 && (
        <div className="space-y-3">
          {faults.map((fault) => (
            <FaultCard
              key={fault.id}
              fault={fault}
              onView={onView}
              onEdit={onEdit}
              onTriage={onTriage}
              onDelete={onDelete}
            />
          ))}
        </div>
      )}

      {/* Pagination */}
      {totalPages > 1 && (
        <div className="flex items-center justify-between py-4 border-t">
          <p className="text-sm text-gray-500">
            Showing {(page - 1) * pageSize + 1} to {Math.min(page * pageSize, total)} of {total}{' '}
            faults
          </p>
          <div className="flex gap-2">
            <button
              type="button"
              onClick={() => onPageChange(page - 1)}
              disabled={page <= 1}
              className="px-3 py-1 border rounded disabled:opacity-50 disabled:cursor-not-allowed hover:bg-gray-50"
            >
              Previous
            </button>
            <button
              type="button"
              onClick={() => onPageChange(page + 1)}
              disabled={page >= totalPages}
              className="px-3 py-1 border rounded disabled:opacity-50 disabled:cursor-not-allowed hover:bg-gray-50"
            >
              Next
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
