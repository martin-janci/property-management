import type { AnnouncementStatus, AnnouncementSummary } from '@ppt/api-client';
import './AnnouncementCard.css';

interface AnnouncementCardProps {
  announcement: AnnouncementSummary;
  onView?: (id: string) => void;
  onEdit?: (id: string) => void;
  onDelete?: (id: string) => void;
  onPublish?: (id: string) => void;
  onArchive?: (id: string) => void;
  onPin?: (id: string, pinned: boolean) => void;
}

const statusPillClass: Record<AnnouncementStatus, string> = {
  draft: 'ann-status-pill ann-status-pill--draft',
  scheduled: 'ann-status-pill ann-status-pill--scheduled',
  published: 'ann-status-pill ann-status-pill--published',
  archived: 'ann-status-pill ann-status-pill--archived',
};

const targetTypeLabels = {
  all: 'All Users',
  building: 'Building',
  units: 'Specific Units',
  roles: 'By Role',
};

export function AnnouncementCard({
  announcement,
  onView,
  onEdit,
  onDelete,
  onPublish,
  onArchive,
  onPin,
}: AnnouncementCardProps) {
  const canEdit = announcement.status === 'draft' || announcement.status === 'scheduled';
  const canDelete = announcement.status === 'draft';
  const canPublish = announcement.status === 'draft' || announcement.status === 'scheduled';
  const canArchive = announcement.status === 'published';

  return (
    <div className="announcement-card">
      <div className="announcement-card__header">
        <div>
          <div className="announcement-card__title-row">
            {announcement.pinned && (
              <span className="announcement-card__pin-icon" title="Pinned">
                <svg width="16" height="16" fill="currentColor" viewBox="0 0 20 20" aria-hidden="true">
                  <path d="M9.828.722a.5.5 0 01.354 0l7 3A.5.5 0 0117.5 4v1.5a.5.5 0 01-.5.5h-1v4.5a.5.5 0 01-.5.5H13v5.5a.5.5 0 01-.5.5h-5a.5.5 0 01-.5-.5V11H4.5a.5.5 0 01-.5-.5V6h-1a.5.5 0 01-.5-.5V4a.5.5 0 01.328-.472l7-3z" />
                </svg>
              </span>
            )}
            <h3 className="announcement-card__title">{announcement.title}</h3>
          </div>
          <div className="announcement-card__meta">
            <span className={statusPillClass[announcement.status]}>
              {announcement.status.charAt(0).toUpperCase() + announcement.status.slice(1)}
            </span>
            <span className="announcement-card__target">
              {targetTypeLabels[announcement.targetType]}
            </span>
            {announcement.acknowledgmentRequired && (
              <span className="announcement-card__flag announcement-card__flag--ack">
                Acknowledgment Required
              </span>
            )}
            {announcement.commentsEnabled && (
              <span className="announcement-card__flag announcement-card__flag--comments">
                Comments Enabled
              </span>
            )}
          </div>
          {announcement.publishedAt && (
            <p className="announcement-card__date">
              Published: {new Date(announcement.publishedAt).toLocaleDateString()}
            </p>
          )}
        </div>
      </div>

      <div className="announcement-card__actions">
        <button
          type="button"
          onClick={() => onView?.(announcement.id)}
          className="announcement-card__action announcement-card__action--view"
        >
          View
        </button>
        {canEdit && (
          <button
            type="button"
            onClick={() => onEdit?.(announcement.id)}
            className="announcement-card__action announcement-card__action--edit"
          >
            Edit
          </button>
        )}
        {canPublish && (
          <button
            type="button"
            onClick={() => onPublish?.(announcement.id)}
            className="announcement-card__action announcement-card__action--publish"
          >
            Publish
          </button>
        )}
        {canArchive && (
          <button
            type="button"
            onClick={() => onArchive?.(announcement.id)}
            className="announcement-card__action announcement-card__action--archive"
          >
            Archive
          </button>
        )}
        <button
          type="button"
          onClick={() => onPin?.(announcement.id, !announcement.pinned)}
          className="announcement-card__action announcement-card__action--pin"
        >
          {announcement.pinned ? 'Unpin' : 'Pin'}
        </button>
        {canDelete && (
          <button
            type="button"
            onClick={() => onDelete?.(announcement.id)}
            className="announcement-card__action announcement-card__action--delete"
          >
            Delete
          </button>
        )}
      </div>
    </div>
  );
}
