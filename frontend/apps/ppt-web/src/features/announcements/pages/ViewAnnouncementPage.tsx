import type {
  AcknowledgmentStats,
  AnnouncementAttachment,
  AnnouncementStatus,
  AnnouncementWithDetails,
} from '@ppt/api-client';
import { AcknowledgmentStats as AcknowledgmentStatsPanel } from '../components/AcknowledgmentStats';
import type { AnnouncementCommentsProps } from '../components/AnnouncementComments';
import { AnnouncementComments } from '../components/AnnouncementComments';
import './ViewAnnouncementPage.css';
import { useTranslation } from 'react-i18next';

interface ViewAnnouncementPageProps {
  announcement: AnnouncementWithDetails;
  attachments: AnnouncementAttachment[];
  isLoading?: boolean;
  onEdit: () => void;
  onPublish: () => void;
  onArchive: () => void;
  onPin: (pinned: boolean) => void;
  onDelete: () => void;
  onBack: () => void;
  onMarkRead?: () => void;
  onAcknowledge?: () => void;
  /** Acknowledgment stats for manager view — shown when acknowledgmentRequired is true */
  acknowledgmentStats?: AcknowledgmentStats;
  /**
   * Comment section props (Story 6.3).
   * When provided the comments thread is rendered below the announcement body.
   * Omit to suppress the section (e.g. commentsEnabled=false, or while loading).
   */
  commentsProps?: AnnouncementCommentsProps;
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

export function ViewAnnouncementPage({
  announcement,
  attachments,
  isLoading,
  onEdit,
  onPublish,
  onArchive,
  onPin,
  onDelete,
  onBack,
  onMarkRead,
  onAcknowledge,
  acknowledgmentStats,
  commentsProps,
}: ViewAnnouncementPageProps) {
  const { t } = useTranslation();
  const canEdit = announcement.status === 'draft' || announcement.status === 'scheduled';
  const canDelete = announcement.status === 'draft';
  const canPublish = announcement.status === 'draft' || announcement.status === 'scheduled';
  const canArchive = announcement.status === 'published';

  if (isLoading) {
    return (
      <div className="view-announcement__loading">
        <div className="view-announcement__spinner" aria-label={t('aria.loading')} />
      </div>
    );
  }

  return (
    <div className="view-announcement">
      <button type="button" onClick={onBack} className="view-announcement__back">
        <svg
          width="16"
          height="16"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
          aria-hidden="true"
        >
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
        </svg>
        Back to Announcements
      </button>

      <div className="view-announcement__card">
        <div className="view-announcement__header">
          <div>
            <div className="view-announcement__badge-row">
              {announcement.pinned && (
                <span className="view-announcement__pin-icon" title="Pinned">
                  <svg
                    width="20"
                    height="20"
                    fill="currentColor"
                    viewBox="0 0 20 20"
                    aria-hidden="true"
                  >
                    <path d="M9.828.722a.5.5 0 01.354 0l7 3A.5.5 0 0117.5 4v1.5a.5.5 0 01-.5.5h-1v4.5a.5.5 0 01-.5.5H13v5.5a.5.5 0 01-.5.5h-5a.5.5 0 01-.5-.5V11H4.5a.5.5 0 01-.5-.5V6h-1a.5.5 0 01-.5-.5V4a.5.5 0 01.328-.472l7-3z" />
                  </svg>
                </span>
              )}
              <span className={statusPillClass[announcement.status]}>
                {announcement.status.charAt(0).toUpperCase() + announcement.status.slice(1)}
              </span>
            </div>
            <h1 className="view-announcement__title">{announcement.title}</h1>
            <div className="view-announcement__byline">
              <span>By {announcement.authorName}</span>
              <span>{targetTypeLabels[announcement.targetType]}</span>
              {announcement.publishedAt && (
                <span>Published: {new Date(announcement.publishedAt).toLocaleDateString()}</span>
              )}
            </div>
          </div>

          <div className="view-announcement__stats">
            <span className="view-announcement__stat">
              <strong>{announcement.readCount}</strong> reads
            </span>
            {announcement.acknowledgmentRequired && (
              <span className="view-announcement__stat">
                <strong>{announcement.acknowledgedCount}</strong> acknowledged
              </span>
            )}
            {announcement.commentsEnabled && (
              <span className="view-announcement__stat">
                <strong>{announcement.commentCount}</strong> comments
              </span>
            )}
            <span className="view-announcement__stat">
              <strong>{announcement.attachmentCount}</strong> attachments
            </span>
          </div>
        </div>

        <div className="view-announcement__content">
          <div className="view-announcement__body">{announcement.content}</div>
        </div>

        {attachments.length > 0 && (
          <div className="view-announcement__attachments">
            <h3 className="view-announcement__attachments-title">Attachments</h3>
            <ul className="view-announcement__attachment-list">
              {attachments.map((attachment) => (
                <li key={attachment.id} className="view-announcement__attachment">
                  <svg
                    className="view-announcement__attachment-icon"
                    width="16"
                    height="16"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                    aria-hidden="true"
                  >
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      strokeWidth={2}
                      d="M15.172 7l-6.586 6.586a2 2 0 102.828 2.828l6.414-6.586a4 4 0 00-5.656-5.656l-6.415 6.585a6 6 0 108.486 8.486L20.5 13"
                    />
                  </svg>
                  <span className="view-announcement__attachment-link">{attachment.fileName}</span>
                  <span className="view-announcement__attachment-size">
                    ({Math.round(attachment.fileSize / 1024)} KB)
                  </span>
                </li>
              ))}
            </ul>
          </div>
        )}

        {announcement.acknowledgmentRequired && acknowledgmentStats && (
          <AcknowledgmentStatsPanel
            stats={acknowledgmentStats}
            className="view-announcement__ack-stats"
          />
        )}

        {/* Story 6.3: Comments & Discussion — rendered when commentsEnabled and commentsProps provided */}
        {announcement.commentsEnabled && commentsProps && (
          <AnnouncementComments {...commentsProps} />
        )}

        <div className="view-announcement__actions">
          {canEdit && (
            <button
              type="button"
              onClick={onEdit}
              className="view-announcement__btn view-announcement__btn--edit"
            >
              Edit
            </button>
          )}
          {canPublish && (
            <button
              type="button"
              onClick={onPublish}
              className="view-announcement__btn view-announcement__btn--publish"
            >
              Publish Now
            </button>
          )}
          {canArchive && (
            <button
              type="button"
              onClick={onArchive}
              className="view-announcement__btn view-announcement__btn--archive"
            >
              Archive
            </button>
          )}
          <button
            type="button"
            onClick={() => onPin(!announcement.pinned)}
            className="view-announcement__btn view-announcement__btn--pin"
          >
            {announcement.pinned ? 'Unpin' : 'Pin'}
          </button>
          {announcement.acknowledgmentRequired && onAcknowledge && (
            <button
              type="button"
              onClick={onAcknowledge}
              className="view-announcement__btn view-announcement__btn--ack"
            >
              Acknowledge
            </button>
          )}
          {onMarkRead && (
            <button
              type="button"
              onClick={onMarkRead}
              className="view-announcement__btn view-announcement__btn--read"
            >
              Mark as Read
            </button>
          )}
          {canDelete && (
            <button
              type="button"
              onClick={onDelete}
              className="view-announcement__btn view-announcement__btn--delete"
            >
              Delete
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
