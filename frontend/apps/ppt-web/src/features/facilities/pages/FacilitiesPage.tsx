/**
 * Facilities Page (Epic 56: Facility Booking).
 *
 * Lists all facilities in a building with filtering options.
 */

import type { FacilitySummary, FacilityType, ListFacilitiesQuery } from '@ppt/api-client';
import { listFacilities } from '@ppt/api-client';
import { useCallback, useEffect, useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import { FacilityList } from '../components';

const PAGE_SIZE = 10;

/**
 * Hook to determine if the current user has manager privileges.
 *
 * This controls visibility of manager-only features (create, edit facilities).
 * Note: Backend still enforces authorization - this is for UI display only.
 *
 * Integration steps when AuthContext is implemented:
 * 1. Import useAuth from the auth context
 * 2. Extract user role from the authenticated user
 * 3. Return true if role is 'manager' or 'admin'
 *
 * Example implementation:
 * ```typescript
 * function useIsManager(): boolean {
 *   const { user } = useAuth();
 *   return user?.role === 'manager' || user?.role === 'admin';
 * }
 * ```
 *
 * @returns true if user has manager privileges, false otherwise
 */
function useIsManager(): boolean {
  // Placeholder: Returns true during development to enable all manager features.
  // Replace with actual role check when AuthContext is implemented for ppt-web.
  // See frontend/apps/mobile/src/contexts/AuthContext.tsx for reference implementation.
  return true;
}

export function FacilitiesPage() {
  const { buildingId } = useParams<{ buildingId: string }>();
  const navigate = useNavigate();
  const isManager = useIsManager();

  const [facilities, setFacilities] = useState<FacilitySummary[]>([]);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(1);
  const [isLoading, setIsLoading] = useState(true);
  const [isError, setIsError] = useState(false);
  const [filters, setFilters] = useState<Omit<ListFacilitiesQuery, 'limit' | 'offset'>>({});

  const fetchFacilities = useCallback(async () => {
    if (!buildingId) return;

    setIsLoading(true);
    setIsError(false);
    try {
      const offset = (page - 1) * PAGE_SIZE;
      const response = await listFacilities(buildingId, {
        ...filters,
        limit: PAGE_SIZE,
        offset,
      });
      setFacilities(response.items);
      setTotal(response.total);
    } catch (error) {
      // A fetch failure must be distinguishable from an empty result — clear
      // any stale rows and flag the error so the list renders a dedicated
      // error state (with retry) instead of the "No facilities found"
      // placeholder, which would otherwise hide the outage from the user.
      console.error('Failed to fetch facilities:', error);
      setFacilities([]);
      setTotal(0);
      setIsError(true);
    } finally {
      setIsLoading(false);
    }
  }, [buildingId, filters, page]);

  useEffect(() => {
    fetchFacilities();
  }, [fetchFacilities]);

  const handleTypeFilter = (type?: FacilityType) => {
    setFilters((prev) => ({ ...prev, facility_type: type }));
    setPage(1); // Reset to first page when filter changes
  };

  const handleBookableFilter = (bookable?: boolean) => {
    setFilters((prev) => ({ ...prev, is_bookable: bookable }));
    setPage(1); // Reset to first page when filter changes
  };

  const handleView = (id: string) => {
    navigate(`/buildings/${buildingId}/facilities/${id}`);
  };

  const handleBook = (id: string) => {
    navigate(`/buildings/${buildingId}/facilities/${id}/book`);
  };

  const handleEdit = (id: string) => {
    navigate(`/buildings/${buildingId}/facilities/${id}/edit`);
  };

  const handleCreate = () => {
    navigate(`/buildings/${buildingId}/facilities/new`);
  };

  const handlePageChange = (newPage: number) => {
    setPage(newPage);
  };

  return (
    <div className="max-w-6xl mx-auto px-4 py-8">
      <FacilityList
        facilities={facilities}
        total={total}
        page={page}
        pageSize={PAGE_SIZE}
        isLoading={isLoading}
        isError={isError}
        isManager={isManager}
        onPageChange={handlePageChange}
        onRetry={fetchFacilities}
        onTypeFilter={handleTypeFilter}
        onBookableFilter={handleBookableFilter}
        onView={handleView}
        onBook={handleBook}
        onEdit={handleEdit}
        onCreate={handleCreate}
      />
    </div>
  );
}
