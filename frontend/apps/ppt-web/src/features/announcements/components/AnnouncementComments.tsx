/**
 * AnnouncementComments — Story 6.3: Announcement Comments & Discussion
 *
 * Renders the comment thread for a published announcement.
 * Supports:
 *  - Listing top-level comments with nested replies
 *  - Creating new top-level comments (and replies)
 *  - Deleting own comments (or any comment for managers)
 */

import type { CommentWithAuthor } from '@ppt/api-client';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import './AnnouncementComments.css';

// ---------------------------------------------------------------------------
// Props
// ---------------------------------------------------------------------------

export interface AnnouncementCommentsProps {
  /** All comments (flat list; replies nested inside .replies) */
  comments: CommentWithAuthor[];
  /** Total comment count from the server */
  total: number;
  /** Whether comment operations are disabled (e.g. loading, commentsEnabled=false) */
  disabled?: boolean;
  /** ID of the currently authenticated user — determines delete affordance */
  currentUserId?: string;
  /** Whether the current user has manager-level moderation rights */
  isManager?: boolean;
  /** Called when user submits a new comment; parentId set for replies */
  onAddComment: (content: string, parentId?: string) => Promise<void>;
  /** Called when user deletes a comment; reason is optional moderation note */
  onDeleteComment: (commentId: string, reason?: string) => Promise<void>;
  /** Whether the comment list is being loaded */
  isLoading?: boolean;
}

// ---------------------------------------------------------------------------
// Single comment item
// ---------------------------------------------------------------------------

interface CommentItemProps {
  comment: CommentWithAuthor;
  currentUserId?: string;
  isManager?: boolean;
  disabled?: boolean;
  depth?: number;
  onReply: (content: string, parentId: string) => Promise<void>;
  onDelete: (commentId: string, reason?: string) => Promise<void>;
}

