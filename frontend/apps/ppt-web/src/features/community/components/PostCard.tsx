/**
 * PostCard Component
 *
 * Displays a community post with like, comment, and share actions.
 * Part of Story 42.2: Community Feed.
 */

import type { CommunityPost, PostType } from '@ppt/api-client';
import { useTranslation } from 'react-i18next';

interface PostCardProps {
  post: CommunityPost;
  onView?: (id: string) => void;
  onLike?: (id: string) => void;
  onComment?: (id: string) => void;
  onShare?: (id: string) => void;
  onEdit?: (id: string) => void;
  onDelete?: (id: string) => void;
  isCurrentUser?: boolean;
  isLiking?: boolean;
}

const postTypeIcons: Record<PostType, React.JSX.Element> = {
  text: (
    <svg
      className="w-4 h-4"
      fill="none"
      viewBox="0 0 24 24"
      stroke="currentColor"
      aria-hidden="true"
    >
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M4 6h16M4 12h16M4 18h7"
      />
    </svg>
  ),
  image: (
    <svg
      className="w-4 h-4"
      fill="none"
      viewBox="0 0 24 24"
      stroke="currentColor"
      aria-hidden="true"
    >
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z"
      />
    </svg>
  ),
  poll: (
    <svg
      className="w-4 h-4"
      fill="none"
      viewBox="0 0 24 24"
      stroke="currentColor"
      aria-hidden="true"
    >
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z"
      />
    </svg>
  ),
  event: (
    <svg
      className="w-4 h-4"
      fill="none"
      viewBox="0 0 24 24"
      stroke="currentColor"
      aria-hidden="true"
    >
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M8 7V3m8 4V3m-9 8h10M5 21h14a2 2 0 002-2V7a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z"
      />
    </svg>
  ),
  item: (
    <svg
      className="w-4 h-4"
      fill="none"
      viewBox="0 0 24 24"
      stroke="currentColor"
      aria-hidden="true"
    >
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M16 11V7a4 4 0 00-8 0v4M5 9h14l1 12H4L5 9z"
      />
    </svg>
  ),
};

