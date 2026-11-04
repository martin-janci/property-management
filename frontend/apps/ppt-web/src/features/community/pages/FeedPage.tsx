/**
 * FeedPage — `/community`.
 *
 * The newest posts from every group the caller belongs to in the selected
 * building, merged newest-first (`GET /community/buildings/{id}/groups`, then
 * `GET /community/groups/{id}/posts` per member group). The composer posts to
 * one of those groups; Like calls `POST /community/posts/{id}/reactions`.
 */

import {
  type CreateCommunityPostRequest,
  useAddPostReaction,
  useCreatePost,
  useFeedPosts,
  useGroups,
} from '@ppt/api-client';
import { useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { useToast } from '../../../components';
import { useAuth } from '../../../contexts';
import { getErrorMessage } from '../../../lib/api';
import { CommunityLayout } from '../components/CommunityLayout';
import { CreatePostForm } from '../components/CreatePostForm';
import { PostCard } from '../components/PostCard';
import { EmptyState, ErrorState, LoadingState } from '../components/StateViews';
import { useCommunityBuilding, withBuilding } from '../hooks/useCommunityBuilding';

export function FeedPage() {
  const { t } = useTranslation();
  const building = useCommunityBuilding();

  return (
    <CommunityLayout
      active="feed"
      title={t('community.feed.title')}
      subtitle={t('community.feed.subtitle')}
      building={building}
    >
      {(buildingId) => <FeedContent buildingId={buildingId} />}
    </CommunityLayout>
  );
}

function FeedContent({ buildingId }: { buildingId: string }) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { user } = useAuth();
  const { showToast } = useToast();
  const [groupFilter, setGroupFilter] = useState('');

  const groupsQuery = useGroups(buildingId);
  const memberGroups = useMemo(
    () => (groupsQuery.data ?? []).filter((g) => g.isMember),
    [groupsQuery.data]
  );
  const groupNames = useMemo(
    () => new Map(memberGroups.map((g) => [g.id, g.name])),
    [memberGroups]
  );
  const feed = useFeedPosts(memberGroups.map((g) => g.id));
  const createPost = useCreatePost();
  const react = useAddPostReaction();

  const openGroup = (groupId: string) =>
    navigate(withBuilding(`/community/groups/${groupId}`, buildingId));

  if (groupsQuery.isLoading) return <LoadingState />;
  if (groupsQuery.error) {
    return (
      <ErrorState message={t('community.hub.loadError')} onRetry={() => groupsQuery.refetch()} />
    );
  }
  if (memberGroups.length === 0) {
    return (
      <EmptyState
        title={t('community.hub.feedNoGroupsTitle')}
        body={t('community.hub.feedNoGroupsBody')}
        action={
          <button
            type="button"
            onClick={() => navigate(withBuilding('/community/groups', buildingId))}
            className="px-4 py-2 text-sm font-medium bg-blue-600 text-white rounded-md hover:bg-blue-700"
          >
            {t('community.hub.browseGroups')}
          </button>
        }
      />
    );
  }

  const posts = groupFilter ? feed.posts.filter((p) => p.groupId === groupFilter) : feed.posts;

  const handleCreate = (groupId: string, data: CreateCommunityPostRequest) =>
    createPost.mutateAsync(
      { groupId, data },
      {
        onSuccess: () =>
          showToast({
            type: 'success',
            title: t('common.success'),
            message: t('community.hub.postCreated'),
          }),
        onError: (err) =>
          showToast({
            type: 'error',
            title: t('community.hub.postFailed'),
            message: getErrorMessage(err),
          }),
      }
    );

  const handleLike = (postId: string) =>
    react.mutate(
      { postId },
      {
        onError: (err) =>
          showToast({
            type: 'error',
            title: t('community.hub.likeFailed'),
            message: getErrorMessage(err),
          }),
      }
    );

  return (
    <div className="max-w-2xl mx-auto space-y-6">
      <CreatePostForm
        groups={memberGroups.map((g) => ({ id: g.id, name: g.name }))}
        isSubmitting={createPost.isPending}
        onSubmit={handleCreate}
      />

      {memberGroups.length > 1 && (
        <div className="flex items-center justify-between gap-3">
          <label className="flex items-center gap-2 text-sm text-gray-600">
            <span>{t('community.hub.filterByGroup')}</span>
            <select
              value={groupFilter}
              onChange={(e) => setGroupFilter(e.target.value)}
              className="px-2 py-1 text-sm border border-gray-300 rounded-md"
            >
              <option value="">{t('community.feed.filters.myGroups')}</option>
              {memberGroups.map((g) => (
                <option key={g.id} value={g.id}>
                  {g.name}
                </option>
              ))}
            </select>
          </label>
          <span className="text-sm text-gray-500">
            {posts.length}{' '}
            {posts.length === 1 ? t('community.feed.post') : t('community.feed.posts')}
          </span>
        </div>
      )}

      {feed.isLoading && feed.posts.length === 0 ? (
        <LoadingState />
      ) : feed.error && feed.posts.length === 0 ? (
        <ErrorState message={t('community.hub.loadError')} onRetry={() => feed.refetch()} />
      ) : posts.length === 0 ? (
        <EmptyState title={t('community.feed.emptyTitle')} body={t('community.feed.emptyBody')} />
      ) : (
        <div className="space-y-4">
          {posts.map((post) => (
            <PostCard
              key={post.id}
              post={post}
              currentUserId={user?.id}
              groupName={groupNames.get(post.groupId)}
              onOpenGroup={openGroup}
              onLike={handleLike}
              isLiking={react.isPending && react.variables?.postId === post.id}
            />
          ))}
        </div>
      )}
    </div>
  );
}