function CommentItem({
  comment,
  currentUserId,
  isManager,
  disabled,
  depth = 0,
  onReply,
  onDelete,
}: CommentItemProps) {
  const { t } = useTranslation();
  const [isReplying, setIsReplying] = useState(false);
  const [replyContent, setReplyContent] = useState('');
  const [isSubmittingReply, setIsSubmittingReply] = useState(false);
  const [isDeleting, setIsDeleting] = useState(false);

  const isOwnComment = currentUserId === comment.userId;
  const canDelete = isOwnComment || isManager;

  if (comment.isDeleted) {
    return (
      <div
        className="ann-comment ann-comment--deleted"
        style={{
          marginLeft: depth > 0 ? `${Math.min(depth, 3) * 24}px` : undefined,
        }}
      >
        <span className="ann-comment__deleted-text">{t('announcements.comments.deletedText')}</span>
      </div>
    );
  }

  const handleReplySubmit = async () => {
    if (!replyContent.trim() || isSubmittingReply) return;
    setIsSubmittingReply(true);
    try {
      await onReply(replyContent.trim(), comment.id);
      setReplyContent('');
      setIsReplying(false);
    } finally {
      setIsSubmittingReply(false);
    }
  };

  const handleDelete = async () => {
    if (isDeleting) return;
    setIsDeleting(true);
    try {
      await onDelete(comment.id);
    } finally {
      setIsDeleting(false);
    }
  };

  const authorInitial = comment.authorName.charAt(0).toUpperCase();
  const formattedDate = new Date(comment.createdAt).toLocaleDateString(undefined, {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
  });

  return (
    <div
      className="ann-comment"
      style={{
        marginLeft: depth > 0 ? `${Math.min(depth, 3) * 24}px` : undefined,
      }}
    >
      <div className="ann-comment__avatar" aria-hidden="true">
        {authorInitial}
      </div>

      <div className="ann-comment__body">
        <div className="ann-comment__meta">
          <span className="ann-comment__author">{comment.authorName}</span>
          <span className="ann-comment__date">{formattedDate}</span>
        </div>

        <p className="ann-comment__content">{comment.content}</p>

        {!disabled && (
          <div className="ann-comment__actions">
            {depth < 3 && (
              <button
                type="button"
                className="ann-comment__action-btn"
                onClick={() => setIsReplying((v) => !v)}
              >
                {t('announcements.comments.reply')}
              </button>
            )}
            {canDelete && (
              <button
                type="button"
                className="ann-comment__action-btn ann-comment__action-btn--danger"
                onClick={handleDelete}
                disabled={isDeleting}
                aria-busy={isDeleting}
              >
                {isDeleting
                  ? t('announcements.comments.deleting')
                  : t('announcements.comments.delete')}
              </button>
            )}
          </div>
        )}

        {isReplying && (
          <div className="ann-comment__reply-form">
            <textarea
              value={replyContent}
              onChange={(e) => setReplyContent(e.target.value)}
              placeholder={t('announcements.comments.replyPlaceholder')}
              className="ann-comment__textarea"
              rows={2}
              maxLength={2000}
              aria-label={t('aria.replyContent')}
            />
            <div className="ann-comment__reply-actions">
              <button
                type="button"
                className="ann-comment__submit-btn"
                onClick={handleReplySubmit}
                disabled={!replyContent.trim() || isSubmittingReply}
                aria-busy={isSubmittingReply}
              >
                {isSubmittingReply
                  ? t('announcements.comments.posting')
                  : t('announcements.comments.postReply')}
              </button>
              <button
                type="button"
                className="ann-comment__cancel-btn"
                onClick={() => {
                  setReplyContent('');
                  setIsReplying(false);
                }}
              >
                {t('announcements.comments.cancel')}
              </button>
            </div>
          </div>
        )}

        {comment.replies && comment.replies.length > 0 && (
          <div className="ann-comment__replies">
            {comment.replies.map((reply) => (
              <CommentItem
                key={reply.id}
                comment={reply}
                currentUserId={currentUserId}
                isManager={isManager}
                disabled={disabled}
                depth={depth + 1}
                onReply={onReply}
                onDelete={onDelete}
              />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Main component
// ---------------------------------------------------------------------------

export function AnnouncementComments({
  comments,
  total,
  disabled = false,
  currentUserId,
  isManager,
  onAddComment,
  onDeleteComment,
  isLoading,
}: AnnouncementCommentsProps) {
  const { t } = useTranslation();
  const [newComment, setNewComment] = useState('');
  const [isSubmitting, setIsSubmitting] = useState(false);

  const handleSubmit = async () => {
    if (!newComment.trim() || isSubmitting) return;
    setIsSubmitting(true);
    try {
      await onAddComment(newComment.trim());
      setNewComment('');
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      void handleSubmit();
    }
  };

  return (
    <section className="ann-comments" aria-label={t('aria.discussion')}>
      <h3 className="ann-comments__heading">
        {t('announcements.comments.discussion')}
        {total > 0 && <span className="ann-comments__count">{total}</span>}
      </h3>

      {!disabled && (
        <div className="ann-comments__compose">
          <textarea
            value={newComment}
            onChange={(e) => setNewComment(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder={t('announcements.comments.newCommentPlaceholder')}
            className="ann-comment__textarea"
            rows={3}
            maxLength={2000}
            disabled={isSubmitting}
            aria-label={t('aria.newComment')}
          />
          <div className="ann-comments__compose-footer">
            <span className="ann-comments__char-count">{newComment.length}/2000</span>
            <button
              type="button"
              className="ann-comment__submit-btn"
              onClick={handleSubmit}
              disabled={!newComment.trim() || isSubmitting}
              aria-busy={isSubmitting}
            >
              {isSubmitting
                ? t('announcements.comments.posting')
                : t('announcements.comments.postComment')}
            </button>
          </div>
        </div>
      )}

      {isLoading && (
        <div
          className="ann-comments__loading"
          aria-live="polite"
          aria-label={t('aria.loadingComments')}
        >
          <div className="view-announcement__spinner" />
        </div>
      )}

      {!isLoading && comments.length === 0 && (
        <p className="ann-comments__empty">{t('announcements.comments.empty')}</p>
      )}

      {!isLoading && comments.length > 0 && (
        <div className="ann-comments__list">
          {comments.map((comment) => (
            <CommentItem
              key={comment.id}
              comment={comment}
              currentUserId={currentUserId}
              isManager={isManager}
              disabled={disabled}
              depth={0}
              onReply={(content, parentId) => onAddComment(content, parentId)}
              onDelete={onDeleteComment}
            />
          ))}
        </div>
      )}
    </section>
  );
}
