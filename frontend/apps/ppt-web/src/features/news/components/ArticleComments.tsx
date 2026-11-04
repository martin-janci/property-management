/**
 * Article Comments Component (Epic 59, Story 59.3)
 * Displays and manages comments on news articles
 */

import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { CommentWithAuthor } from '../types';

interface ArticleCommentsProps {
  comments: CommentWithAuthor[];
  onAddComment: (content: string, parentId?: string) => Promise<void>;
  onEditComment: (commentId: string, content: string) => Promise<void>;
  onDeleteComment: (commentId: string) => Promise<void>;
  onLoadReplies?: (commentId: string) => Promise<void>;
  currentUserId?: string;
  disabled?: boolean;
}

interface CommentItemProps {
  comment: CommentWithAuthor;
  currentUserId?: string;
  onReply: (content: string) => void;
  onEdit: (content: string) => void;
  onDelete: () => void;
  onShowReplies?: () => void;
  disabled?: boolean;
}

function CommentItem({
  comment,
  currentUserId,
  onReply,
  onEdit,
  onDelete,
  onShowReplies,
  disabled,
}: CommentItemProps) {
  const { t, i18n } = useTranslation();
  const [isReplying, setIsReplying] = useState(false);
  const [isEditing, setIsEditing] = useState(false);
  const [replyContent, setReplyContent] = useState('');
  const [editContent, setEditContent] = useState(comment.content);

  const isOwnComment = currentUserId === comment.userId;
  const isDeleted = comment.deletedAt !== null;

  const handleSubmitReply = () => {
    if (!replyContent.trim()) return;
    onReply(replyContent);
    setReplyContent('');
    setIsReplying(false);
  };

  const handleSubmitEdit = () => {
    if (!editContent.trim() || editContent === comment.content) {
      setIsEditing(false);
      return;
    }
    onEdit(editContent);
    setIsEditing(false);
  };

  if (isDeleted) {
    return <div className="text-sm text-gray-400 italic py-2">{t('news.comments.deleted')}</div>;
  }

  return (
    <div className="py-3">
      <div className="flex items-start gap-3">
        <div className="flex-shrink-0">
          {comment.authorAvatarUrl ? (
            <img
              src={comment.authorAvatarUrl}
              alt={comment.authorName}
              className="w-8 h-8 rounded-full"
            />
          ) : (
            <div className="w-8 h-8 rounded-full bg-gray-200 flex items-center justify-center">
              <span className="text-xs font-semibold text-gray-600">
                {comment.authorName.charAt(0).toUpperCase()}
              </span>
            </div>
          )}
        </div>

        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2">
            <span className="font-medium text-sm text-gray-900">{comment.authorName}</span>
            <span className="text-xs text-gray-500">
              {new Date(comment.createdAt).toLocaleDateString(i18n.language)}
            </span>
          </div>

          {isEditing ? (
            <div className="mt-2">
              <textarea
                value={editContent}
                onChange={(e) => setEditContent(e.target.value)}
                className="w-full px-3 py-2 border border-gray-300 rounded-md text-sm"
                rows={3}
                maxLength={2000}
              />
              <div className="mt-2 flex gap-2">
                <button
                  type="button"
                  onClick={handleSubmitEdit}
                  className="px-3 py-1 bg-blue-600 text-white text-sm rounded hover:bg-blue-700"
                >
                  {t('common.save')}
                </button>
                <button
                  type="button"
                  onClick={() => {
                    setEditContent(comment.content);
                    setIsEditing(false);
                  }}
                  className="px-3 py-1 bg-gray-200 text-gray-700 text-sm rounded hover:bg-gray-300"
                >
                  {t('common.cancel')}
                </button>
              </div>
            </div>
          ) : (
            <p className="mt-1 text-sm text-gray-700 whitespace-pre-wrap">{comment.content}</p>
          )}

          <div className="mt-2 flex items-center gap-4 text-xs">
            {!disabled && (
              <button
                type="button"
                onClick={() => setIsReplying(!isReplying)}
                className="text-blue-600 hover:text-blue-800"
              >
                {t('news.comments.reply')}
              </button>
            )}
            {isOwnComment && !disabled && (
              <>
                <button
                  type="button"
                  onClick={() => setIsEditing(true)}
                  className="text-gray-600 hover:text-gray-800"
                >
                  {t('common.edit')}
                </button>
                <button
                  type="button"
                  onClick={onDelete}
                  className="text-red-600 hover:text-red-800"
                >
                  {t('common.delete')}
                </button>
              </>
            )}
            {comment.replyCount > 0 && onShowReplies && (
              <button
                type="button"
                onClick={onShowReplies}
                className="text-blue-600 hover:text-blue-800"
              >
                {t('news.comments.replyCount', { count: comment.replyCount })}
              </button>
            )}
          </div>

          {isReplying && (
            <div className="mt-3">
              <textarea
                value={replyContent}
                onChange={(e) => setReplyContent(e.target.value)}
                placeholder={t('news.comments.replyPlaceholder')}
                className="w-full px-3 py-2 border border-gray-300 rounded-md text-sm"
                rows={2}
                maxLength={2000}
              />
              <div className="mt-2 flex gap-2">
                <button
                  type="button"
                  onClick={handleSubmitReply}
                  disabled={!replyContent.trim()}
                  className="px-3 py-1 bg-blue-600 text-white text-sm rounded hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed"
                >
                  {t('news.comments.reply')}
                </button>
                <button
                  type="button"
                  onClick={() => {
                    setReplyContent('');
                    setIsReplying(false);
                  }}
                  className="px-3 py-1 bg-gray-200 text-gray-700 text-sm rounded hover:bg-gray-300"
                >
                  {t('common.cancel')}
                </button>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

export function ArticleComments({
  comments,
  onAddComment,
  onEditComment,
  onDeleteComment,
  onLoadReplies,
  currentUserId,
  disabled = false,
}: ArticleCommentsProps) {
  const { t } = useTranslation();
  const [newComment, setNewComment] = useState('');
  const [isSubmitting, setIsSubmitting] = useState(false);

  const handleSubmitComment = async () => {
    if (!newComment.trim() || isSubmitting) return;

    setIsSubmitting(true);
    try {
      await onAddComment(newComment);
      setNewComment('');
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div className="space-y-4">
      <h3 className="text-lg font-semibold text-gray-900">
        {t('news.comments.title', { count: comments.length })}
      </h3>

      {!disabled && (
        <div>
          <textarea
            value={newComment}
            onChange={(e) => setNewComment(e.target.value)}
            placeholder={t('news.comments.placeholder')}
            className="w-full px-4 py-3 border border-gray-300 rounded-lg text-sm focus:ring-2 focus:ring-blue-500 focus:border-transparent"
            rows={3}
            maxLength={2000}
            disabled={isSubmitting}
          />
          <div className="mt-2 flex justify-between items-center">
            <span className="text-xs text-gray-500">
              {t('news.comments.charCount', { length: newComment.length, max: 2000 })}
            </span>
            <button
              type="button"
              onClick={handleSubmitComment}
              disabled={!newComment.trim() || isSubmitting}
              className="px-4 py-2 bg-blue-600 text-white text-sm font-medium rounded-lg hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {isSubmitting ? t('news.comments.posting') : t('news.comments.post')}
            </button>
          </div>
        </div>
      )}

      <div className="space-y-1 divide-y divide-gray-200">
        {comments.map((comment) => (
          <CommentItem
            key={comment.id}
            comment={comment}
            currentUserId={currentUserId}
            onReply={(content) => onAddComment(content, comment.id)}
            onEdit={(content) => onEditComment(comment.id, content)}
            onDelete={() => onDeleteComment(comment.id)}
            onShowReplies={
              comment.replyCount > 0 && onLoadReplies ? () => onLoadReplies(comment.id) : undefined
            }
            disabled={disabled}
          />
        ))}
      </div>

      {comments.length === 0 && (
        <p className="text-center text-gray-500 py-8">{t('news.comments.empty')}</p>
      )}
    </div>
  );
}
