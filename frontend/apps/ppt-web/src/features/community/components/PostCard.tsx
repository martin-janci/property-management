/**
 * PostCard — a single community post with its like action.
 *
 * The API exposes author ids only (no names / avatars), no "has liked" flag
 * and no comment listing, so the card shows a role label for the author, the
 * server counts, and a Like button; liking an already-liked post is a no-op on
 * the server (`ON CONFLICT DO NOTHING`).
 */

import type { CommunityPost } from '@ppt/api-client';
import { useTranslation } from 'react-i18next';
import { formatRelative, personLabel } from '../utils/format';

interface PostCardProps {
  post: CommunityPost;
  currentUserId?: string;
  groupName?: string;
  onOpenGroup?: (groupId: string) => void;
  onLike: (postId: string) => void;
  isLiking?: boolean;
}

export function PostCard({
  post,
  currentUserId,
  groupName,
  onOpenGroup,
  onLike,
  isLiking,
}: PostCardProps) {
  const { t, i18n } = useTranslation();
  const author = personLabel(post.authorId, currentUserId, t, post.isAnonymous);

  return (
    <article className="bg-white rounded-lg shadow p-4">
      <header className="flex items-center justify-between gap-2">
        <div className="min-w-0">
          <p className="text-sm font-medium text-gray-900">{author}</p>
          <p className="text-xs text-gray-500">
            {formatRelative(post.createdAt, i18n.language, t)}
            {groupName && (
              <>
                {' · '}
                {onOpenGroup ? (
                  <button
                    type="button"
                    onClick={() => onOpenGroup(post.groupId)}
                    className="text-blue-600 hover:text-blue-800"
                  >
                    {groupName}
                  </button>
                ) : (
                  groupName
                )}
              </>
            )}
          </p>
        </div>
        <div className="flex items-center gap-2 shrink-0">
          {post.postType !== 'text' && (
            <span className="text-xs px-2 py-0.5 rounded-full bg-gray-100 text-gray-700">
              {t(`community.postTypes.${post.postType}`)}
            </span>
          )}
          {post.isPinned && (
            <span className="text-xs px-2 py-0.5 rounded-full bg-yellow-100 text-yellow-800">
              {t('community.post.pinned')}
            </span>
          )}
        </div>
      </header>

      {post.title && <h3 className="mt-3 text-base font-semibold text-gray-900">{post.title}</h3>}
      <p className="mt-2 text-sm text-gray-800 whitespace-pre-wrap break-words">{post.content}</p>

      {post.mediaUrls.length > 0 && (
        <div className="mt-3 grid grid-cols-2 gap-2">
          {post.mediaUrls.map((url, index) => (
            <img
              key={url}
              src={url}
              alt={t('community.post.mediaAlt', { index: index + 1 })}
              className="w-full h-40 object-cover rounded-md"
            />
          ))}
        </div>
      )}

      <footer className="mt-4 pt-3 border-t border-gray-100 flex items-center justify-between text-sm text-gray-500">
        <span>
          {t('community.post.likes', { count: post.likeCount })} ·{' '}
          {t('community.post.comments', { count: post.commentCount })}
        </span>
        <button
          type="button"
          onClick={() => onLike(post.id)}
          disabled={isLiking || post.isLocked}
          className="px-3 py-1 rounded-md text-blue-600 hover:bg-blue-50 font-medium disabled:opacity-50"
        >
          {t('community.post.like')}
        </button>
      </footer>
    </article>
  );
}
