/**
 * Messaging TanStack Query Hooks
 *
 * React hooks for direct messaging with server state caching (Epic 6, Story 6.5).
 */

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import type { MessagingApi } from './api';
import type {
  ListMessagesParams,
  ListThreadsParams,
  SendMessageRequest,
  StartThreadRequest,
} from './types';

// Query keys factory for cache management
export const messagingKeys = {
  all: ['messages'] as const,
  threads: () => [...messagingKeys.all, 'threads'] as const,
  threadList: (params?: ListThreadsParams) => [...messagingKeys.threads(), 'list', params] as const,
  threadDetail: (id: string, params?: ListMessagesParams) =>
    [...messagingKeys.threads(), 'detail', id, params] as const,
  blockedUsers: () => [...messagingKeys.all, 'blocked'] as const,
  unreadCount: () => [...messagingKeys.all, 'unread'] as const,
};

export const createMessagingHooks = (api: MessagingApi) => ({
  /**
   * List message threads
   */
  useThreads: (params?: ListThreadsParams) =>
    useQuery({
      queryKey: messagingKeys.threadList(params),
      queryFn: () => api.listThreads(params),
    }),

  /**
   * Get thread details with messages
   */
  useThread: (id: string, params?: ListMessagesParams, enabled = true) =>
    useQuery({
      queryKey: messagingKeys.threadDetail(id, params),
      queryFn: () => api.getThread(id, params),
      enabled: enabled && !!id,
    }),

  /**
   * Get unread message count
   */
  useUnreadCount: () =>
    useQuery({
      queryKey: messagingKeys.unreadCount(),
      queryFn: () => api.getUnreadCount(),
      refetchInterval: 30000, // Refetch every 30 seconds
    }),

  /**
   * List blocked users
   */
  useBlockedUsers: () =>
    useQuery({
      queryKey: messagingKeys.blockedUsers(),
      queryFn: () => api.listBlockedUsers(),
    }),

  /**
   * Start a new thread mutation
   */
  useStartThread: () => {
    const queryClient = useQueryClient();
    return useMutation({
      mutationFn: (data: StartThreadRequest) => api.startThread(data),
      onSuccess: () => {
        queryClient.invalidateQueries({ queryKey: messagingKeys.threads() });
      },
    });
  },

  /**
   * Send message mutation
   */
  useSendMessage: () => {
    const queryClient = useQueryClient();
    return useMutation({
      mutationFn: ({ threadId, data }: { threadId: string; data: SendMessageRequest }) =>
        api.sendMessage(threadId, data),
      onSuccess: (_, { threadId }) => {
        queryClient.invalidateQueries({ queryKey: messagingKeys.threadDetail(threadId) });
        queryClient.invalidateQueries({ queryKey: messagingKeys.threads() });
      },
    });
  },

  /**
   * Mark thread as read mutation
   */
  useMarkThreadRead: () => {
    const queryClient = useQueryClient();
    return useMutation({
      mutationFn: (threadId: string) => api.markThreadRead(threadId),
      onSuccess: (_data: unknown, threadId: string) => {
        queryClient.invalidateQueries({ queryKey: messagingKeys.threadDetail(threadId) });
        queryClient.invalidateQueries({ queryKey: messagingKeys.threads() });
        queryClient.invalidateQueries({ queryKey: messagingKeys.unreadCount() });
      },
    });
  },

  /**
   * Delete a thread for the current user only (per-user soft hide; BIT-182).
   */
  useDeleteThread: () => {
    const queryClient = useQueryClient();
    return useMutation({
      mutationFn: (threadId: string) => api.deleteThread(threadId),
      onSuccess: () => {
        queryClient.invalidateQueries({ queryKey: messagingKeys.threads() });
        queryClient.invalidateQueries({ queryKey: messagingKeys.unreadCount() });
      },
    });
  },

  /**
   * Archive a thread for the current user only (BIT-182).
   */
  useArchiveThread: () => {
    const queryClient = useQueryClient();
    return useMutation({
      mutationFn: (threadId: string) => api.archiveThread(threadId),
      onSuccess: () => {
        queryClient.invalidateQueries({ queryKey: messagingKeys.threads() });
      },
    });
  },

  /**
   * Un-archive a thread for the current user only (BIT-182).
   */
  useUnarchiveThread: () => {
    const queryClient = useQueryClient();
    return useMutation({
      mutationFn: (threadId: string) => api.unarchiveThread(threadId),
      onSuccess: () => {
        queryClient.invalidateQueries({ queryKey: messagingKeys.threads() });
      },
    });
  },

  /**
   * Block user mutation
   */
  useBlockUser: () => {
    const queryClient = useQueryClient();
    return useMutation({
      mutationFn: (userId: string) => api.blockUser(userId),
      onSuccess: () => {
        queryClient.invalidateQueries({ queryKey: messagingKeys.blockedUsers() });
      },
    });
  },

  /**
   * Unblock user mutation
   */
  useUnblockUser: () => {
    const queryClient = useQueryClient();
    return useMutation({
      mutationFn: (userId: string) => api.unblockUser(userId),
      onSuccess: () => {
        queryClient.invalidateQueries({ queryKey: messagingKeys.blockedUsers() });
      },
    });
  },
});

export type MessagingHooks = ReturnType<typeof createMessagingHooks>;
