/**
 * BuildingsPage - main page listing all buildings.
 *
 * Epic 81: Frontend API Integration (Story 81.2)
 * Wires the buildings page to the API client.
 *
 * @see Story 81.2 - Wire buildings page to API
 */

import type { BuildingStatus, BuildingType, ListBuildingsParams } from '@ppt/api-client';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { BuildingList } from '../components/BuildingList';
import { useBuildings } from '../hooks';

interface BuildingsPageProps {
  onNavigateToCreate?: () => void;
  onNavigateToView: (id: string) => void;
  onNavigateToEdit?: (id: string) => void;
}

export function BuildingsPage({
  onNavigateToCreate,
  onNavigateToView,
  onNavigateToEdit,
}: BuildingsPageProps) {
  const { t } = useTranslation();
  const [page, setPage] = useState(1);
  const [pageSize] = useState(12);
  const [filters, setFilters] = useState<Omit<ListBuildingsParams, 'page' | 'pageSize'>>({});

  // Query buildings from API
  const { data, isLoading, error } = useBuildings({
    ...filters,
    page,
    pageSize,
  });

  const buildings = data?.items ?? [];
  const total = data?.total ?? 0;

  const handlePageChange = (newPage: number) => {
    setPage(newPage);
  };

  const handleStatusFilter = (status?: BuildingStatus) => {
    setFilters((prev) => ({ ...prev, status }));
    setPage(1);
  };

  const handleTypeFilter = (type?: BuildingType) => {
    setFilters((prev) => ({ ...prev, type }));
    setPage(1);
  };

  // Show error state
  if (error) {
    return (
      <div className="max-w-6xl mx-auto px-4 py-8">
        <div className="bg-red-50 border border-red-200 rounded-md p-4">
          <div className="flex">
            <div className="flex-shrink-0">
              <svg className="h-5 w-5 text-red-400" viewBox="0 0 20 20" fill="currentColor">
                <path
                  fillRule="evenodd"
                  d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z"
                  clipRule="evenodd"
                />
              </svg>
            </div>
            <div className="ml-3">
              <h3 className="text-sm font-medium text-red-800">
                {t('buildings.errors.loadFailed')}
              </h3>
              <div className="mt-2 text-sm text-red-700">
                {error instanceof Error ? error.message : t('buildings.errors.unexpected')}
              </div>
            </div>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="max-w-6xl mx-auto px-4 py-8">
      <BuildingList
        buildings={buildings}
        total={total}
        page={page}
        pageSize={pageSize}
        isLoading={isLoading}
        onPageChange={handlePageChange}
        onStatusFilter={handleStatusFilter}
        onTypeFilter={handleTypeFilter}
        onView={onNavigateToView}
        onEdit={onNavigateToEdit}
        onCreate={onNavigateToCreate}
      />
    </div>
  );
}

BuildingsPage.displayName = 'BuildingsPage';
