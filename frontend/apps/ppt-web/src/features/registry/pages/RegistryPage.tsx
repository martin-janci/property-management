/**
 * RegistryPage Component
 *
 * Main page for building registries with tabs for pets and vehicles (Epic 57).
 * Wired to the registry API for CRUD operations.
 */

import type { RegistryStatus } from '@ppt/api-client';
import { useCallback, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useConfirmDialog } from '../../../components/useConfirmDialog';
import { PetRegistrationList } from '../components/PetRegistrationList';
import { VehicleRegistrationList } from '../components/VehicleRegistrationList';
import { useRegistryApi } from '../hooks/useRegistry';

export function RegistryPage() {
  const { t } = useTranslation();
  const { confirm, confirmDialog } = useConfirmDialog();
  const [activeTab, setActiveTab] = useState<'pets' | 'vehicles'>('pets');

  // Pet state
  const [petPage, setPetPage] = useState(1);
  const [petSearch, setPetSearch] = useState('');
  const [petStatusFilter, setPetStatusFilter] = useState<RegistryStatus | undefined>(undefined);

  // Vehicle state
  const [vehiclePage, setVehiclePage] = useState(1);
  const [vehicleSearch, setVehicleSearch] = useState('');
  const [vehicleStatusFilter, setVehicleStatusFilter] = useState<RegistryStatus | undefined>(
    undefined
  );

  const pageSize = 10;

  // Registry API hooks carrying the current auth context (token rotates via
  // the canonical useBuildings.ts getToken() + useMemo pattern).
  const registryHooks = useRegistryApi();

  // Pet queries
  const {
    data: petsData,
    isLoading: petsLoading,
    error: petsError,
  } = registryHooks.usePetsList({
    page: petPage,
    pageSize,
    status: petStatusFilter,
    search: petSearch || undefined,
  });

  // Vehicle queries
  const {
    data: vehiclesData,
    isLoading: vehiclesLoading,
    error: vehiclesError,
  } = registryHooks.useVehiclesList({
    page: vehiclePage,
    pageSize,
    status: vehicleStatusFilter,
    search: vehicleSearch || undefined,
  });

  // Pet mutations
  const deletePetMutation = registryHooks.useDeletePet();
  const reviewPetMutation = registryHooks.useReviewPet();

  // Vehicle mutations
  const deleteVehicleMutation = registryHooks.useDeleteVehicle();
  const reviewVehicleMutation = registryHooks.useReviewVehicle();

  // Pet handlers
  const handlePetPageChange = useCallback((page: number) => {
    setPetPage(page);
  }, []);

  const handlePetStatusFilter = useCallback((status?: string) => {
    setPetStatusFilter(status as RegistryStatus | undefined);
    setPetPage(1); // Reset to first page when filter changes
  }, []);

  const handlePetSearchChange = useCallback((search: string) => {
    setPetSearch(search);
    setPetPage(1); // Reset to first page when search changes
  }, []);

  const handlePetView = useCallback((id: string) => {
    window.location.href = `/registry/pets/${id}`;
  }, []);

  const handlePetEdit = useCallback((id: string) => {
    window.location.href = `/registry/pets/${id}/edit`;
  }, []);

  const handlePetDelete = useCallback(
    async (id: string) => {
      if (
        await confirm({
          message: t('registry.confirm.deletePet'),
          confirmLabel: t('common.delete'),
          variant: 'danger',
        })
      ) {
        try {
          await deletePetMutation.mutateAsync(id);
        } catch (error) {
          console.error('Failed to delete pet registration:', error);
        }
      }
    },
    [deletePetMutation, confirm, t]
  );

  const handlePetReview = useCallback(
    async (id: string) => {
      // Simple approval - in a real app, this would open a modal to approve/reject with reason
      if (
        await confirm({
          message: t('registry.confirm.approvePet'),
          confirmLabel: t('registry.confirm.approve'),
          variant: 'default',
        })
      ) {
        try {
          await reviewPetMutation.mutateAsync({
            id,
            data: { approve: true },
          });
        } catch (error) {
          console.error('Failed to review pet registration:', error);
        }
      }
    },
    [reviewPetMutation, confirm, t]
  );

  const handlePetCreate = useCallback(() => {
    window.location.href = '/registry/pets/new';
  }, []);

  // Vehicle handlers
  const handleVehiclePageChange = useCallback((page: number) => {
    setVehiclePage(page);
  }, []);

  const handleVehicleStatusFilter = useCallback((status?: string) => {
    setVehicleStatusFilter(status as RegistryStatus | undefined);
    setVehiclePage(1); // Reset to first page when filter changes
  }, []);

  const handleVehicleSearchChange = useCallback((search: string) => {
    setVehicleSearch(search);
    setVehiclePage(1); // Reset to first page when search changes
  }, []);

  const handleVehicleView = useCallback((id: string) => {
    window.location.href = `/registry/vehicles/${id}`;
  }, []);

  const handleVehicleEdit = useCallback((id: string) => {
    window.location.href = `/registry/vehicles/${id}/edit`;
  }, []);

  const handleVehicleDelete = useCallback(
    async (id: string) => {
      if (
        await confirm({
          message: t('registry.confirm.deleteVehicle'),
          confirmLabel: t('common.delete'),
          variant: 'danger',
        })
      ) {
        try {
          await deleteVehicleMutation.mutateAsync(id);
        } catch (error) {
          console.error('Failed to delete vehicle registration:', error);
        }
      }
    },
    [deleteVehicleMutation, confirm, t]
  );

  const handleVehicleReview = useCallback(
    async (id: string) => {
      // Simple approval - in a real app, this would open a modal to approve/reject with reason
      if (
        await confirm({
          message: t('registry.confirm.approveVehicle'),
          confirmLabel: t('registry.confirm.approve'),
          variant: 'default',
        })
      ) {
        try {
          await reviewVehicleMutation.mutateAsync({
            id,
            data: { approve: true },
          });
        } catch (error) {
          console.error('Failed to review vehicle registration:', error);
        }
      }
    },
    [reviewVehicleMutation, confirm, t]
  );

  const handleVehicleCreate = useCallback(() => {
    window.location.href = '/registry/vehicles/new';
  }, []);

  // Transform API response to component format
  const petListData = petsData
    ? {
        registrations: petsData.items,
        total: petsData.total,
        page: petsData.page,
        pageSize: petsData.pageSize,
      }
    : {
        registrations: [],
        total: 0,
        page: 1,
        pageSize,
      };

  const vehicleListData = vehiclesData
    ? {
        registrations: vehiclesData.items,
        total: vehiclesData.total,
        page: vehiclesData.page,
        pageSize: vehiclesData.pageSize,
      }
    : {
        registrations: [],
        total: 0,
        page: 1,
        pageSize,
      };

  return (
    <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
      <div className="mb-8">
        <h1 className="text-3xl font-bold text-gray-900">{t('registry.page.title')}</h1>
        <p className="mt-2 text-gray-600">{t('registry.page.subtitle')}</p>
      </div>

      {/* Tabs */}
      <div className="border-b border-gray-200 mb-6">
        <nav className="-mb-px flex space-x-8">
          <button
            type="button"
            onClick={() => setActiveTab('pets')}
            className={`${
              activeTab === 'pets'
                ? 'border-blue-500 text-blue-600'
                : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'
            } whitespace-nowrap py-4 px-1 border-b-2 font-medium text-sm transition-colors`}
          >
            <div className="flex items-center gap-2">
              <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <title>{t('registry.page.tabs.pets')}</title>
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M14.828 14.828a4 4 0 01-5.656 0M9 10h.01M15 10h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
                />
              </svg>
              {t('registry.page.tabs.pets')}
              {petsData?.total ? (
                <span className="ml-1 px-2 py-0.5 text-xs bg-gray-100 text-gray-600 rounded-full">
                  {petsData.total}
                </span>
              ) : null}
            </div>
          </button>
          <button
            type="button"
            onClick={() => setActiveTab('vehicles')}
            className={`${
              activeTab === 'vehicles'
                ? 'border-blue-500 text-blue-600'
                : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'
            } whitespace-nowrap py-4 px-1 border-b-2 font-medium text-sm transition-colors`}
          >
            <div className="flex items-center gap-2">
              <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <title>{t('registry.page.tabs.vehicles')}</title>
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M8 7v8a2 2 0 002 2h6M8 7V5a2 2 0 012-2h4.586a1 1 0 01.707.293l4.414 4.414a1 1 0 01.293.707V15a2 2 0 01-2 2h-2M8 7H6a2 2 0 00-2 2v10a2 2 0 002 2h8a2 2 0 002-2v-2"
                />
              </svg>
              {t('registry.page.tabs.vehicles')}
              {vehiclesData?.total ? (
                <span className="ml-1 px-2 py-0.5 text-xs bg-gray-100 text-gray-600 rounded-full">
                  {vehiclesData.total}
                </span>
              ) : null}
            </div>
          </button>
        </nav>
      </div>

      {/* Error display */}
      {(petsError || vehiclesError) && (
        <div className="mb-6 p-4 bg-red-50 border border-red-200 rounded-lg">
          <p className="text-sm text-red-700">{t('registry.page.loadError')}</p>
        </div>
      )}

      {/* Tab Content */}
      <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-6">
        {activeTab === 'pets' && (
          <PetRegistrationList
            registrations={petListData.registrations}
            total={petListData.total}
            page={petListData.page}
            pageSize={petListData.pageSize}
            onPageChange={handlePetPageChange}
            onStatusFilter={handlePetStatusFilter}
            onSearchChange={handlePetSearchChange}
            onView={handlePetView}
            onEdit={handlePetEdit}
            onDelete={handlePetDelete}
            onReview={handlePetReview}
            onCreate={handlePetCreate}
            isLoading={petsLoading}
          />
        )}

        {activeTab === 'vehicles' && (
          <VehicleRegistrationList
            registrations={vehicleListData.registrations}
            total={vehicleListData.total}
            page={vehicleListData.page}
            pageSize={vehicleListData.pageSize}
            onPageChange={handleVehiclePageChange}
            onStatusFilter={handleVehicleStatusFilter}
            onSearchChange={handleVehicleSearchChange}
            onView={handleVehicleView}
            onEdit={handleVehicleEdit}
            onDelete={handleVehicleDelete}
            onReview={handleVehicleReview}
            onCreate={handleVehicleCreate}
            isLoading={vehiclesLoading}
          />
        )}
      </div>
      {confirmDialog}
    </div>
  );
}
