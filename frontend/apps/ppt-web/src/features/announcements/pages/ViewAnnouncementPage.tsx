import type {
  AcknowledgmentStats,
  AnnouncementAttachment,
  AnnouncementStatus,
  AnnouncementWithDetails,
} from '@ppt/api-client';
import { renderMarkdownSafe } from '../../../lib/markdown';
import { AcknowledgmentStats as AcknowledgmentStatsPanel } from '../components/AcknowledgmentStats';
import type { AnnouncementCommentsProps } from '../components/AnnouncementComments';
import { AnnouncementComments } from '../components/AnnouncementComments';
import './ViewAnnouncementPage.css';
import { useMemo } from 'react';
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
  /**
   * Whether the viewer may manage the announcement (edit / publish / archive /
   * pin / delete, read statistics). Residents get a read-only view.
   * Defaults to `true` for backwards compatibility.
   */
  canManage?: boolean;
}

const statusPillClass: Record<AnnouncementStatus, string> = {
  draft: 'ann-status-pill ann-status-pill--draft',
  scheduled: 'ann-status-pill ann-status-pill--scheduled',
  published: 'ann-status-pill ann-status-pill--published',
  archived: 'ann-status-pill ann-status-pill--archived',
};

const targetTypeLabelKeys = {
  all: 'announcements.target.allUsers',
  building: 'announcements.target.building',
  units: 'announcements.target.specificUnits',
  roles: 'announcements.target.byRole',
} as const;

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
  canManage = true,
}: ViewAnnouncementPageProps) {
  const { t, i18n } = useTranslation();
  const canEdit =
    canManage && (announcement.status === 'draft' || announcement.status === 'scheduled');
  const canDelete = canManage && announcement.status === 'draft';
  const canPublish =
    canManage && (announcement.status === 'draft' || announcement.status === 'scheduled');
  const canArchive = canManage && announcement.status === 'published';
  const bodyHtml = useMemo(() => renderMarkdownSafe(announcement.content), [announcement.content]);

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
        {t('announcements.view.backToList')}
      </button>

      <div className="view-announcement__card">
        <div className="view-announcement__header">
          <div>
            <div className="view-announcement__badge-row">
              {announcement.pinned && (
                <span className="view-announcement__pin-icon" title={t('announcements.pinned')}>
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
                {t(`announcements.status.${announcement.status}`)}
              </span>
            </div>
            <h1 className="view-announcement__title">{announcement.title}</h1>
            <div className="view-announcement__byline">
              {announcement.authorName && (
                <span>{t('announcements.view.byAuthor', { author: announcement.authorName })}</span>
              )}
              <span>{t(targetTypeLabelKeys[announcement.targetType])}</span>
              {announcement.publishedAt && (
                <span>
                  {t('announcements.card.publishedOn', {
                    date: new Date(announcement.publishedAt).toLocaleDateString(i18n.language),
                  })}
                </span>
              )}
            </div>
          </div>

          <div className="view-announcement__stats">
            {canManage && (
              <span className="view-announcement__stat">
                <strong>{announcement.readCount ?? 0}</strong>{' '}
                {t('announcements.view.reads', { count: announcement.readCount ?? 0 })}
              </span>
            )}
            {canManage && announcement.acknowledgmentRequired && (
              <span className="view-announcement__stat">
                <strong>{announcement.acknowledgedCount ?? 0}</strong>{' '}
                {t('announcements.view.acknowledgedCount', {
                  count: announcement.acknowledgedCount ?? 0,
                })}
              </span>
            )}
            {announcement.commentsEnabled && (
              <span className="view-announcement__stat">
                <strong>{announcement.commentCount ?? 0}</strong>{' '}
                {t('announcements.view.comments', { count: announcement.commentCount ?? 0 })}
              </span>
            )}
            <span className="view-announcement__stat">
              <strong>{announcement.attachmentCount ?? 0}</strong>{' '}
              {t('announcements.view.attachments', { count: announcement.attachmentCount ?? 0 })}
            </span>
          </div>
        </div>

        <div className="view-announcement__content">
          <div
            className="view-announcement__body view-announcement__body--markdown"
            // biome-ignore lint/security/noDangerouslySetInnerHtml: markdown is HTML-escaped first, then sanitized with a strict DOMPurify allowlist (lib/markdown)
            dangerouslySetInnerHTML={{ __html: bodyHtml }}
          />
        </div>

        {attachments.length > 0 && (
          <div className="view-announcement__attachments">
            <h3 className="view-announcement__attachments-title">
              {t('announcements.view.attachmentsTitle')}
            </h3>
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

        {canManage && announcement.acknowledgmentRequired && acknowledgmentStats && (
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
              {t('common.edit')}
            </button>
          )}
          {canPublish && (
            <button
              type="button"
              onClick={onPublish}
              className="view-announcement__btn view-announcement__btn--publish"
            >
              {t('announcements.actions.publishNow')}
            </button>
          )}
          {canArchive && (
            <button
              type="button"
              onClick={onArchive}
              className="view-announcement__btn view-announcement__btn--archive"
            >
              {t('announcements.actions.archive')}
            </button>
          )}
          {canManage && (
            <button
              type="button"
              onClick={() => onPin(!announcement.pinned)}
              className="view-announcement__btn view-announcement__btn--pin"
            >
              {announcement.pinned
                ? t('announcements.actions.unpin')
                : t('announcements.actions.pin')}
            </button>
          )}
          {announcement.acknowledgmentRequired && onAcknowledge && (
            <button
              type="button"
              onClick={onAcknowledge}
              className="view-announcement__btn view-announcement__btn--ack"
            >
              {t('announcements.actions.acknowledge')}
            </button>
          )}
          {onMarkRead && (
            <button
              type="button"
              onClick={onMarkRead}
              className="view-announcement__btn view-announcement__btn--read"
            >
              {t('announcements.actions.markAsRead')}
            </button>
          )}
          {canDelete && (
            <button
              type="button"
              onClick={onDelete}
              className="view-announcement__btn view-announcement__btn--delete"
            >
              {t('common.delete')}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
