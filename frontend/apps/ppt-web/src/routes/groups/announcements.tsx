/**
 * Announcements route group (UC-06).
 *
 * Owns the announcement route-wrapper components and the `<Route>` table
 * fragment. Extracted from App.tsx to isolate announcement work.
 */
import type {
  AnnouncementWithDetails,
  CreateAnnouncementRequest,
  ListAnnouncementsParams,
  UpdateAnnouncementRequest,
} from '@ppt/api-client';
import {
  useAcknowledgeAnnouncement,
  useAnnouncement,
  useAnnouncementAcknowledgmentStats,
  useAnnouncementComments,
  useAnnouncements,
  useArchiveAnnouncement,
  useCreateAnnouncement,
  useCreateAnnouncementComment,
  useDeleteAnnouncement,
  useDeleteAnnouncementComment,
  useMarkReadAnnouncement,
  usePinAnnouncement,
  usePinnedAnnouncements,
  usePublishAnnouncement,
  useUpdateAnnouncement,
} from '@ppt/api-client';
import { useCallback, useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Route, useNavigate, useParams } from 'react-router-dom';
import { useToast } from '../../components';
import { useAuth } from '../../contexts';
import {
  AnnouncementsPage,
  CreateAnnouncementPage,
  EditAnnouncementPage,
  ViewAnnouncementPage,
} from '../lazyRoutes';
import { isManagerRole } from '../shared';

/**
 * Route wrapper for announcements list page (UC-06, gap-79-1).
 *
 * Wired to @ppt/api-client standalone hooks:
 *   useAnnouncements — TanStack Query list with filter params
 *   useDeleteAnnouncement / usePublishAnnouncement / useArchiveAnnouncement / usePinAnnouncement
 *
 * AnnouncementSummary from @ppt/api-client matches the page's prop type
 * directly — no type mapping required.
 */
function AnnouncementsPageRoute() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { showToast } = useToast();
  const [listParams, setListParams] = useState<ListAnnouncementsParams>({
    page: 1,
    pageSize: 10,
  });

  const { data, isLoading, error, refetch } = useAnnouncements(listParams);
  // Story 6.4 — separate query for the sticky pinned band; immune to list filters.
  // Routes through the shared hook (#486 / #516) so the URL + staleTime + pageSize
  // can't drift between this callsite and other consumers (mobile).
  const { data: pinnedData } = usePinnedAnnouncements();
  const deleteAnnouncement = useDeleteAnnouncement();
  const publishAnnouncement = usePublishAnnouncement();
  const archiveAnnouncement = useArchiveAnnouncement();
  const pinAnnouncement = usePinAnnouncement();

  useEffect(() => {
    if (error) {
      showToast({
        type: 'error',
        title: t('announcements.failedToLoad', { defaultValue: 'Failed to load announcements' }),
        message: error instanceof Error ? error.message : t('auth.unexpectedError'),
      });
    }
  }, [error, showToast, t]);

  const announcements = data?.items ?? [];
  const total = data?.total ?? 0;
  const pinnedAnnouncements = pinnedData?.items ?? [];

  const handleDelete = async (id: string) => {
    try {
      await deleteAnnouncement.mutateAsync(id);
      showToast({
        type: 'success',
        title: t('announcements.deleted', { defaultValue: 'Deleted' }),
        message: '',
      });
    } catch (err) {
      showToast({
        type: 'error',
        title: t('announcements.deleteFailed', { defaultValue: 'Delete failed' }),
        message: err instanceof Error ? err.message : '',
      });
    }
  };

  const handlePublish = async (id: string) => {
    try {
      await publishAnnouncement.mutateAsync(id);
      showToast({
        type: 'success',
        title: t('announcements.published', { defaultValue: 'Published' }),
        message: '',
      });
    } catch (err) {
      showToast({
        type: 'error',
        title: t('announcements.publishFailed', { defaultValue: 'Publish failed' }),
        message: err instanceof Error ? err.message : '',
      });
    }
  };

  const handleArchive = async (id: string) => {
    try {
      await archiveAnnouncement.mutateAsync(id);
      showToast({
        type: 'info',
        title: t('announcements.archived', { defaultValue: 'Archived' }),
        message: '',
      });
    } catch (err) {
      showToast({
        type: 'error',
        title: t('announcements.archiveFailed', { defaultValue: 'Archive failed' }),
        message: err instanceof Error ? err.message : '',
      });
    }
  };

  const handleAnnouncementsFilterChange = useCallback(
    (params: Partial<ListAnnouncementsParams>) => setListParams((prev) => ({ ...prev, ...params })),
    []
  );

  const handlePin = async (id: string, pinned: boolean) => {
    try {
      await pinAnnouncement.mutateAsync({ id, pinned });
      showToast({
        type: 'info',
        title: pinned
          ? t('announcements.pinned', { defaultValue: 'Pinned' })
          : t('announcements.unpinned', { defaultValue: 'Unpinned' }),
        message: '',
      });
    } catch (err) {
      showToast({
        type: 'error',
        title: t('announcements.pinFailed', { defaultValue: 'Pin action failed' }),
        message: err instanceof Error ? err.message : '',
      });
    }
  };

  return (
    <AnnouncementsPage
      announcements={announcements}
      total={total}
      isLoading={isLoading}
      isError={!!error}
      onRetry={() => {
        void refetch();
      }}
      pinnedAnnouncements={pinnedAnnouncements}
      onNavigateToCreate={() => navigate('/announcements/new')}
      onNavigateToView={(id) => navigate(`/announcements/${id}`)}
      onNavigateToEdit={(id) => navigate(`/announcements/${id}/edit`)}
      onDelete={handleDelete}
      onPublish={handlePublish}
      onArchive={handleArchive}
      onPin={handlePin}
      onFilterChange={handleAnnouncementsFilterChange}
    />
  );
}

