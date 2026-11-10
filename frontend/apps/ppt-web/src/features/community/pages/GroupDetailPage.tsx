/**
 * GroupDetailPage — `/community/groups/:groupId`.
 *
 * Loads the group (`GET /community/groups/{id}`), the caller's membership from
 * the building's group list (`GET /community/buildings/{id}/groups` — the
 * detail endpoint carries no membership), and the group's posts
 * (`GET /community/groups/{id}/posts`). Members can post and like; Join /
 * Leave and Delete (owner or manager) are real mutations.
 *
 * Not offered because the server has no endpoint for them: member list,
 * inviting members, editing the group, moderating members, comment threads.
 */

import {
  type CreateCommunityPostRequest,
  useAddPostReaction,
  useCreatePost,
  useDeleteGroup,
  useGroup,
  useGroupPosts,
  useGroups,
  useJoinGroup,
  useLeaveGroup,
} from '@ppt/api-client';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate, useParams } from 'react-router-dom';
import { ConfirmationDialog, useToast } from '../../../components';
import { useAuth } from '../../../contexts';
import { getErrorMessage } from '../../../lib/api';
import { isManagerRole } from '../../../routes/shared';
import { CreatePostForm } from '../components/CreatePostForm';
import { PostCard } from '../components/PostCard';
import { EmptyState, ErrorState, LoadingState } from '../components/StateViews';
import { withBuilding } from '../hooks/useCommunityBuilding';
import { formatDate } from '../utils/format';

