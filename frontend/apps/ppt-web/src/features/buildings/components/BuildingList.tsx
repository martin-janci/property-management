/**
 * BuildingList Component
 *
 * Displays a list of buildings with filtering and pagination.
 *
 * @see Story 81.2 - Wire buildings page to API
 */

import type { Building, BuildingStatus, BuildingType } from '@ppt/api-client';
import { Trans, useTranslation } from 'react-i18next';
import { BuildingCard } from './BuildingCard';

interface BuildingListProps {
  buildings: Building[];
  total: number;
  page: number;
  pageSize: number;
  isLoading?: boolean;
  onPageChange: (page: number) => void;
  onStatusFilter: (status?: BuildingStatus) => void;
  onTypeFilter: (type?: BuildingType) => void;
  onView: (id: string) => void;
  onEdit?: (id: string) => void;
  onCreate?: () => void;
}

const STATUS_OPTIONS: { value: BuildingStatus | ''; labelKey: string }[] = [
  { value: '', labelKey: 'buildings.list.allStatuses' },
  { value: 'active', labelKey: 'buildings.status.active' },
  { value: 'under_construction', labelKey: 'buildings.status.under_construction' },
  { value: 'renovation', labelKey: 'buildings.status.renovation' },
  { value: 'inactive', labelKey: 'buildings.status.inactive' },
];

const TYPE_OPTIONS: { value: BuildingType | ''; labelKey: string }[] = [
  { value: '', labelKey: 'buildings.list.allTypes' },
  { value: 'residential', labelKey: 'buildings.type.residential' },
  { value: 'commercial', labelKey: 'buildings.type.commercial' },
  { value: 'mixed', labelKey: 'buildings.type.mixed' },
  { value: 'industrial', labelKey: 'buildings.type.industrial' },
];