/**
 * Route wrapper for the create-announcement page (UC-06, gap-79-1).
 *
 * Wired to `useCreateAnnouncement` (POST /api/v1/announcements). On success the
 * server returns the new id; we navigate to its detail view.
 */
function CreateAnnouncementPageRoute() {
  const navigate = useNavigate();
  const { showToast } = useToast();
  const { t } = useTranslation();
  const createAnnouncement = useCreateAnnouncement();

  return (
    <CreateAnnouncementPage
      buildings={[]}
      units={[]}
      roles={[]}
      isLoading={createAnnouncement.isPending}
      onSubmit={async (data: CreateAnnouncementRequest) => {
        try {
          const created = await createAnnouncement.mutateAsync(data);
          showToast({
            type: 'success',
            title: t('announcements.created', { defaultValue: 'Created' }),
            message: t('announcements.createdMessage', { defaultValue: 'Announcement created' }),
          });
          navigate(created?.id ? `/announcements/${created.id}` : '/announcements');
        } catch (err) {
          showToast({
            type: 'error',
            title: t('announcements.createFailed', {
              defaultValue: 'Failed to create announcement',
            }),
            message: err instanceof Error ? err.message : '',
          });
        }
      }}
      onCancel={() => navigate('/announcements')}
    />
  );
}

/**
 * Inner component for announcement detail — all hooks called unconditionally.
 *
 * Wired to @ppt/api-client standalone hooks:
 *   useAnnouncement                    — TanStack Query detail (announcement + attachments)
 *   useMarkReadAnnouncement            — POST /api/v1/announcements/:id/read
 *   useAcknowledgeAnnouncement         — POST /api/v1/announcements/:id/acknowledge
 *   useAnnouncementAcknowledgmentStats — GET  /api/v1/announcements/:id/acknowledgments
 *   useAnnouncementComments            — GET  /api/v1/announcements/:id/comments (Story 6.3)
 *   useCreateAnnouncementComment       — POST /api/v1/announcements/:id/comments (Story 6.3)
 *   useDeleteAnnouncementComment       — DELETE /api/v1/announcements/:id/comments/:cid (Story 6.3)
 *   usePublishAnnouncement / useArchiveAnnouncement / usePinAnnouncement / useDeleteAnnouncement
 */
