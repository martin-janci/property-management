/**
 * CommentList Component
 *
 * Displays comments with nested replies and like functionality.
 * Part of Story 42.2: Community Feed.
 */

import type { PostComment } from '@ppt/api-client';
import type { TFunction } from 'i18next';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';

interface CommentListProps {
  comments: PostComment[];
  currentUserId?: string;
  onLike?: (commentId: string) => void;
  onReply?: (commentId: string, content: string) => void;
  onEdit?: (commentId: string) => void;
  onDelete?: (commentId: string) => void;
  isLoading?: boolean;
}

interface CommentItemProps {
  comment: PostComment;
  currentUserId?: string;
  isNested?: boolean;
  onLike?: (commentId: string) => void;
  onReply?: (commentId: string, content: string) => void;
  onEdit?: (commentId: string) => void;
  onDelete?: (commentId: string) => void;
}

function formatTimeAgo(dateString: string, t: TFunction): string {
  const date = new Date(dateString);
  const now = new Date();
  const seconds = Math.floor((now.getTime() - date.getTime()) / 1000);

  if (seconds < 60) return t('community.comments.justNow');
  if (seconds < 3600) return t('community.comments.minutes', { count: Math.floor(seconds / 60) });
  if (seconds < 86400) return t('community.comments.hours', { count: Math.floor(seconds / 3600) });
  if (seconds < 604800) return t('community.comments.days', { count: Math.floor(seconds / 86400) });
  return date.toLocaleDateString();
}

function CommentItem({
  comment,
  currentUserId,
  isNested = false,
  onLike,
  onReply,
  onEdit,
  onDelete,
}: CommentItemProps) {
  const { t } = useTranslation();
  const [showReplyForm, setShowReplyForm] = useState(false);
  const [replyContent, setReplyContent] = useState('');
  const isCurrentUser = comment.authorId === currentUserId;

  const handleReplySubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (replyContent.trim()) {
      onReply?.(comment.id, replyContent.trim());
      setReplyContent('');
      setShowReplyForm(false);
    }
  };

  return (
    <div className={`flex gap-3 ${isNested ? 'ml-8' : ''}`}>
      {/* Avatar */}
      {comment.authorAvatar ? (
        <img
          src={comment.authorAvatar}
          alt={comment.authorName}
          className="w-8 h-8 rounded-full object-cover flex-shrink-0"
        />
      ) : (
        <div className="w-8 h-8 rounded-full bg-gray-300 flex items-center justify-center flex-shrink-0">
          <span className="text-gray-600 text-xs font-medium">
            {comment.authorName.charAt(0).toUpperCase()}
          </span>
        </div>
      )}

      <div className="flex-1 min-w-0">
        {/* Comment Bubble */}
        <div className="bg-gray-100 rounded-lg px-3 py-2">
          <div className="flex items-center gap-2">
            <span className="font-medium text-sm text-gray-900">{comment.authorName}</span>
            <span className="text-xs text-gray-500">{formatTimeAgo(comment.createdAt, t)}</span>
          </div>
          <p className="text-sm text-gray-800 mt-1">{comment.content}</p>
        </div>

        {/* Actions */}
        <div className="flex items-center gap-3 mt-1 text-xs">
          <button
            type="button"
            onClick={() => onLike?.(comment.id)}
            className={`font-medium ${
              comment.isLikedByUser ? 'text-blue-600' : 'text-gray-500 hover:text-gray-700'
            }`}
          >
            {comment.isLikedByUser ? t('community.comments.liked') : t('community.comments.like')}
            {comment.likeCount > 0 && ` (${comment.likeCount})`}
          </button>
          {!isNested && (
            <button
              type="button"
              onClick={() => setShowReplyForm(!showReplyForm)}
              className="text-gray-500 hover:text-gray-700 font-medium"
            >
              {t('community.comments.reply')}
            </button>
          )}
          {isCurrentUser && (
            <>
              <button
                type="button"
                onClick={() => onEdit?.(comment.id)}
                className="text-gray-500 hover:text-gray-700"
              >
                {t('community.common.edit')}
              </button>
              <button
                type="button"
                onClick={() => onDelete?.(comment.id)}
                className="text-red-500 hover:text-red-700"
              >
                {t('community.common.delete')}
              </button>
            </>
          )}
        </div>

        {/* Reply Form */}
        {showReplyForm && (
          <form onSubmit={handleReplySubmit} className="flex gap-2 mt-2">
            <input
              type="text"
              value={replyContent}
              onChange={(e) => setReplyContent(e.target.value)}
              placeholder={t('community.comments.replyPlaceholder')}
              className="flex-1 px-3 py-1 text-sm border border-gray-300 rounded-full focus:ring-blue-500 focus:border-blue-500"
            />
            <button
              type="submit"
              disabled={!replyContent.trim()}
              className="px-3 py-1 text-sm bg-blue-600 text-white rounded-full hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {t('community.comments.reply')}
            </button>
          </form>
        )}

        {/* Nested Replies */}
        {comment.replies && comment.replies.length > 0 && (
          <div className="mt-3 space-y-3">
            {comment.replies.map((reply) => (
              <CommentItem
                key={reply.id}
                comment={reply}
                currentUserId={currentUserId}
                isNested
                onLike={onLike}
                onEdit={onEdit}
                onDelete={onDelete}
              />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

export function CommentList({
  comments,
  currentUserId,
  onLike,
  onReply,
  onEdit,
  onDelete,
  isLoading,
}: CommentListProps) {
  const { t } = useTranslation();
  if (isLoading) {
    const skeletonKeys = ['skeleton-1', 'skeleton-2', 'skeleton-3'];
    return (
      <div className="space-y-4">
        {skeletonKeys.map((key) => (
          <div key={key} className="flex gap-3 animate-pulse">
            <div className="w-8 h-8 rounded-full bg-gray-200" />
            <div className="flex-1 space-y-2">
              <div className="h-16 bg-gray-200 rounded-lg" />
              <div className="h-3 bg-gray-200 rounded w-24" />
            </div>
          </div>
        ))}
      </div>
    );
  }

  if (comments.length === 0) {
    return (
      <div className="text-center py-6">
        <p className="text-sm text-gray-500">{t('community.comments.empty')}</p>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      {comments.map((comment) => (
        <CommentItem
          key={comment.id}
          comment={comment}
          currentUserId={currentUserId}
          onLike={onLike}
          onReply={onReply}
          onEdit={onEdit}
          onDelete={onDelete}
        />
      ))}
    </div>
  );
}
