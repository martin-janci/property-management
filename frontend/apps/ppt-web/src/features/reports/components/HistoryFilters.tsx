/**
 * HistoryFilters - Filter controls for execution history.
 *
 * Story 81.2 - Report Execution History
 */

import type { ReportExecutionStatus } from '@ppt/api-client';
import { useCallback } from 'react';
import { useTranslation } from 'react-i18next';

export interface ExecutionFilters {
  status?: ReportExecutionStatus;
  dateFrom?: string;
  dateTo?: string;
}

interface HistoryFiltersProps {
  filters: ExecutionFilters;
  onChange: (filters: ExecutionFilters) => void;
}

export function HistoryFilters({ filters, onChange }: HistoryFiltersProps) {
  const { t } = useTranslation();

  const STATUS_OPTIONS: { value: ReportExecutionStatus | ''; label: string }[] = [
    { value: '', label: t('reports.filters.allStatuses', 'All Statuses') },
    { value: 'completed', label: t('reports.filters.statusCompleted', 'Completed') },
    { value: 'failed', label: t('reports.filters.statusFailed', 'Failed') },
    { value: 'running', label: t('reports.filters.statusRunning', 'Running') },
    { value: 'pending', label: t('reports.filters.statusPending', 'Pending') },
    { value: 'cancelled', label: t('reports.filters.statusCancelled', 'Cancelled') },
    { value: 'skipped', label: t('reports.filters.statusSkipped', 'Skipped') },
  ];

  const handleStatusChange = useCallback(
    (e: React.ChangeEvent<HTMLSelectElement>) => {
      const value = e.target.value as ReportExecutionStatus | '';
      onChange({
        ...filters,
        status: value || undefined,
      });
    },
    [filters, onChange]
  );

  const handleDateFromChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      onChange({
        ...filters,
        dateFrom: e.target.value || undefined,
      });
    },
    [filters, onChange]
  );

  const handleDateToChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      onChange({
        ...filters,
        dateTo: e.target.value || undefined,
      });
    },
    [filters, onChange]
  );

  const handleClear = useCallback(() => {
    onChange({});
  }, [onChange]);

  const hasFilters = filters.status || filters.dateFrom || filters.dateTo;

  return (
    <div className="flex flex-wrap items-center gap-4">
      {/* Status Filter */}
      <div className="flex items-center gap-2">
        <label htmlFor="status-filter" className="text-sm font-medium text-gray-700">
          {t('reports.filters.status', 'Status:')}
        </label>
        <select
          id="status-filter"
          value={filters.status || ''}
          onChange={handleStatusChange}
          className="px-3 py-1.5 text-sm border border-gray-300 rounded-md focus:outline-none focus:ring-blue-500 focus:border-blue-500"
        >
          {STATUS_OPTIONS.map((option) => (
            <option key={option.value} value={option.value}>
              {option.label}
            </option>
          ))}
        </select>
      </div>

      {/* Date Range Filter */}
      <div className="flex items-center gap-2">
        <label htmlFor="date-from" className="text-sm font-medium text-gray-700">
          {t('reports.filters.dateFrom', 'From:')}
        </label>
        <input
          type="date"
          id="date-from"
          value={filters.dateFrom || ''}
          onChange={handleDateFromChange}
          max={filters.dateTo || undefined}
          className="px-3 py-1.5 text-sm border border-gray-300 rounded-md focus:outline-none focus:ring-blue-500 focus:border-blue-500"
        />
      </div>

      <div className="flex items-center gap-2">
        <label htmlFor="date-to" className="text-sm font-medium text-gray-700">
          {t('reports.filters.dateTo', 'To:')}
        </label>
        <input
          type="date"
          id="date-to"
          value={filters.dateTo || ''}
          onChange={handleDateToChange}
          min={filters.dateFrom || undefined}
          className="px-3 py-1.5 text-sm border border-gray-300 rounded-md focus:outline-none focus:ring-blue-500 focus:border-blue-500"
        />
      </div>

      {/* Clear Filters */}
      {hasFilters && (
        <button
          type="button"
          onClick={handleClear}
          className="text-sm text-gray-500 hover:text-gray-700 underline"
        >
          {t('reports.filters.clearFilters', 'Clear filters')}
        </button>
      )}
    </div>
  );
}
