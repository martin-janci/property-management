/**
 * FeedPage
 *
 * Main community feed page showing posts from all groups.
 * Part of Story 42.2: Community Feed.
 */

import type { CommunityPost, CreatePostRequest, ListPostsParams, PostType } from '@ppt/api-client';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { CreatePostForm } from '../components/CreatePostForm';
import { PostCard } from '../components/PostCard';

interface FeedPageProps {
  posts: CommunityPost[];
  total: number;
  isLoading?: boolean;
  currentUserId?: string;
  userAvatar?: string;
  userName: string;
  selectedGroupId?: string;
  isCreatingPost?: boolean;
  likingPostId?: string;
  onCreatePost: (data: CreatePostRequest) => void;
  onLikePost: (postId: string) => void;
  onViewPost: (postId: string) => void;
  onCommentPost: (postId: string) => void;
  onSharePost: (postId: string) => void;
  onEditPost: (postId: string) => void;
  onDeletePost: (postId: string) => void;
  onFilterChange: (params: ListPostsParams) => void;
  onLoadMore: () => void;
  hasMore?: boolean;
}

type FeedFilter = 'all' | 'my_groups' | 'building';

const feedFilters: { value: FeedFilter; key: string }[] = [
  { value: 'all', key: 'all' },
  { value: 'my_groups', key: 'myGroups' },
  { value: 'building', key: 'building' },
];

const postTypeFilters: { value: PostType | ''; key: string }[] = [
  { value: '', key: 'all' },
  { value: 'text', key: 'text' },
  { value: 'image', key: 'photos' },
  { value: 'poll', key: 'polls' },
  { value: 'event', key: 'events' },
  { value: 'item', key: 'items' },
];

export function FeedPage({
  posts,
  total,
  isLoading,
  currentUserId,
  userAvatar,
  userName,
  selectedGroupId,
  isCreatingPost,
  likingPostId,
  onCreatePost,
  onLikePost,
  onViewPost,
  onCommentPost,
  onSharePost,
  onEditPost,
  onDeletePost,
  onFilterChange,
  onLoadMore,
  hasMore,
}: FeedPageProps) {
  const { t } = useTranslation();
  const [feedFilter, setFeedFilter] = useState<FeedFilter>('all');
  const [typeFilter, setTypeFilter] = useState<PostType | ''>('');

  const handleFeedFilterChange = (filter: FeedFilter) => {
    setFeedFilter(filter);
    onFilterChange({
      feed: filter,
      type: typeFilter || undefined,
      page: 1,
    });
  };

  const handleTypeFilterChange = (type: PostType | '') => {
    setTypeFilter(type);
    onFilterChange({
      feed: feedFilter,
      type: type || undefined,
      page: 1,
    });
  };

  return (
    <div className="max-w-2xl mx-auto px-4 py-8">
      {/* Header */}
      <div className="mb-6">
        <h1 className="text-2xl font-bold text-gray-900">{t('community.feed.title')}</h1>
        <p className="mt-1 text-sm text-gray-500">{t('community.feed.subtitle')}</p>
      </div>

      {/* Create Post */}
      {selectedGroupId && (
        <div className="mb-6">
          <CreatePostForm
            groupId={selectedGroupId}
            userAvatar={userAvatar}
            userName={userName}
            isSubmitting={isCreatingPost}
            onSubmit={onCreatePost}
          />
        </div>
      )}

      {/* Filters */}
      <div className="bg-white rounded-lg shadow p-4 mb-6">
        <div className="flex flex-wrap items-center gap-4">
          {/* Feed Filter */}
          <div className="flex items-center gap-1 border rounded-lg p-1">
            {feedFilters.map((filter) => (
              <button
                key={filter.value}
                type="button"
                onClick={() => handleFeedFilterChange(filter.value)}
                className={`px-3 py-1 text-sm rounded-md transition-colors ${
                  feedFilter === filter.value
                    ? 'bg-blue-600 text-white'
                    : 'text-gray-600 hover:bg-gray-100'
                }`}
              >
                {t(`community.feed.filters.${filter.key}`)}
              </button>
            ))}
          </div>

          {/* Type Filter */}
          <select
            value={typeFilter}
            onChange={(e) => handleTypeFilterChange(e.target.value as PostType | '')}
            className="px-3 py-1 text-sm border border-gray-300 rounded-md focus:ring-blue-500 focus:border-blue-500"
          >
            {postTypeFilters.map((type) => (
              <option key={type.value} value={type.value}>
                {t(`community.feed.types.${type.key}`)}
              </option>
            ))}
          </select>

          <span className="ml-auto text-sm text-gray-500">
            {total} {total === 1 ? t('community.feed.post') : t('community.feed.posts')}
          </span>
        </div>
      </div>

      {/* Posts */}
      {isLoading && posts.length === 0 ? (
        <div className="space-y-4">
          {['skeleton-1', 'skeleton-2', 'skeleton-3'].map((key) => (
            <div key={key} className="bg-white rounded-lg shadow p-4 animate-pulse">
              <div className="flex items-center gap-3 mb-4">
                <div className="w-10 h-10 rounded-full bg-gray-200" />
                <div className="flex-1 space-y-2">
                  <div className="h-4 bg-gray-200 rounded w-1/4" />
                  <div className="h-3 bg-gray-200 rounded w-1/6" />
                </div>
              </div>
              <div className="space-y-2">
                <div className="h-4 bg-gray-200 rounded w-full" />
                <div className="h-4 bg-gray-200 rounded w-3/4" />
              </div>
            </div>
          ))}
        </div>
      ) : posts.length === 0 ? (
        <div className="bg-white rounded-lg shadow p-8 text-center">
          <svg
            className="mx-auto h-12 w-12 text-gray-400"
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
          <h3 className="mt-4 text-lg font-medium text-gray-900">
            {t('community.feed.emptyTitle')}
          </h3>
          <p className="mt-2 text-sm text-gray-500">{t('community.feed.emptyBody')}</p>
        </div>
      ) : (
        <div className="space-y-4">
          {posts.map((post) => (
            <PostCard
              key={post.id}
              post={post}
              isCurrentUser={post.authorId === currentUserId}
              isLiking={likingPostId === post.id}
              onView={onViewPost}
              onLike={onLikePost}
              onComment={onCommentPost}
              onShare={onSharePost}
              onEdit={onEditPost}
              onDelete={onDeletePost}
            />
          ))}

          {/* Load More */}
          {hasMore && (
            <div className="text-center pt-4">
              <button
                type="button"
                onClick={onLoadMore}
                disabled={isLoading}
                className="px-6 py-2 text-sm text-blue-600 hover:text-blue-800 font-medium disabled:opacity-50"
              >
                {isLoading ? t('community.common.loading') : t('community.common.loadMore')}
              </button>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
