import type {
  AnnouncementStatus,
  AnnouncementSummary,
  AnnouncementTargetType,
  ListAnnouncementsParams,
} from '@ppt/api-client';
import { useState } from 'react';
import { AnnouncementList } from '../components/AnnouncementList';

interface AnnouncementsPageProps {
  announcements: AnnouncementSummary[];
  total: number;
  isLoading?: boolean;
  /** True when the announcements list query failed (surfaces an inline error state). */
  isError?: boolean;
  /** Retry the failed announcements list query. */
  onRetry?: () => void;
  /** Pinned published announcements for the sticky band (Story 6.4). Fetched
   *  separately in the route wrapper so the band is immune to list filters. */
  pinnedAnnouncements?: AnnouncementSummary[];
  onNavigateToCreate: () => void;
  onNavigateToView: (id: string) => void;
  onNavigateToEdit: (id: string) => void;
  onDelete: (id: string) => void;
  onPublish: (id: string) => void;
  onArchive: (id: string) => void;
  onPin: (id: string, pinned: boolean) => void;
  onFilterChange: (params: ListAnnouncementsParams) => void;
}

export function AnnouncementsPage({
  announcements,
  total,
  isLoading,
  isError,
  onRetry,
  pinnedAnnouncements = [],
  onNavigateToCreate,
  onNavigateToView,
  onNavigateToEdit,
  onDelete,
  onPublish,
  onArchive,
  onPin,
  onFilterChange,
}: AnnouncementsPageProps) {
  const [page, setPage] = useState(1);
  const [pageSize] = useState(10);
  const [filters, setFilters] = useState<ListAnnouncementsParams>({});

  const handlePageChange = (newPage: number) => {
    setPage(newPage);
    onFilterChange({ ...filters, page: newPage, pageSize });
  };

  const handleStatusFilter = (status?: AnnouncementStatus) => {
    const newFilters = { ...filters, status, page: 1 };
    setFilters(newFilters);
    setPage(1);
    onFilterChange(newFilters);
  };

  const handleTargetTypeFilter = (targetType?: AnnouncementTargetType) => {
    const newFilters = { ...filters, targetType, page: 1 };
    setFilters(newFilters);
    setPage(1);
    onFilterChange(newFilters);
  };

  return (
    <div className="max-w-4xl mx-auto px-4 py-8">
      <AnnouncementList
        announcements={announcements}
        total={total}
        page={page}
        pageSize={pageSize}
        isLoading={isLoading}
        isError={isError}
        onRetry={onRetry}
        pinnedAnnouncements={pinnedAnnouncements}
        onPageChange={handlePageChange}
        onStatusFilter={handleStatusFilter}
        onTargetTypeFilter={handleTargetTypeFilter}
        onView={onNavigateToView}
        onEdit={onNavigateToEdit}
        onDelete={onDelete}
        onPublish={onPublish}
        onArchive={onArchive}
        onPin={onPin}
        onCreate={onNavigateToCreate}
      />
    </div>
  );
}
