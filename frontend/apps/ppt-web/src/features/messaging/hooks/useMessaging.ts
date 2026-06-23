/**
 * Messaging Hooks
 *
 * React hooks for direct messaging, wiring @ppt/api-client to the
 * feature-level UI types (Epic 6, Story 6.5).
 */

import type {
  ListThreadsParams as ApiListThreadsParams,
  MessageWithSender,
  StartThreadRequest,
  ThreadDetailResponse,
  ThreadWithPreview,
} from '@ppt/api-client';
import {
  createMessagingApi,
  createMessagingHooks,
  createNeighborHooks,
  createNeighborsApi,
  getToken,
  messagingKeys,
} from '@ppt/api-client';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { useMemo } from 'react';
import type {
  Message,
  MessageThread,
  RecipientOption,
  SendMessageRequest,
  ThreadWithMessages,
} from '../types';

// ---------------------------------------------------------------------------
// API client factories
// ---------------------------------------------------------------------------

const API_BASE_URL = import.meta.env.VITE_API_BASE_URL ?? '';

// NOTE: `createMessagingApi` / `createNeighborsApi` bake the
// `Authorization: Bearer <token>` header into the returned client at
// construction time. Callers must re-create the client whenever the token
// rotates (the `useMemo` deps below are keyed on the token), otherwise a stale
// Bearer token is sent for the rest of the component's lifetime.
function getMessagingApi(accessToken: string | undefined) {
  return createMessagingApi({
    baseUrl: API_BASE_URL,
    accessToken,
  });
}

function getNeighborsApi(accessToken: string | undefined) {
  return createNeighborsApi({
    baseUrl: API_BASE_URL,
    accessToken,
  });
}

// ---------------------------------------------------------------------------
// Type adapters: API → UI
// ---------------------------------------------------------------------------

/**
 * Join a participant's first/last name into a display name.
 *
 * The `users` table has a single `name` column, so the API maps it onto
 * `firstName` and leaves `lastName` empty. Trim the join so a missing last
 * name doesn't leave a trailing space (e.g. `"Jane "`).
 */
function fullName(firstName: string, lastName: string): string {
  return `${firstName} ${lastName}`.trim();
}

/**
 * Map an API ThreadWithPreview to the feature-layer MessageThread type.
 */
export function mapApiThreadToUi(apiThread: ThreadWithPreview): MessageThread {
  const participant = apiThread.otherParticipant;
  const lastMsg = apiThread.lastMessage;

  return {
    id: apiThread.id,
    subject: undefined,
    participantCount: apiThread.participantIds.length,
    messageCount: 0, // not available in list view
    lastMessageAt: apiThread.lastMessage?.createdAt ?? undefined,
    lastMessagePreview: lastMsg?.content ?? undefined,
    lastMessageSenderId: lastMsg?.senderId ?? undefined,
    lastMessageSenderName: lastMsg?.isFromMe
      ? undefined
      : fullName(participant.firstName, participant.lastName),
    unreadCount: apiThread.unreadCount,
    createdAt: apiThread.createdAt,
    updatedAt: apiThread.updatedAt,
    participants: [
      {
        id: participant.id,
        userId: participant.id,
        userName: fullName(participant.firstName, participant.lastName),
        userAvatar: undefined,
        joinedAt: apiThread.createdAt,
        lastReadAt: undefined,
      },
    ],
    isArchived: false,
  };
}

/**
 * Map an API MessageWithSender to the feature-layer Message type.
 */
export function mapApiMessageToUi(apiMsg: MessageWithSender): Message {
  return {
    id: apiMsg.id,
    threadId: apiMsg.threadId,
    senderId: apiMsg.sender.id,
    senderName: fullName(apiMsg.sender.firstName, apiMsg.sender.lastName),
    content: apiMsg.content,
    createdAt: apiMsg.createdAt,
    readBy: apiMsg.readAt ? [apiMsg.sender.id] : [],
  };
}

/**
 * Map an API ThreadDetailResponse to the feature-layer ThreadWithMessages type.
 */
export function mapApiThreadDetailToUi(detail: ThreadDetailResponse): ThreadWithMessages {
  const thread = detail.thread;
  const other = detail.otherParticipant;
  const messages = detail.messages.map(mapApiMessageToUi);

  return {
    id: thread.id,
    subject: undefined,
    participantCount: thread.participantIds.length,
    messageCount: detail.messageCount,
    lastMessageAt: thread.lastMessageAt ?? undefined,
    unreadCount: 0, // mark-as-read keeps this fresh
    createdAt: thread.createdAt,
    updatedAt: thread.updatedAt,
    participants: [
      {
        id: other.id,
        userId: other.id,
        userName: fullName(other.firstName, other.lastName),
        userAvatar: undefined,
        joinedAt: thread.createdAt,
        lastReadAt: undefined,
      },
    ],
    messages,
  };
}