export function GroupDetailPage() {
  const { groupId } = useParams<{ groupId: string }>();
  const { t, i18n } = useTranslation();
  const navigate = useNavigate();
  const { user } = useAuth();
  const { showToast } = useToast();
  const [confirmDelete, setConfirmDelete] = useState(false);

  const groupQuery = useGroup(groupId);
  const group = groupQuery.data;
  const siblings = useGroups(group?.buildingId);
  const postsQuery = useGroupPosts(groupId, { limit: 50 });
  const joinGroup = useJoinGroup();
  const leaveGroup = useLeaveGroup();
  const deleteGroup = useDeleteGroup();
  const createPost = useCreatePost();
  const react = useAddPostReaction();

  const backToGroups = () => navigate(withBuilding('/community/groups', group?.buildingId));

  if (groupQuery.isLoading) return <LoadingState />;

  if (groupQuery.error || !group || !groupId) {
    return (
      <div className="max-w-4xl mx-auto px-4 py-16 text-center">
        <p className="text-gray-500">{t('errors.groupNotFound')}</p>
        <button
          type="button"
          onClick={backToGroups}
          className="mt-4 text-sm text-blue-600 hover:text-blue-800"
        >
          {t('common.back')}
        </button>
      </div>
    );
  }

  const membership = siblings.data?.find((g) => g.id === group.id);
  const isMember = membership?.isMember ?? false;
  const role = membership?.memberRole ?? null;
  const isOwner = role === 'owner' || (!!user?.id && group.createdBy === user.id);
  const canDelete = isOwner || isManagerRole(user?.role);

  const handleJoin = () =>
    joinGroup.mutate(groupId, {
      onSuccess: () =>
        showToast({
          type: 'success',
          title: t('common.success'),
          message: t('community.groupJoined'),
        }),
      onError: (err) =>
        showToast({
          type: 'error',
          title: t('community.joinFailed'),
          message: getErrorMessage(err),
        }),
    });

  const handleLeave = () =>
    leaveGroup.mutate(groupId, {
      onSuccess: () =>
        showToast({
          type: 'success',
          title: t('common.success'),
          message: t('community.groupLeft'),
        }),
      onError: (err) =>
        showToast({
          type: 'error',
          title: t('community.leaveFailed'),
          message: getErrorMessage(err),
        }),
    });

  const handleDelete = () =>
    deleteGroup.mutate(groupId, {
      onSuccess: () => {
        setConfirmDelete(false);
        showToast({
          type: 'success',
          title: t('common.success'),
          message: t('community.groupDeleted'),
        });
        backToGroups();
      },
      onError: (err) => {
        setConfirmDelete(false);
        showToast({
          type: 'error',
          title: t('community.deleteFailed'),
          message: getErrorMessage(err),
        });
      },
    });

  const handleCreatePost = (gid: string, data: CreateCommunityPostRequest) =>
    createPost.mutateAsync(
      { groupId: gid, data },
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

  const posts = postsQuery.data ?? [];

  return (
    <div className="max-w-4xl mx-auto px-4 py-8 space-y-6">
      <button
        type="button"
        onClick={backToGroups}
        className="text-sm text-gray-600 hover:text-gray-900"
      >
        ← {t('community.common.backToGroups')}
      </button>

      <section className="bg-white rounded-lg shadow p-6">
        <div className="flex flex-wrap items-start justify-between gap-4">
          <div className="min-w-0">
            <h1 className="text-2xl font-bold text-gray-900">{group.name}</h1>
            <p className="mt-1 text-sm text-gray-500">
              {t(`community.groupTypes.${group.groupType}`)} ·{' '}
              {t(`community.groupCard.visibility.${group.visibility}`)} ·{' '}
              {t('community.groupDetail.members', { count: group.memberCount })} ·{' '}
              {t('community.hub.postCount', { count: group.postCount })}
            </p>
            {group.isOfficial && (
              <span className="mt-2 inline-block text-xs font-medium px-2 py-0.5 rounded-full bg-blue-100 text-blue-700">
                {t('community.common.official')}
              </span>
            )}
          </div>
          <div className="flex flex-wrap gap-2">
            {siblings.isLoading ? null : isMember ? (
              isOwner ? (
                <span
                  className="text-sm text-gray-500"
                  title={t('community.groupDetail.ownersCannotLeave')}
                >
                  {t('community.groupMembers.roles.owner')}
                </span>
              ) : (
                <button
                  type="button"
                  onClick={handleLeave}
                  disabled={leaveGroup.isPending}
                  className="px-4 py-2 text-sm border border-red-300 text-red-600 rounded-md hover:bg-red-50 disabled:opacity-50"
                >
                  {leaveGroup.isPending
                    ? t('community.groupDetail.leaving')
                    : t('community.groupDetail.leaveGroup')}
                </button>
              )
            ) : (
              <button
                type="button"
                onClick={handleJoin}
                disabled={joinGroup.isPending}
                className="px-4 py-2 text-sm bg-blue-600 text-white rounded-md hover:bg-blue-700 disabled:opacity-50"
              >
                {joinGroup.isPending
                  ? t('community.groupDetail.joining')
                  : t('community.groupDetail.joinGroup')}
              </button>
            )}
            {canDelete && (
              <button
                type="button"
                onClick={() => setConfirmDelete(true)}
                className="px-4 py-2 text-sm border border-gray-300 text-gray-700 rounded-md hover:bg-gray-50"
              >
                {t('community.groupDetail.deleteGroup')}
              </button>
            )}
          </div>
        </div>

        {group.description && (
          <div className="mt-4">
            <h2 className="text-sm font-semibold text-gray-900">
              {t('community.groupDetail.aboutTitle')}
            </h2>
            <p className="mt-1 text-sm text-gray-700 whitespace-pre-wrap">{group.description}</p>
          </div>
        )}
        {group.rules && (
          <div className="mt-4">
            <h2 className="text-sm font-semibold text-gray-900">{t('community.hub.groupRules')}</h2>
            <p className="mt-1 text-sm text-gray-700 whitespace-pre-wrap">{group.rules}</p>
          </div>
        )}
        <p className="mt-4 text-xs text-gray-500">
          {t('community.groupDetail.createdOn', {
            date: formatDate(group.createdAt, i18n.language),
          })}
        </p>
      </section>

      <section className="space-y-4" aria-labelledby="group-posts-heading">
        <h2 id="group-posts-heading" className="text-lg font-semibold text-gray-900">
          {t('community.hub.postsHeading')}
        </h2>
        {isMember ? (
          <CreatePostForm
            groups={[{ id: group.id, name: group.name }]}
            isSubmitting={createPost.isPending}
            onSubmit={handleCreatePost}
          />
        ) : (
          !siblings.isLoading && (
            <p className="text-sm text-gray-500">{t('community.hub.joinToPost')}</p>
          )
        )}
        {postsQuery.isLoading ? (
          <LoadingState />
        ) : postsQuery.error ? (
          <ErrorState message={t('community.hub.loadError')} onRetry={() => postsQuery.refetch()} />
        ) : posts.length === 0 ? (
          <EmptyState title={t('community.feed.emptyTitle')} body={t('community.feed.emptyBody')} />
        ) : (
          posts.map((post) => (
            <PostCard
              key={post.id}
              post={post}
              currentUserId={user?.id}
              onLike={handleLike}
              isLiking={react.isPending && react.variables?.postId === post.id}
            />
          ))
        )}
      </section>

      <ConfirmationDialog
        isOpen={confirmDelete}
        title={t('community.groupDetail.deleteGroup')}
        message={t('community.deleteGroupConfirm')}
        confirmLabel={t('community.common.delete')}
        cancelLabel={t('community.common.cancel')}
        variant="danger"
        isLoading={deleteGroup.isPending}
        onConfirm={handleDelete}
        onCancel={() => setConfirmDelete(false)}
      />
    </div>
  );
}