export function ViewAnnouncementPageInner({ announcementId }: { announcementId: string }) {
  const navigate = useNavigate();
  const { showToast } = useToast();
  const { t } = useTranslation();
  const { user } = useAuth();

  // Guard ref: fire mark-read exactly once per announcement view, even in StrictMode.
  const autoMarkReadFired = useRef(false);

  const { data, isLoading, error } = useAnnouncement(announcementId);
  const markRead = useMarkReadAnnouncement();
  const acknowledge = useAcknowledgeAnnouncement();
  const publishAnnouncement = usePublishAnnouncement();
  const archiveAnnouncement = useArchiveAnnouncement();
  const pinAnnouncement = usePinAnnouncement();
  const deleteAnnouncement = useDeleteAnnouncement();

  // Fetch ack stats only when the announcement requires acknowledgment (manager view)
  const { data: ackStatsData } = useAnnouncementAcknowledgmentStats(
    announcementId,
    !!data?.announcement?.acknowledgmentRequired
  );

  // Story 6.3: Comments — only fetch when commentsEnabled
  const commentsEnabled = !!data?.announcement?.commentsEnabled;
  const { data: commentsData, isLoading: commentsLoading } = useAnnouncementComments(
    announcementId,
    undefined,
    commentsEnabled
  );
  const createComment = useCreateAnnouncementComment();
  const deleteComment = useDeleteAnnouncementComment();

  useEffect(() => {
    if (error) {
      showToast({
        type: 'error',
        title: t('announcements.failedToLoad', { defaultValue: 'Failed to load announcement' }),
        message: error instanceof Error ? error.message : t('auth.unexpectedError'),
      });
    }
  }, [error, showToast, t]);

  // Story 6.2: auto-fire read-receipt when announcement loads.
  // Fire-and-forget — the user shouldn't have to click "Mark as Read".
  // The mutation is idempotent (upsert on server), so double-firing is safe.
  // Retry is handled by TanStack Mutation default (3 attempts, exponential back-off).
  // markRead.mutate is intentionally excluded from deps — including a new mutate
  // instance on each render would cause an infinite loop; the ref guard ensures
  // the call fires exactly once per viewed announcement.
  // biome-ignore lint/correctness/useExhaustiveDependencies: markRead.mutate excluded intentionally (see comment)
  useEffect(() => {
    if (data?.announcement && !autoMarkReadFired.current) {
      autoMarkReadFired.current = true;
      markRead.mutate(announcementId);
    }
  }, [data?.announcement, announcementId]);

  const announcement = data?.announcement;
  const attachments = data?.attachments ?? [];

  const handleMarkRead = async () => {
    try {
      await markRead.mutateAsync(announcementId);
      showToast({
        type: 'success',
        title: t('announcements.markedRead', { defaultValue: 'Marked as read' }),
        message: '',
      });
    } catch (err) {
      showToast({
        type: 'error',
        title: t('announcements.markReadFailed', { defaultValue: 'Failed to mark as read' }),
        message: err instanceof Error ? err.message : '',
      });
    }
  };

  const handleAcknowledge = async () => {
    try {
      await acknowledge.mutateAsync(announcementId);
      showToast({
        type: 'success',
        title: t('announcements.acknowledged', { defaultValue: 'Acknowledged' }),
        message: '',
      });
    } catch (err) {
      showToast({
        type: 'error',
        title: t('announcements.acknowledgeFailed', { defaultValue: 'Failed to acknowledge' }),
        message: err instanceof Error ? err.message : '',
      });
    }
  };

  const handlePublish = async () => {
    try {
      await publishAnnouncement.mutateAsync(announcementId);
      showToast({
        type: 'success',
        title: t('announcements.published', { defaultValue: 'Published' }),
        message: '',
      });
    } catch (err) {
      showToast({
        type: 'error',
        title: t('announcements.publishFailed', { defaultValue: 'Publish failed' }),
        message: err instanceof Error ? err.message : '',
      });
    }
  };

  const handleArchive = async () => {
    try {
      await archiveAnnouncement.mutateAsync(announcementId);
      showToast({
        type: 'info',
        title: t('announcements.archived', { defaultValue: 'Archived' }),
        message: '',
      });
    } catch (err) {
      showToast({
        type: 'error',
        title: t('announcements.archiveFailed', { defaultValue: 'Archive failed' }),
        message: err instanceof Error ? err.message : '',
      });
    }
  };

  const handlePin = async (pinned: boolean) => {
    try {
      await pinAnnouncement.mutateAsync({ id: announcementId, pinned });
      showToast({
        type: 'info',
        title: pinned
          ? t('announcements.pinned', { defaultValue: 'Pinned' })
          : t('announcements.unpinned', { defaultValue: 'Unpinned' }),
        message: '',
      });
    } catch (err) {
      showToast({
        type: 'error',
        title: t('announcements.pinFailed', { defaultValue: 'Pin action failed' }),
        message: err instanceof Error ? err.message : '',
      });
    }
  };

  const handleDelete = async () => {
    try {
      await deleteAnnouncement.mutateAsync(announcementId);
      showToast({
        type: 'success',
        title: t('announcements.deleted', { defaultValue: 'Deleted' }),
        message: '',
      });
      navigate('/announcements');
    } catch (err) {
      showToast({
        type: 'error',
        title: t('announcements.deleteFailed', { defaultValue: 'Delete failed' }),
        message: err instanceof Error ? err.message : '',
      });
    }
  };

  // Story 6.3: Comment handlers
  const optimisticAuthorName =
    [user?.firstName, user?.lastName].filter(Boolean).join(' ') || user?.email;
  const handleAddComment = async (content: string, parentId?: string) => {
    try {
      await createComment.mutateAsync({
        announcementId,
        content,
        parentId,
        // Optimistic add: placeholder shows immediately, reconciled on settle.
        optimisticAuthorName,
        optimisticUserId: user?.id,
      });
      showToast({
        type: 'success',
        title: t('announcements.commentPosted', { defaultValue: 'Comment posted' }),
        message: '',
      });
    } catch (err) {
      showToast({
        type: 'error',
        title: t('announcements.commentFailed', { defaultValue: 'Failed to post comment' }),
        message: err instanceof Error ? err.message : '',
      });
    }
  };

  const handleDeleteComment = async (commentId: string, reason?: string) => {
    try {
      await deleteComment.mutateAsync({ announcementId, commentId, reason });
      showToast({
        type: 'info',
        title: t('announcements.commentDeleted', { defaultValue: 'Comment deleted' }),
        message: '',
      });
    } catch (err) {
      showToast({
        type: 'error',
        title: t('announcements.commentDeleteFailed', { defaultValue: 'Failed to delete comment' }),
        message: err instanceof Error ? err.message : '',
      });
    }
  };

  const EMPTY_ANNOUNCEMENT: AnnouncementWithDetails = {
    id: announcementId,
    organizationId: '',
    authorId: '',
    authorName: '',
    title: '',
    content: '',
    status: 'draft',
    targetType: 'all',
    targetIds: [],
    pinned: false,
    acknowledgmentRequired: false,
    commentsEnabled: false,
    readCount: 0,
    acknowledgedCount: 0,
    commentCount: 0,
    attachmentCount: 0,
    createdAt: '',
    updatedAt: '',
  };

  // Build commentsProps when comments are enabled (Story 6.3)
  const isManager = isManagerRole(user?.role);
  const commentsProps = commentsEnabled
    ? {
        comments: commentsData?.comments ?? [],
        total: commentsData?.total ?? 0,
        isLoading: commentsLoading,
        currentUserId: user?.id,
        isManager,
        onAddComment: handleAddComment,
        onDeleteComment: handleDeleteComment,
      }
    : undefined;

  if (isLoading || !announcement) {
    return (
      <ViewAnnouncementPage
        announcement={EMPTY_ANNOUNCEMENT}
        attachments={[]}
        isLoading
        onEdit={() => navigate(`/announcements/${announcementId}/edit`)}
        onPublish={handlePublish}
        onArchive={handleArchive}
        onPin={handlePin}
        onDelete={handleDelete}
        onBack={() => navigate('/announcements')}
      />
    );
  }

  return (
    <ViewAnnouncementPage
      announcement={announcement}
      attachments={attachments}
      onEdit={() => navigate(`/announcements/${announcementId}/edit`)}
      onPublish={handlePublish}
      onArchive={handleArchive}
      onPin={handlePin}
      onDelete={handleDelete}
      onBack={() => navigate('/announcements')}
      onMarkRead={handleMarkRead}
      onAcknowledge={announcement.acknowledgmentRequired ? handleAcknowledge : undefined}
      acknowledgmentStats={ackStatsData?.stats}
      commentsProps={commentsProps}
    />
  );
}