// ---------------------------------------------------------------------------
// Hook factory
// ---------------------------------------------------------------------------

/**
 * Returns TanStack Query hooks for the messaging API.
 */
export function useMessagingApi() {
  const token = getToken() ?? undefined;
  const api = useMemo(() => getMessagingApi(token), [token]);
  return useMemo(() => createMessagingHooks(api), [api]);
}

/**
 * Hook to list threads (adapted to UI types).
 */
export function useThreads(params?: ApiListThreadsParams) {
  const hooks = useMessagingApi();
  const query = hooks.useThreads(params);
  return {
    ...query,
    threads: query.data?.threads.map(mapApiThreadToUi) ?? [],
    total: query.data?.total ?? 0,
  };
}

/**
 * Hook to get unread count.
 */
export function useUnreadCount() {
  const hooks = useMessagingApi();
  return hooks.useUnreadCount();
}

/**
 * Hook to get a single thread with messages (adapted to UI types).
 */
export function useThread(threadId: string, enabled = true) {
  const hooks = useMessagingApi();
  const query = hooks.useThread(threadId, undefined, enabled);
  return {
    ...query,
    thread: query.data ? mapApiThreadDetailToUi(query.data) : undefined,
  };
}

/**
 * Mutation to start a new thread.
 */
export function useStartThread() {
  const hooks = useMessagingApi();
  return hooks.useStartThread();
}

/**
 * Mutation to send a message in a thread.
 */
export function useSendMessage() {
  const hooks = useMessagingApi();
  return hooks.useSendMessage();
}

/**
 * Mutation to mark a thread as read.
 */
export function useMarkThreadRead() {
  const hooks = useMessagingApi();
  return hooks.useMarkThreadRead();
}

/**
 * Mutation to delete (soft-delete) a message in a thread (UC-05.6).
 *
 * The generated messaging hooks factory does not expose a delete mutation, so
 * we build it here against the raw messaging API client (re-created when the
 * token rotates) and invalidate the affected thread + thread list on success.
 */
export function useDeleteMessage() {
  const token = getToken() ?? undefined;
  const api = useMemo(() => getMessagingApi(token), [token]);
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ threadId, messageId }: { threadId: string; messageId: string }) =>
      api.deleteMessage(threadId, messageId),
    onSuccess: (_data, { threadId }) => {
      queryClient.invalidateQueries({ queryKey: messagingKeys.threadDetail(threadId) });
      queryClient.invalidateQueries({ queryKey: messagingKeys.threads() });
    },
  });
}

/**
 * Mutation to delete a thread for the current user only (per-user soft hide;
 * BIT-182, UC-05.7). The shared thread and the other participant's copy are
 * untouched. Invalidation of the thread list is handled by the api-client hook.
 */
export function useDeleteThread() {
  const hooks = useMessagingApi();
  return hooks.useDeleteThread();
}

/**
 * Mutation to archive a thread for the current user only (BIT-182, UC-05.11).
 */
export function useArchiveThread() {
  const hooks = useMessagingApi();
  return hooks.useArchiveThread();
}

/**
 * Mutation to un-archive a thread for the current user only (BIT-182).
 */
export function useUnarchiveThread() {
  const hooks = useMessagingApi();
  return hooks.useUnarchiveThread();
}

// ---------------------------------------------------------------------------
// Recipients (for NewMessagePage) — sourced from neighbors API
// ---------------------------------------------------------------------------

/**
 * Hook to list potential message recipients from neighbors in the given building.
 * Falls back to empty list when buildingId is absent.
 */
export function useMessageRecipients(buildingId?: string): {
  recipients: RecipientOption[];
  isLoading: boolean;
} {
  const token = getToken() ?? undefined;
  const api = useMemo(() => getNeighborsApi(token), [token]);
  const hooks = useMemo(() => createNeighborHooks(api), [api]);
  const { data, isLoading } = hooks.useNeighbors(buildingId ?? '', !!buildingId);

  const recipients: RecipientOption[] = (data?.neighbors ?? [])
    .filter((n) => n.isVisible)
    .map((n) => ({
      id: n.userId,
      name: n.displayName,
      email: n.email ?? undefined,
    }));

  return { recipients, isLoading };
}

// ---------------------------------------------------------------------------
// Data adapters for mutation inputs
// ---------------------------------------------------------------------------

/**
 * Map feature-layer CreateThreadRequest to the API StartThreadRequest.
 * The API only supports a single recipient; we use the first one.
 */
export function toStartThreadRequest(data: {
  recipientIds: string[];
  initialMessage: string;
}): StartThreadRequest {
  return {
    recipientId: data.recipientIds[0] ?? '',
    initialMessage: data.initialMessage,
  };
}

/**
 * Map feature-layer SendMessageRequest to the API SendMessageRequest.
 */
export function toApiSendMessageRequest(data: SendMessageRequest): { content: string } {
  return { content: data.content };
}