export function BuildingList({
  buildings,
  total,
  page,
  pageSize,
  isLoading,
  onPageChange,
  onStatusFilter,
  onTypeFilter,
  onView,
  onEdit,
  onCreate,
}: BuildingListProps) {
  const { t } = useTranslation();
  const totalPages = Math.ceil(total / pageSize);

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">{t('buildings.title')}</h1>
          <p className="text-sm text-gray-500 mt-1">
            {t('buildings.list.subtitle', { count: total })}
          </p>
        </div>
        {onCreate && (
          <button
            type="button"
            onClick={onCreate}
            className="inline-flex items-center px-4 py-2 border border-transparent text-sm font-medium rounded-md shadow-sm text-white bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500"
          >
            <svg className="w-5 h-5 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M12 6v6m0 0v6m0-6h6m-6 0H6"
              />
            </svg>
            {t('buildings.list.addBuilding')}
          </button>
        )}
      </div>

      {/* Filters */}
      <div className="flex flex-wrap gap-4">
        <select
          className="block w-40 rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 sm:text-sm"
          onChange={(e) => onStatusFilter(e.target.value as BuildingStatus | undefined)}
          aria-label={t('buildings.list.statusFilter')}
          defaultValue=""
        >
          {STATUS_OPTIONS.map((option) => (
            <option key={option.value} value={option.value}>
              {t(option.labelKey)}
            </option>
          ))}
        </select>

        <select
          className="block w-40 rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 sm:text-sm"
          onChange={(e) => onTypeFilter(e.target.value as BuildingType | undefined)}
          aria-label={t('buildings.list.typeFilter')}
          defaultValue=""
        >
          {TYPE_OPTIONS.map((option) => (
            <option key={option.value} value={option.value}>
              {t(option.labelKey)}
            </option>
          ))}
        </select>
      </div>

      {/* Loading State */}
      {isLoading && (
        <div className="flex justify-center py-12">
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600" />
        </div>
      )}

      {/* Empty State */}
      {!isLoading && buildings.length === 0 && (
        <div className="text-center py-12">
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
              d="M19 21V5a2 2 0 00-2-2H7a2 2 0 00-2 2v16m14 0h2m-2 0h-5m-9 0H3m2 0h5M9 7h1m-1 4h1m4-4h1m-1 4h1m-5 10v-5a1 1 0 011-1h2a1 1 0 011 1v5m-4 0h4"
            />
          </svg>
          <h3 className="mt-2 text-sm font-medium text-gray-900">
            {t('buildings.list.emptyTitle')}
          </h3>
          <p className="mt-1 text-sm text-gray-500">{t('buildings.list.emptyBody')}</p>
          {onCreate && (
            <div className="mt-6">
              <button
                type="button"
                onClick={onCreate}
                className="inline-flex items-center px-4 py-2 border border-transparent shadow-sm text-sm font-medium rounded-md text-white bg-blue-600 hover:bg-blue-700"
              >
                {t('buildings.list.addBuilding')}
              </button>
            </div>
          )}
        </div>
      )}

      {/* Building Grid */}
      {!isLoading && buildings.length > 0 && (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
          {buildings.map((building) => (
            <BuildingCard key={building.id} building={building} onView={onView} onEdit={onEdit} />
          ))}
        </div>
      )}

      {/* Pagination */}
      {totalPages > 1 && (
        <div className="flex items-center justify-between border-t border-gray-200 bg-white px-4 py-3 sm:px-6">
          <div className="flex flex-1 justify-between sm:hidden">
            <button
              type="button"
              onClick={() => onPageChange(page - 1)}
              disabled={page === 1}
              className="relative inline-flex items-center rounded-md border border-gray-300 bg-white px-4 py-2 text-sm font-medium text-gray-700 hover:bg-gray-50 disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {t('common.previous')}
            </button>
            <button
              type="button"
              onClick={() => onPageChange(page + 1)}
              disabled={page === totalPages}
              className="relative ml-3 inline-flex items-center rounded-md border border-gray-300 bg-white px-4 py-2 text-sm font-medium text-gray-700 hover:bg-gray-50 disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {t('common.next')}
            </button>
          </div>
          <div className="hidden sm:flex sm:flex-1 sm:items-center sm:justify-between">
            <div>
              <p className="text-sm text-gray-700">
                <Trans
                  i18nKey="buildings.list.pageOf"
                  values={{ page, totalPages }}
                  components={{ bold: <span className="font-medium" /> }}
                />
              </p>
            </div>
            <div>
              <nav className="isolate inline-flex -space-x-px rounded-md shadow-sm">
                <button
                  type="button"
                  onClick={() => onPageChange(page - 1)}
                  disabled={page === 1}
                  className="relative inline-flex items-center rounded-l-md px-2 py-2 text-gray-400 ring-1 ring-inset ring-gray-300 hover:bg-gray-50 focus:z-20 focus:outline-offset-0 disabled:opacity-50 disabled:cursor-not-allowed"
                >
                  <span className="sr-only">{t('common.previous')}</span>
                  <svg className="h-5 w-5" viewBox="0 0 20 20" fill="currentColor">
                    <path
                      fillRule="evenodd"
                      d="M12.79 5.23a.75.75 0 01-.02 1.06L8.832 10l3.938 3.71a.75.75 0 11-1.04 1.08l-4.5-4.25a.75.75 0 010-1.08l4.5-4.25a.75.75 0 011.06.02z"
                      clipRule="evenodd"
                    />
                  </svg>
                </button>
                <button
                  type="button"
                  onClick={() => onPageChange(page + 1)}
                  disabled={page === totalPages}
                  className="relative inline-flex items-center rounded-r-md px-2 py-2 text-gray-400 ring-1 ring-inset ring-gray-300 hover:bg-gray-50 focus:z-20 focus:outline-offset-0 disabled:opacity-50 disabled:cursor-not-allowed"
                >
                  <span className="sr-only">{t('common.next')}</span>
                  <svg className="h-5 w-5" viewBox="0 0 20 20" fill="currentColor">
                    <path
                      fillRule="evenodd"
                      d="M7.21 14.77a.75.75 0 01.02-1.06L11.168 10 7.23 6.29a.75.75 0 111.04-1.08l4.5 4.25a.75.75 0 010 1.08l-4.5 4.25a.75.75 0 01-1.06-.02z"
                      clipRule="evenodd"
                    />
                  </svg>
                </button>
              </nav>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

BuildingList.displayName = 'BuildingList';
