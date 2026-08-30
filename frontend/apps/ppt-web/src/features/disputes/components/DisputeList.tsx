/**
 * DisputeList - displays a filterable list of disputes.
 * Epic 77: Dispute Resolution (Story 77.1)
 */

import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Spinner } from '../../../components/Spinner';
import {
  categoryLabels,
  DisputeCard,
  type DisputeCategory,
  type DisputePriority,
  type DisputeStatus,
  type DisputeSummary,
  priorityLabels,
  statusLabels,
} from './DisputeCard';

interface DisputeListProps {
  disputes: DisputeSummary[];
  total: number;
  page: number;
  pageSize: number;
  isLoading?: boolean;
  onPageChange: (page: number) => void;
  onStatusFilter: (status?: DisputeStatus) => void;
  onPriorityFilter: (priority?: DisputePriority) => void;
  onCategoryFilter: (category?: DisputeCategory) => void;
  onSearch: (search: string) => void;
  onView: (id: string) => void;
  onManage?: (id: string) => void;
  onCreate: () => void;
}

export function DisputeList({
  disputes,
  total,
  page,
  pageSize,
  isLoading,
  onPageChange,
  onStatusFilter,
  onPriorityFilter,
  onCategoryFilter,
  onSearch,
  onView,
  onManage,
  onCreate,
}: DisputeListProps) {
  const { t } = useTranslation();
  const [searchInput, setSearchInput] = useState('');
  const [activeStatus, setActiveStatus] = useState<DisputeStatus | undefined>();
  const [activePriority, setActivePriority] = useState<DisputePriority | undefined>();
  const [activeCategory, setActiveCategory] = useState<DisputeCategory | undefined>();

  const totalPages = Math.ceil(total / pageSize);

  const handleSearchSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    onSearch(searchInput);
  };

  const handleStatusChange = (status: DisputeStatus | '') => {
    const newStatus = status === '' ? undefined : status;
    setActiveStatus(newStatus);
    onStatusFilter(newStatus);
  };

  const handlePriorityChange = (priority: DisputePriority | '') => {
    const newPriority = priority === '' ? undefined : priority;
    setActivePriority(newPriority);
    onPriorityFilter(newPriority);
  };

  const handleCategoryChange = (category: DisputeCategory | '') => {
    const newCategory = category === '' ? undefined : category;
    setActiveCategory(newCategory);
    onCategoryFilter(newCategory);
  };

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold text-gray-900">{t('disputes.title')}</h1>
        <button
          type="button"
          onClick={onCreate}
          className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700"
        >
          {t('disputes.fileNewDispute')}
        </button>
      </div>

      {/* Filters */}
      <div className="bg-white rounded-lg shadow p-4">
        <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
          {/* Search */}
          <form onSubmit={handleSearchSubmit} className="relative">
            <input
              type="text"
              placeholder={t('disputes.searchPlaceholder')}
              value={searchInput}
              onChange={(e) => setSearchInput(e.target.value)}
              className="w-full rounded-md border border-gray-300 px-3 py-2 pr-10 focus:outline-none focus:ring-2 focus:ring-blue-500"
            />
            <button
              type="submit"
              className="absolute right-2 top-1/2 -translate-y-1/2 text-gray-400 hover:text-gray-600"
            >
              {t('disputes.search')}
            </button>
          </form>

          {/* Status Filter */}
          <select
            value={activeStatus || ''}
            onChange={(e) => handleStatusChange(e.target.value as DisputeStatus | '')}
            className="rounded-md border border-gray-300 px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500"
          >
            <option value="">{t('disputes.allStatuses')}</option>
            {Object.entries(statusLabels).map(([value, label]) => (
              <option key={value} value={value}>
                {label}
              </option>
            ))}
          </select>

          {/* Priority Filter */}
          <select
            value={activePriority || ''}
            onChange={(e) => handlePriorityChange(e.target.value as DisputePriority | '')}
            className="rounded-md border border-gray-300 px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500"
          >
            <option value="">{t('disputes.allPriorities')}</option>
            {Object.entries(priorityLabels).map(([value, label]) => (
              <option key={value} value={value}>
                {label}
              </option>
            ))}
          </select>

          {/* Category Filter */}
          <select
            value={activeCategory || ''}
            onChange={(e) => handleCategoryChange(e.target.value as DisputeCategory | '')}
            className="rounded-md border border-gray-300 px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500"
          >
            <option value="">{t('disputes.allCategories')}</option>
            {Object.entries(categoryLabels).map(([value, label]) => (
              <option key={value} value={value}>
                {label}
              </option>
            ))}
          </select>
        </div>
      </div>

      {/* Results */}
      {isLoading ? (
        <div className="flex justify-center py-12">
          <Spinner size="lg" color="blue" />
        </div>
      ) : disputes.length === 0 ? (
        <div className="bg-white rounded-lg shadow p-8 text-center">
          <p className="text-gray-500">{t('disputes.noDisputesFound')}</p>
          <button
            type="button"
            onClick={onCreate}
            className="mt-4 text-blue-600 hover:text-blue-800"
          >
            {t('disputes.fileANewDispute')}
          </button>
        </div>
      ) : (
        <div className="space-y-4">
          {disputes.map((dispute) => (
            <DisputeCard key={dispute.id} dispute={dispute} onView={onView} onManage={onManage} />
          ))}
        </div>
      )}

      {/* Pagination */}
      {totalPages > 1 && (
        <div className="flex items-center justify-between">
          <p className="text-sm text-gray-500">
            {t('disputes.paginationShowing', {
              from: (page - 1) * pageSize + 1,
              to: Math.min(page * pageSize, total),
              total,
            })}
          </p>
          <div className="flex gap-2">
            <button
              type="button"
              onClick={() => onPageChange(page - 1)}
              disabled={page <= 1}
              className="px-3 py-1 border rounded-lg hover:bg-gray-50 disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {t('disputes.previous')}
            </button>
            {Array.from({ length: Math.min(5, totalPages) }, (_, i) => {
              const pageNum = page <= 3 ? i + 1 : page + i - 2;
              if (pageNum > totalPages) return null;
              return (
                <button
                  key={pageNum}
                  type="button"
                  onClick={() => onPageChange(pageNum)}
                  className={`px-3 py-1 rounded-lg ${
                    pageNum === page ? 'bg-blue-600 text-white' : 'border hover:bg-gray-50'
                  }`}
                >
                  {pageNum}
                </button>
              );
            })}
            <button
              type="button"
              onClick={() => onPageChange(page + 1)}
              disabled={page >= totalPages}
              className="px-3 py-1 border rounded-lg hover:bg-gray-50 disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {t('disputes.next')}
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
