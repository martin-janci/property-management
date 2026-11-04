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

  // Reflect pinned ordering in the list (Story 6.4): pinned items float to the
  // top. The backend already returns `ORDER BY pinned DESC, …`, but a stable
  // client-side pass keeps the order correct between an optimistic pin/unpin
  // toggle and the list refetch, and is robust to any caller that hands us an
  // un-sorted page. `Array.prototype.sort` is stable (ES2019+), so rows that
  // share a pinned state keep their server-provided order.
  const orderedAnnouncements =
    announcements.length > 1
      ? [...announcements].sort((a, b) => Number(b.pinned) - Number(a.pinned))
      : announcements;

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
        <h1 className="text-2xl font-bold text-gray-900">{t('announcements.title')}</h1>
        <button
          type="button"
          onClick={onCreate}
          className="px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 transition-colors"
        >
          {t('announcements.form.create')}
        </button>
      </div>

      {/* Filters */}
      <div className="mb-6 flex flex-wrap gap-4">
        <select
          value={statusFilter}
          onChange={(e) => handleStatusChange(e.target.value)}
          className="px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
        >
          <option value="">{t('announcements.list.allStatuses')}</option>
          <option value="draft">{t('announcements.status.draft')}</option>
          <option value="scheduled">{t('announcements.status.scheduled')}</option>
          <option value="published">{t('announcements.status.published')}</option>
          <option value="archived">{t('announcements.status.archived')}</option>
        </select>

        <select
          value={targetTypeFilter}
          onChange={(e) => handleTargetTypeChange(e.target.value)}
          className="px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
        >
          <option value="">{t('announcements.list.allTargets')}</option>
          <option value="all">{t('announcements.target.allUsers')}</option>
          <option value="building">{t('announcements.target.building')}</option>
          <option value="units">{t('announcements.target.specificUnits')}</option>
          <option value="roles">{t('announcements.target.byRole')}</option>
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
          <p>{t('announcements.list.empty')}</p>
          <button
            type="button"
            onClick={onCreate}
            className="mt-4 text-blue-600 hover:text-blue-800"
          >
            {t('announcements.list.createFirst')}
          </button>
        </div>
      ) : (
        <div className="grid gap-4">
          {orderedAnnouncements.map((announcement) => (
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
            {t('announcements.list.showingRange', {
              from: (page - 1) * pageSize + 1,
              to: Math.min(page * pageSize, total),
              total,
            })}
          </p>
          <div className="flex gap-2">
            <button
              type="button"
              onClick={() => onPageChange(page - 1)}
              disabled={page === 1}
              className="px-3 py-1 border rounded-md disabled:opacity-50 disabled:cursor-not-allowed hover:bg-gray-50"
            >
              {t('common.previous')}
            </button>
            <span className="px-3 py-1">
              {t('announcements.list.pageOf', { page, totalPages })}
            </span>
            <button
              type="button"
              onClick={() => onPageChange(page + 1)}
              disabled={page === totalPages}
              className="px-3 py-1 border rounded-md disabled:opacity-50 disabled:cursor-not-allowed hover:bg-gray-50"
            >
              {t('common.next')}
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
