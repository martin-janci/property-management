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
  onPageChange: (page: number) => void;
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
  onPageChange,
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

      {/* Empty state */}
      {!isLoading && outages.length === 0 && (
        <div className="text-center py-12">
          <p className="text-gray-500">{t('outages.noResults')}</p>
        </div>
      )}

      {/* List */}
      {!isLoading && outages.length > 0 && (
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
