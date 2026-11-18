/**
 * Facilities Page (Epic 56: Facility Booking).
 *
 * Lists all facilities in a building with filtering options.
 */

import type { FacilitySummary, FacilityType, ListFacilitiesQuery } from '@ppt/api-client';
import { listFacilities } from '@ppt/api-client';
import { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Link, useNavigate, useParams } from 'react-router-dom';
import { useAuth } from '../../../contexts';
import { isManagerRole } from '../../../routes/shared';
import { FacilityList } from '../components';

const PAGE_SIZE = 10;

export function FacilitiesPage() {
  const { buildingId } = useParams<{ buildingId: string }>();
  const navigate = useNavigate();
  // Manager-only affordances (create/edit, pending approvals) are gated on the
  // authenticated user's role — mirrors isManagerRole() usage elsewhere in
  // ppt-web (e.g. PendingBookingsPage). Backend still enforces authorization;
  // this is UI display only.
  const { user } = useAuth();
  const isManager = isManagerRole(user?.role);
  const { t } = useTranslation();

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
      {/* Gap audit R6 P1-9: the pending-approvals page had no entry point. */}
      <nav className="mb-4 flex flex-wrap gap-4 text-sm" aria-label={t('facilities.title')}>
        {isManager && (
          <Link
            to={`/buildings/${buildingId}/facilities/pending`}
            className="text-blue-600 hover:underline"
          >
            {t('facilities.bookings.pendingTitle')}
          </Link>
        )}
        <Link to="/facilities/bookings/my" className="text-blue-600 hover:underline">
          {t('facilities.bookings.myTitle')}
        </Link>
      </nav>
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
        onBook={handleBook}
        onEdit={handleEdit}
        onCreate={handleCreate}
      />
    </div>
  );
}
