import type {
  AnnouncementStatus,
  AnnouncementSummary,
  AnnouncementTargetType,
} from '@ppt/api-client';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { AnnouncementCard } from './AnnouncementCard';
import { PinnedAnnouncementsBand } from './PinnedAnnouncementsBand';

interface AnnouncementListProps {
  announcements: AnnouncementSummary[];
  total: number;
  page: number;
  pageSize: number;
  isLoading?: boolean;
  /** True when the list query failed. Renders an inline error state. */
  isError?: boolean;
  /** Retry the failed list query. */
  onRetry?: () => void;
  /** Pinned published announcements shown in the sticky band above the list (Story 6.4) */
  pinnedAnnouncements?: AnnouncementSummary[];
  onPageChange: (page: number) => void;
  onStatusFilter: (status?: AnnouncementStatus) => void;
  onTargetTypeFilter: (targetType?: AnnouncementTargetType) => void;
  onView: (id: string) => void;
  onEdit: (id: string) => void;
  onDelete: (id: string) => void;
  onPublish: (id: string) => void;
  onArchive: (id: string) => void;
  onPin: (id: string, pinned: boolean) => void;
  onCreate: () => void;
}

export function AnnouncementList({
  announcements,
  total,
  page,
  pageSize,
  isLoading,
  isError,
  onRetry,
  pinnedAnnouncements = [],
  onPageChange,
  onStatusFilter,
  onTargetTypeFilter,
  onView,
  onEdit,
  onDelete,
  onPublish,
  onArchive,
  onPin,
  onCreate,
}: AnnouncementListProps) {
  const { t } = useTranslation();
  const [statusFilter, setStatusFilter] = useState<AnnouncementStatus | ''>('');
  const [targetTypeFilter, setTargetTypeFilter] = useState<AnnouncementTargetType | ''>('');

  const totalPages = Math.ceil(total / pageSize);

  const handleStatusChange = (value: string) => {
    setStatusFilter(value as AnnouncementStatus | '');
    onStatusFilter(value ? (value as AnnouncementStatus) : undefined);
  };

  const handleTargetTypeChange = (value: string) => {
    setTargetTypeFilter(value as AnnouncementTargetType | '');
    onTargetTypeFilter(value ? (value as AnnouncementTargetType) : undefined);
  };

  return (
    <div>
      {/* Pinned announcements sticky band (Story 6.4) */}
      <PinnedAnnouncementsBand announcements={pinnedAnnouncements} onView={onView} />

      {/* Header with filters */}
      <div className="mb-6 flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4">
        <h1 className="text-2xl font-bold text-gray-900">Announcements</h1>
        <button
          type="button"
          onClick={onCreate}
          className="px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 transition-colors"
        >
          Create Announcement
        </button>
      </div>

      {/* Filters */}
      <div className="mb-6 flex flex-wrap gap-4">
        <select
          value={statusFilter}
          onChange={(e) => handleStatusChange(e.target.value)}
          className="px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
        >
          <option value="">All Statuses</option>
          <option value="draft">Draft</option>
          <option value="scheduled">Scheduled</option>
          <option value="published">Published</option>
          <option value="archived">Archived</option>
        </select>

        <select
          value={targetTypeFilter}
          onChange={(e) => handleTargetTypeChange(e.target.value)}
          className="px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
        >
          <option value="">All Targets</option>
          <option value="all">All Users</option>
          <option value="building">Building</option>
          <option value="units">Specific Units</option>
          <option value="roles">By Role</option>
        </select>
      </div>

      {/* List */}
      {isError ? (
        <div
          role="alert"
          className="text-center py-12 border border-red-200 bg-red-50 rounded-md text-red-700"
        >
          <p className="font-medium">
            {t('announcements.failedToLoad', { defaultValue: 'Failed to load announcements' })}
          </p>
          {onRetry && (
            <button
              type="button"
              onClick={onRetry}
              className="mt-3 px-3 py-1 border border-red-300 rounded text-red-700 hover:bg-red-100"
            >
              {t('common.retry', { defaultValue: 'Retry' })}
            </button>
          )}
        </div>
      ) : isLoading ? (
        <div className="flex items-center justify-center py-12">
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600" />
        </div>
      ) : announcements.length === 0 ? (
        <div className="text-center py-12 text-gray-500">
          <p>No announcements found.</p>
          <button
            type="button"
            onClick={onCreate}
            className="mt-4 text-blue-600 hover:text-blue-800"
          >
            Create your first announcement
          </button>
        </div>
      ) : (
        <div className="grid gap-4">
          {announcements.map((announcement) => (
            <AnnouncementCard
              key={announcement.id}
              announcement={announcement}
              onView={onView}
              onEdit={onEdit}
              onDelete={onDelete}
              onPublish={onPublish}
              onArchive={onArchive}
              onPin={onPin}
            />
          ))}
        </div>
      )}

      {/* Pagination */}
      {totalPages > 1 && (
        <div className="mt-6 flex items-center justify-between">
          <p className="text-sm text-gray-500">
            Showing {(page - 1) * pageSize + 1} to {Math.min(page * pageSize, total)} of {total}
          </p>
          <div className="flex gap-2">
            <button
              type="button"
              onClick={() => onPageChange(page - 1)}
              disabled={page === 1}
              className="px-3 py-1 border rounded-md disabled:opacity-50 disabled:cursor-not-allowed hover:bg-gray-50"
            >
              Previous
            </button>
            <span className="px-3 py-1">
              Page {page} of {totalPages}
            </span>
            <button
              type="button"
              onClick={() => onPageChange(page + 1)}
              disabled={page === totalPages}
              className="px-3 py-1 border rounded-md disabled:opacity-50 disabled:cursor-not-allowed hover:bg-gray-50"
            >
              Next
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