/** Route wrapper — guards for missing param before mounting inner component */
function ViewAnnouncementPageRoute() {
  const { announcementId } = useParams<{ announcementId: string }>();
  const { t } = useTranslation();
  if (!announcementId) {
    return <div>{t('errors.announcementNotFound', 'Announcement not found')}</div>;
  }
  return <ViewAnnouncementPageInner announcementId={announcementId} />;
}

/**
 * Inner component for the edit-announcement page — all hooks called
 * unconditionally (the route wrapper guards the missing-param case).
 *
 * Wired to @ppt/api-client standalone hooks (UC-06, gap-79-1):
 *   useAnnouncement       — load the current draft/scheduled announcement
 *   useUpdateAnnouncement  — PUT /api/v1/announcements/:id
 *
 * `AnnouncementWithDetails` extends `Announcement`, so the detail payload feeds
 * `EditAnnouncementPage` (which expects `Announcement`) without any mapping.
 */
function EditAnnouncementPageInner({ announcementId }: { announcementId: string }) {
  const navigate = useNavigate();
  const { showToast } = useToast();
  const { t } = useTranslation();

  const { data, isLoading, error } = useAnnouncement(announcementId);
  const updateAnnouncement = useUpdateAnnouncement(announcementId);

  useEffect(() => {
    if (error) {
      showToast({
        type: 'error',
        title: t('announcements.failedToLoad', { defaultValue: 'Failed to load announcement' }),
        message: error instanceof Error ? error.message : t('auth.unexpectedError'),
      });
    }
  }, [error, showToast, t]);

  const announcement = data?.announcement;

  if (isLoading || !announcement) {
    return <div className="p-6">{t('common.loading', { defaultValue: 'Loading…' })}</div>;
  }

  return (
    <EditAnnouncementPage
      announcement={announcement}
      buildings={[]}
      units={[]}
      roles={[]}
      isLoading={updateAnnouncement.isPending}
      onSubmit={async (formData: UpdateAnnouncementRequest) => {
        try {
          await updateAnnouncement.mutateAsync(formData);
          showToast({
            type: 'success',
            title: t('announcements.updated', { defaultValue: 'Updated' }),
            message: t('announcements.updatedMessage', { defaultValue: 'Announcement updated' }),
          });
          navigate(`/announcements/${announcementId}`);
        } catch (err) {
          showToast({
            type: 'error',
            title: t('announcements.updateFailed', {
              defaultValue: 'Failed to update announcement',
            }),
            message: err instanceof Error ? err.message : '',
          });
        }
      }}
      onCancel={() => navigate(`/announcements/${announcementId}`)}
    />
  );
}

/** Route wrapper — guards for missing param before mounting inner component */
function EditAnnouncementPageRoute() {
  const { announcementId } = useParams<{ announcementId: string }>();
  const { t } = useTranslation();
  if (!announcementId) {
    return <div>{t('errors.announcementNotFound', 'Announcement not found')}</div>;
  }
  return <EditAnnouncementPageInner announcementId={announcementId} />;
}

/** Announcements routes (UC-06). */
export function announcementRoutes() {
  return (
    <>
      <Route path="/announcements" element={<AnnouncementsPageRoute />} />
      <Route path="/announcements/new" element={<CreateAnnouncementPageRoute />} />
      <Route path="/announcements/:announcementId" element={<ViewAnnouncementPageRoute />} />
      <Route path="/announcements/:announcementId/edit" element={<EditAnnouncementPageRoute />} />
    </>
  );
}
