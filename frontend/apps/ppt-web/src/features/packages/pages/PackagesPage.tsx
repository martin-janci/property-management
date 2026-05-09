/**
 * PackagesPage
 *
 * Main page for package management (Epic 58, Story 58.1-58.3).
 * Epic 90, Story 90.1: Wire up package handlers to API.
 */

import {
  type ApiConfig,
  getToken,
  type PackageStatus,
  usePackages,
  usePickupPackage,
  useReceivePackage,
} from '@ppt/api-client';
import { useCallback, useMemo, useState } from 'react';
import { PackageCard } from '../components/PackageCard';

// API base URL from environment
const API_BASE_URL = import.meta.env.VITE_API_BASE_URL ?? '';

type FilterStatus = 'all' | PackageStatus;

export function PackagesPage() {
  const [filter, setFilter] = useState<FilterStatus>('all');

  // API configuration
  // TODO(Phase-2): Make token reactive - add getToken to deps or use auth context
  // Phase 1: Token retrieved once at component mount
  const apiConfig: ApiConfig = useMemo(
    () => ({
      baseUrl: API_BASE_URL,
      accessToken: getToken() ?? undefined,
    }),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    []
  );

  // Fetch packages from API
  const { data: packagesData, isLoading, error } = usePackages(apiConfig);

  // Mutations for package status updates
  const receivePackage = useReceivePackage(apiConfig);
  const pickupPackage = usePickupPackage(apiConfig);

  // Get packages from API response or use empty array
  const packages = packagesData?.packages ?? [];
  const filteredPackages = packages.filter((pkg) => filter === 'all' || pkg.status === filter);

  const handleView = useCallback((_id: string) => {
    // TODO(Phase-2): Use React Router's useNavigate for SPA navigation
    // Phase 1: Full page reload for simplicity
    window.location.href = `/packages/${_id}`;
  }, []);

  const handleReceive = useCallback(
    (id: string) => {
      // TODO(Phase-2): Add modal to collect receiver signature, timestamp, photos
      // Phase 1: Empty data object for basic functionality
      receivePackage.mutate(
        { id, data: {} },
        {
          onError: (err) => {
            console.error('Failed to mark package as received:', err);
            alert('Failed to mark package as received. Please try again.');
          },
        }
      );
    },
    [receivePackage]
  );

  const handlePickup = useCallback(
    (id: string) => {
      // TODO(Phase-2): Add modal to collect pickup details (signature, ID verification)
      // Phase 1: Empty data object for basic functionality
      pickupPackage.mutate(
        { id, data: {} },
        {
          onError: (err) => {
            console.error('Failed to mark package as picked up:', err);
            alert('Failed to mark package as picked up. Please try again.');
          },
        }
      );
    },
    [pickupPackage]
  );

  // Loading state
  if (isLoading) {
    return (
      <div className="container mx-auto px-4 py-8">
        <div className="text-center py-12">
          <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600 mx-auto" />
          <p className="text-gray-500 mt-4">Loading packages...</p>
        </div>
      </div>
    );
  }

  // Error state
  if (error) {
    return (
      <div className="container mx-auto px-4 py-8">
        <div className="text-center py-12 bg-red-50 rounded-lg">
          <p className="text-red-600">Failed to load packages: {error.message}</p>
          <button
            type="button"
            onClick={() => window.location.reload()}
            className="mt-4 px-4 py-2 bg-red-600 text-white rounded-md hover:bg-red-700"
          >
            Retry
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="container mx-auto px-4 py-8">
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">Package Management</h1>
          <p className="text-gray-600 mt-1">Track and manage building packages</p>
        </div>
        <button
          type="button"
          className="px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 flex items-center gap-2"
        >
          <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <title>Add</title>
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4v16m8-8H4" />
          </svg>
          Register Package
        </button>
      </div>

      {/* Filters */}
      <div className="flex gap-2 mb-6">
        {(['all', 'expected', 'received', 'notified', 'picked_up', 'unclaimed'] as const).map(
          (status) => (
            <button
              key={status}
              type="button"
              onClick={() => setFilter(status)}
              className={`px-3 py-1.5 rounded-md text-sm font-medium transition-colors ${
                filter === status
                  ? 'bg-blue-600 text-white'
                  : 'bg-gray-100 text-gray-700 hover:bg-gray-200'
              }`}
            >
              {status === 'all'
                ? 'All'
                : status === 'picked_up'
                  ? 'Picked Up'
                  : status.charAt(0).toUpperCase() + status.slice(1)}
            </button>
          )
        )}
      </div>

      {/* Package List */}
      <div className="space-y-4">
        {filteredPackages.length === 0 ? (
          <div className="text-center py-12 bg-gray-50 rounded-lg">
            <svg
              className="w-12 h-12 text-gray-400 mx-auto mb-4"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <title>No packages</title>
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4"
              />
            </svg>
            <p className="text-gray-500">No packages found</p>
          </div>
        ) : (
          filteredPackages.map((pkg) => (
            <PackageCard
              key={pkg.id}
              pkg={pkg}
              onView={handleView}
              onReceive={handleReceive}
              onPickup={handlePickup}
            />
          ))
        )}
      </div>
    </div>
  );
}
