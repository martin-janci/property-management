/**
 * OutageList component displays a filterable list of outages.
 * UC-12: Utility Outages
 */

import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  OutageCard,
  type OutageCommodity,
  type OutageSeverity,
  type OutageStatus,
  type OutageSummary,
} from './OutageCard';

interface OutageListProps {
  outages: OutageSummary[];
  total: number;
  page: number;
  pageSize: number;
  isLoading?: boolean;
  /** True when the outages fetch failed. Renders a distinct error state
   * (with retry) instead of the empty "No outages found" placeholder, so a
   * genuine fetch failure is not hidden as an empty result. */
  isError?: boolean;
  onPageChange: (page: number) => void;
  /** Re-run the outages fetch (wired to the error state's retry button). */
  onRetry?: () => void;
  onStatusFilter: (status?: OutageStatus) => void;
  onCommodityFilter: (commodity?: OutageCommodity) => void;
  onSeverityFilter: (severity?: OutageSeverity) => void;
  onView: (id: string) => void;
  onEdit?: (id: string) => void;
  /** Omitted for residents: the create button is hidden. */
  onCreate?: () => void;
}

const statusOptions: OutageStatus[] = ['planned', 'ongoing', 'resolved', 'cancelled'];
const commodityOptions: OutageCommodity[] = [
  'electricity',
  'gas',
  'water',
  'heating',
  'internet',
  'other',
];
const severityOptions: OutageSeverity[] = ['informational', 'minor', 'major', 'critical'];

export function OutageList({
  outages,
  total,
  page,
  pageSize,
  isLoading,
  isError,
  onPageChange,
  onRetry,
  onStatusFilter,
  onCommodityFilter,
  onSeverityFilter,
  onView,
  onEdit,
  onCreate,
}: OutageListProps) {
  const { t } = useTranslation();
  const [statusFilter, setStatusFilter] = useState<OutageStatus | ''>('');
  const [commodityFilter, setCommodityFilter] = useState<OutageCommodity | ''>('');
  const [severityFilter, setSeverityFilter] = useState<OutageSeverity | ''>('');

  const totalPages = Math.ceil(total / pageSize);

  const handleStatusChange = (value: string) => {
    setStatusFilter(value as OutageStatus | '');
    onStatusFilter(value ? (value as OutageStatus) : undefined);
  };

  const handleCommodityChange = (value: string) => {
    setCommodityFilter(value as OutageCommodity | '');
    onCommodityFilter(value ? (value as OutageCommodity) : undefined);
  };

  const handleSeverityChange = (value: string) => {
    setSeverityFilter(value as OutageSeverity | '');
    onSeverityFilter(value ? (value as OutageSeverity) : undefined);
  };

  return (
    <div className="space-y-4">
      {/* Header */}
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold text-gray-900">{t('outages.title')}</h1>
        {onCreate && (
          <button
            type="button"
            onClick={onCreate}
            className="px-4 py-2 text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 rounded-md"
          >
            {t('outages.createNew')}
          </button>
        )}
      </div>

      {/* Filters */}
      <div className="flex flex-wrap gap-3">
        <select
          value={statusFilter}
          onChange={(e) => handleStatusChange(e.target.value)}
          className="px-3 py-2 text-sm border border-gray-300 rounded-md focus:ring-blue-500 focus:border-blue-500"
        >
          <option value="">{t('outages.allStatuses')}</option>
          {statusOptions.map((status) => (
            <option key={status} value={status}>
              {t(`outages.status.${status}`)}
            </option>
          ))}
        </select>

        <select
          value={commodityFilter}
          onChange={(e) => handleCommodityChange(e.target.value)}
          className="px-3 py-2 text-sm border border-gray-300 rounded-md focus:ring-blue-500 focus:border-blue-500"
        >
          <option value="">{t('outages.allCommodities')}</option>
          {commodityOptions.map((commodity) => (
            <option key={commodity} value={commodity}>
              {t(`outages.commodity.${commodity}`)}
            </option>
          ))}
        </select>

        <select
          value={severityFilter}
          onChange={(e) => handleSeverityChange(e.target.value)}
          className="px-3 py-2 text-sm border border-gray-300 rounded-md focus:ring-blue-500 focus:border-blue-500"
        >
          <option value="">{t('outages.allSeverities')}</option>
          {severityOptions.map((severity) => (
            <option key={severity} value={severity}>
              {t(`outages.severity.${severity}`)}
            </option>
          ))}
        </select>
      </div>

      {/* Loading state */}
      {isLoading && (
        <div className="flex items-center justify-center py-12">
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600" />
        </div>
      )}

      {/* Error state — a failed fetch must be distinguishable from an empty
          result, otherwise a genuine failure is hidden as "No outages found". */}
      {!isLoading && isError && (
        <div className="text-center py-12" role="alert">
          <svg
            className="mx-auto h-12 w-12 text-red-400"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
            aria-hidden="true"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"
            />
          </svg>
          <p className="mt-4 text-gray-900 font-medium">{t('outages.errors.fetchFailedTitle')}</p>
          <p className="mt-1 text-gray-500">{t('outages.errors.fetchFailedDescription')}</p>
          {onRetry && (
            <button
              type="button"
              onClick={onRetry}
              className="mt-4 px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 transition-colors"
            >
              {t('outages.errors.retry')}
            </button>
          )}
        </div>
      )}

      {/* Empty state */}
      {!isLoading && !isError && outages.length === 0 && (
        <div className="text-center py-12">
          <p className="text-gray-500">{t('outages.noResults')}</p>
        </div>
      )}

      {/* List */}
      {!isLoading && !isError && outages.length > 0 && (
        <div className="space-y-3">
          {outages.map((outage) => (
            <OutageCard key={outage.id} outage={outage} onView={onView} onEdit={onEdit} />
          ))}
        </div>
      )}

      {/* Pagination */}
      {totalPages > 1 && (
        <div className="flex items-center justify-between pt-4">
          <p className="text-sm text-gray-500">
            {t('common.showing')} {(page - 1) * pageSize + 1} {t('common.to')}{' '}
            {Math.min(page * pageSize, total)} {t('common.of')} {total}
          </p>
          <div className="flex gap-2">
            <button
              type="button"
              onClick={() => onPageChange(page - 1)}
              disabled={page <= 1}
              className="px-3 py-1.5 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-md hover:bg-gray-50 disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {t('common.previous')}
            </button>
            <button
              type="button"
              onClick={() => onPageChange(page + 1)}
              disabled={page >= totalPages}
              className="px-3 py-1.5 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-md hover:bg-gray-50 disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {t('common.next')}
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
