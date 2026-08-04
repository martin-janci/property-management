/**
 * Messaging route group (UC-07).
 *
 * Owns the messaging route-wrapper components and the `<Route>` table fragment.
 * Extracted from App.tsx to isolate messaging work.
 */

import { useBuildings } from '@ppt/api-client';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Route, useNavigate, useParams, useSearchParams } from 'react-router-dom';
import { useToast } from '../../components';
import { useAuth } from '../../contexts';
import type { CreateThreadRequest, SendMessageRequest } from '../../features/messaging';
import {
  toStartThreadRequest,
  useArchiveThread,
  useDeleteMessage,
  useDeleteThread,
  useMarkThreadRead,
  useMessageRecipients,
  useSendMessageWithAttachments,
  useStartThread,
  useThread,
  useThreads,
  useUnreadCount,
} from '../../features/messaging/hooks/useMessaging';
import { MessagesPage, NewMessagePage, ThreadDetailPage } from '../lazyRoutes';

function MessagesPageRoute() {
  const navigate = useNavigate();
  const { t } = useTranslation();
  const { showToast } = useToast();
  const [msgQueryParams, setMsgQueryParams] = useState<{
    limit: number;
    offset: number;
    search?: string;
    archived?: boolean;
  }>({
    limit: 20,
    offset: 0,
  });

  const { threads, total, isLoading: threadsLoading } = useThreads(msgQueryParams);
  const { data: unreadData } = useUnreadCount();
  const unreadCount = unreadData?.unreadCount ?? 0;

  const deleteThread = useDeleteThread();
  const archiveThread = useArchiveThread();

  // Per-user delete / archive operate on one thread at a time (BIT-182); the
  // bulk-select UI hands us the selected ids, so fan out and surface a single
  // toast for the batch. The mutation hooks invalidate the thread list on
  // success.
  const handleDeleteThreads = async (threadIds: string[]) => {
    try {
      await Promise.all(threadIds.map((id) => deleteThread.mutateAsync(id)));
      showToast({
        type: 'success',
        title: t('common.success'),
        message: t('messaging.deleteSuccess', 'Conversations deleted'),
      });
    } catch {
      showToast({
        type: 'error',
        title: t('common.error'),
        message: t('messaging.deleteFailed', 'Failed to delete conversations'),
      });
    }
  };

  const handleArchiveThreads = async (threadIds: string[]) => {
    try {
      await Promise.all(threadIds.map((id) => archiveThread.mutateAsync(id)));
      showToast({
        type: 'success',
        title: t('common.success'),
        message: t('messaging.archiveSuccess', 'Conversations archived'),
      });
    } catch {
      showToast({
        type: 'error',
        title: t('common.error'),
        message: t('messaging.archiveFailed', 'Failed to archive conversations'),
      });
    }
  };

  return (
    <MessagesPage
      threads={threads}
      total={total}
      unreadCount={unreadCount}
      isLoading={threadsLoading}
      onNavigateToThread={(threadId) => navigate(`/messages/${threadId}`)}
      onNavigateToCreate={() => navigate('/messages/new')}
      onFilterChange={(params) => {
        setMsgQueryParams({
          limit: params.pageSize ?? 20,
          offset: ((params.page ?? 1) - 1) * (params.pageSize ?? 20),
          search: params.search,
          archived: params.filter === 'archived',
        });
      }}
      onDeleteThreads={handleDeleteThreads}
      onArchiveThreads={handleArchiveThreads}
    />
  );
}

function NewMessagePageRoute() {
  const navigate = useNavigate();
  const { showToast } = useToast();
  const { t } = useTranslation();
  const [searchParams] = useSearchParams();

  // Fetch potential recipients from the user's building neighbors. Without a
  // building id the recipients query stays disabled and the list is always
  // empty — making the page unusable. Mirror the NeighborsPage convention and
  // use the first building the user has access to (building-selector deferred).
  const { data: buildingsData, isLoading: isLoadingBuildings } = useBuildings();
  const buildingId = buildingsData?.items?.[0]?.id;
  const { recipients, isLoading: isLoadingRecipients } = useMessageRecipients(buildingId);
  const startThread = useStartThread();

  const handleNewMsgSubmit = async (data: CreateThreadRequest) => {
    if (!data.recipientIds[0]) return;
    try {
      const result = await startThread.mutateAsync(toStartThreadRequest(data));
      showToast({
        type: 'success',
        title: t('common.success'),
        message: t('messaging.messageSent', 'Message sent'),
      });
      navigate(`/messages/${result.thread.id}`);
    } catch {
      showToast({
        type: 'error',
        title: t('common.error'),
        message: t('messaging.sendFailed', 'Failed to send message'),
      });
    }
  };

  return (
    <NewMessagePage
      recipients={recipients}
      initialRecipientIds={
        searchParams.get('recipientId') ? [searchParams.get('recipientId') as string] : undefined
      }
      isLoadingRecipients={isLoadingRecipients || isLoadingBuildings}
      isSubmitting={startThread.isPending}
      onSubmit={handleNewMsgSubmit}
      onCancel={() => navigate('/messages')}
    />
  );
}

function ThreadDetailPageRoute() {
  const { threadId } = useParams<{ threadId: string }>();
  const navigate = useNavigate();
  const { user } = useAuth();
  const { showToast } = useToast();
  const { t } = useTranslation();

  const { thread, isLoading: threadLoading } = useThread(threadId ?? '', !!threadId);
  const sendMessage = useSendMessageWithAttachments();
  const markRead = useMarkThreadRead();
  const deleteMessage = useDeleteMessage();

  if (!threadId) {
    return <div>{t('errors.threadNotFound', 'Thread not found')}</div>;
  }

  const handleThreadSendMessage = async (data: SendMessageRequest) => {
    try {
      await sendMessage.mutateAsync({ threadId, data });
    } catch {
      showToast({
        type: 'error',
        title: t('common.error'),
        message: t('messaging.sendFailed', 'Failed to send message'),
      });
    }
  };

  const handleThreadMarkAsRead = () => {
    markRead.mutate(threadId);
  };

  // Show loading skeleton until thread data arrives
  if (threadLoading || !thread) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600" />
      </div>
    );
  }

  return (
    <ThreadDetailPage
      thread={thread}
      currentUserId={user?.id ?? ''}
      isLoading={false}
      isSending={sendMessage.isPending}
      onSendMessage={handleThreadSendMessage}
      onDeleteMessage={(messageId) => deleteMessage.mutate({ threadId, messageId })}
      onMarkAsRead={handleThreadMarkAsRead}
      onBack={() => navigate('/messages')}
    />
  );
}

/** Messaging routes (UC-07). */
export function messagingRoutes() {
  return (
    <>
      <Route path="/messages" element={<MessagesPageRoute />} />
      <Route path="/messages/new" element={<NewMessagePageRoute />} />
      <Route path="/messages/:threadId" element={<ThreadDetailPageRoute />} />
    </>
  );
}