function formatTimeAgo(dateString: string): string {
  const date = new Date(dateString);
  const now = new Date();
  const seconds = Math.floor((now.getTime() - date.getTime()) / 1000);

  if (seconds < 60) return 'just now';
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`;
  if (seconds < 86400) return `${Math.floor(seconds / 3600)}h ago`;
  if (seconds < 604800) return `${Math.floor(seconds / 86400)}d ago`;
  return date.toLocaleDateString();
}

export function PostCard({
  post,
  onView,
  onLike,
  onComment,
  onShare,
  onEdit,
  onDelete,
  isCurrentUser,
  isLiking,
}: PostCardProps) {
  const { t } = useTranslation();
  return (
    <div className="bg-white rounded-lg shadow hover:shadow-md transition-shadow">
      {/* Header */}
      <div className="p-4 flex items-start gap-3">
        {/* Author Avatar */}
        {post.authorAvatar ? (
          <img
            src={post.authorAvatar}
            alt={post.authorName}
            className="w-10 h-10 rounded-full object-cover"
          />
        ) : (
          <div className="w-10 h-10 rounded-full bg-gray-300 flex items-center justify-center">
            <span className="text-gray-600 font-medium">
              {post.authorName.charAt(0).toUpperCase()}
            </span>
          </div>
        )}

        {/* Author Info & Time */}
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2">
            <span className="font-medium text-gray-900">{post.authorName}</span>
            <span className="text-gray-400 text-xs flex items-center gap-1">
              {postTypeIcons[post.type]}
            </span>
          </div>
          <p className="text-xs text-gray-500">{formatTimeAgo(post.createdAt)}</p>
        </div>

        {/* Menu */}
        {isCurrentUser && (
          <div className="relative group">
            <button
              type="button"
              className="p-1 text-gray-400 hover:text-gray-600 rounded"
              aria-label={t('aria.postOptionsMenu')}
            >
              <svg
                className="w-5 h-5"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
                aria-hidden="true"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M12 5v.01M12 12v.01M12 19v.01M12 6a1 1 0 110-2 1 1 0 010 2zm0 7a1 1 0 110-2 1 1 0 010 2zm0 7a1 1 0 110-2 1 1 0 010 2z"
                />
              </svg>
            </button>
            <div className="absolute right-0 mt-1 w-32 bg-white rounded-md shadow-lg opacity-0 invisible group-hover:opacity-100 group-hover:visible transition-all z-10">
              <button
                type="button"
                onClick={() => onEdit?.(post.id)}
                className="block w-full px-4 py-2 text-left text-sm text-gray-700 hover:bg-gray-100"
              >
                Edit
              </button>
              <button
                type="button"
                onClick={() => onDelete?.(post.id)}
                className="block w-full px-4 py-2 text-left text-sm text-red-600 hover:bg-red-50"
              >
                Delete
              </button>
            </div>
          </div>
        )}

        {/* Pinned Badge */}
        {post.isPinned && (
          <span className="text-amber-500" title="Pinned">
            <svg className="w-4 h-4" fill="currentColor" viewBox="0 0 20 20" aria-hidden="true">
              <path d="M9.828.722a.5.5 0 01.354 0l7 3A.5.5 0 0117.5 4v1.5a.5.5 0 01-.5.5h-1v4.5a.5.5 0 01-.5.5H13v5.5a.5.5 0 01-.5.5h-5a.5.5 0 01-.5-.5V11H4.5a.5.5 0 01-.5-.5V6h-1a.5.5 0 01-.5-.5V4a.5.5 0 01.328-.472l7-3z" />
            </svg>
          </span>
        )}
      </div>

      {/* Content */}
      <div className="px-4 pb-3">
        <button
          type="button"
          className="text-gray-800 whitespace-pre-wrap cursor-pointer text-left w-full"
          onClick={() => onView?.(post.id)}
        >
          {post.content}
        </button>
      </div>

      {/* Media Gallery */}
      {post.mediaUrls.length > 0 && (
        <div className="px-4 pb-3">
          <div
            className={`grid gap-2 ${
              post.mediaUrls.length === 1
                ? 'grid-cols-1'
                : post.mediaUrls.length === 2
                  ? 'grid-cols-2'
                  : 'grid-cols-2'
            }`}
          >
            {post.mediaUrls.slice(0, 4).map((url, index) => (
              <div
                key={`${post.id}-media-${index}`}
                className={`relative ${
                  post.mediaUrls.length === 3 && index === 0 ? 'col-span-2' : ''
                }`}
              >
                <img
                  src={url}
                  alt={`Post media ${index + 1}`}
                  className="w-full h-48 object-cover rounded-lg"
                />
                {index === 3 && post.mediaUrls.length > 4 && (
                  <div className="absolute inset-0 bg-black bg-opacity-50 flex items-center justify-center rounded-lg">
                    <span className="text-white text-xl font-bold">
                      +{post.mediaUrls.length - 4}
                    </span>
                  </div>
                )}
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Stats */}
      <div className="px-4 py-2 flex items-center gap-4 text-sm text-gray-500 border-t border-b">
        <span>{post.likeCount} likes</span>
        <span>{post.commentCount} comments</span>
        <span>{post.shareCount} shares</span>
      </div>

      {/* Actions */}
      <div className="px-4 py-2 flex items-center">
        <button
          type="button"
          onClick={() => onLike?.(post.id)}
          disabled={isLiking}
          className={`flex-1 flex items-center justify-center gap-2 py-2 rounded-md transition-colors ${
            post.isLikedByUser
              ? 'text-blue-600 hover:bg-blue-50'
              : 'text-gray-600 hover:bg-gray-100'
          }`}
        >
          <svg
            className="w-5 h-5"
            fill={post.isLikedByUser ? 'currentColor' : 'none'}
            viewBox="0 0 24 24"
            stroke="currentColor"
            aria-hidden="true"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M14 10h4.764a2 2 0 011.789 2.894l-3.5 7A2 2 0 0115.263 21h-4.017c-.163 0-.326-.02-.485-.06L7 20m7-10V5a2 2 0 00-2-2h-.095c-.5 0-.905.405-.905.905 0 .714-.211 1.412-.608 2.006L7 11v9m7-10h-2M7 20H5a2 2 0 01-2-2v-6a2 2 0 012-2h2.5"
            />
          </svg>
          Like
        </button>
        <button
          type="button"
          onClick={() => onComment?.(post.id)}
          className="flex-1 flex items-center justify-center gap-2 py-2 text-gray-600 hover:bg-gray-100 rounded-md transition-colors"
        >
          <svg
            className="w-5 h-5"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
            aria-hidden="true"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M7 8h10M7 12h4m1 8l-4-4H5a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v8a2 2 0 01-2 2h-3l-4 4z"
            />
          </svg>
          Comment
        </button>
        <button
          type="button"
          onClick={() => onShare?.(post.id)}
          className="flex-1 flex items-center justify-center gap-2 py-2 text-gray-600 hover:bg-gray-100 rounded-md transition-colors"
        >
          <svg
            className="w-5 h-5"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
            aria-hidden="true"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M8.684 13.342C8.886 12.938 9 12.482 9 12c0-.482-.114-.938-.316-1.342m0 2.684a3 3 0 110-2.684m0 2.684l6.632 3.316m-6.632-6l6.632-3.316m0 0a3 3 0 105.367-2.684 3 3 0 00-5.367 2.684zm0 9.316a3 3 0 105.368 2.684 3 3 0 00-5.368-2.684z"
            />
          </svg>
          Share
        </button>
      </div>
    </div>
  );
}
