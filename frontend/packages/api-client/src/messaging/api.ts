/**
 * Messaging API Client
 *
 * API functions for direct messaging (Epic 6, Story 6.5).
 */

import type { ApiConfig } from '../index';
import type {
  BlockedUsersResponse,
  ListMessagesParams,
  ListThreadsParams,
  MessageSuccessResponse,
  SendMessageRequest,
  SendMessageResponse,
  StartThreadRequest,
  ThreadDetailResponse,
  ThreadListResponse,
  UnreadMessagesResponse,
} from './types';

const buildHeaders = (config: ApiConfig): HeadersInit => ({
  'Content-Type': 'application/json',
  ...(config.accessToken && { Authorization: `Bearer ${config.accessToken}` }),
  ...(config.tenantId && { 'X-Tenant-ID': config.tenantId }),
});

const handleResponse = async <T>(response: Response): Promise<T> => {
  if (!response.ok) {
    const error = await response.json().catch(() => ({ message: 'Unknown error' }));
    throw new Error(error.message || `HTTP ${response.status}`);
  }
  return response.json();
};

export const createMessagingApi = (config: ApiConfig) => {
  const baseUrl = `${config.baseUrl}/api/v1/messages`;
  const headers = buildHeaders(config);

  return {
    // ========================================================================
    // Thread Operations
    // ========================================================================

    /**
     * List message threads for the current user
     */
    listThreads: async (params?: ListThreadsParams): Promise<ThreadListResponse> => {
      const searchParams = new URLSearchParams();
      if (params?.limit) searchParams.set('limit', params.limit.toString());
      if (params?.offset) searchParams.set('offset', params.offset.toString());
      if (params?.search) searchParams.set('search', params.search);
      if (params?.archived) searchParams.set('archived', 'true');

      const url = searchParams.toString()
        ? `${baseUrl}/threads?${searchParams}`
        : `${baseUrl}/threads`;
      const response = await fetch(url, { headers });
      return handleResponse(response);
    },

    /**
     * Start a new thread or get existing thread with another user
     */
    startThread: async (data: StartThreadRequest): Promise<ThreadDetailResponse> => {
      const response = await fetch(`${baseUrl}/threads`, {
        method: 'POST',
        headers,
        body: JSON.stringify(data),
      });
      return handleResponse(response);
    },

    /**
     * Get thread details with messages
     */
    getThread: async (id: string, params?: ListMessagesParams): Promise<ThreadDetailResponse> => {
      const searchParams = new URLSearchParams();
      if (params?.limit) searchParams.set('limit', params.limit.toString());
      if (params?.offset) searchParams.set('offset', params.offset.toString());

      const url = searchParams.toString()
        ? `${baseUrl}/threads/${id}?${searchParams}`
        : `${baseUrl}/threads/${id}`;
      const response = await fetch(url, { headers });
      return handleResponse(response);
    },

    /**
     * Send a message in a thread
     */
    sendMessage: async (
      threadId: string,
      data: SendMessageRequest
    ): Promise<SendMessageResponse> => {
      const response = await fetch(`${baseUrl}/threads/${threadId}/messages`, {
        method: 'POST',
        headers,
        body: JSON.stringify(data),
      });
      return handleResponse(response);
    },

    /**
     * Mark all messages in a thread as read
     */
    markThreadRead: async (threadId: string): Promise<MessageSuccessResponse> => {
      const response = await fetch(`${baseUrl}/threads/${threadId}/read`, {
        method: 'POST',
        headers,
      });
      return handleResponse(response);
    },

    /**
     * Delete a message from a thread (soft delete on the backend).
     */
    deleteMessage: async (threadId: string, messageId: string): Promise<MessageSuccessResponse> => {
      const response = await fetch(`${baseUrl}/threads/${threadId}/messages/${messageId}`, {
        method: 'DELETE',
        headers,
      });
      return handleResponse(response);
    },

    /**
     * Delete a thread for the current user only (per-user soft hide; BIT-182).
     *
     * The shared thread and the other participant's copy are untouched — a
     * later inbound message un-hides it.
     */
    deleteThread: async (threadId: string): Promise<MessageSuccessResponse> => {
      const response = await fetch(`${baseUrl}/threads/${threadId}`, {
        method: 'DELETE',
        headers,
      });
      return handleResponse(response);
    },

    /**
     * Archive a thread for the current user only (BIT-182).
     */
    archiveThread: async (threadId: string): Promise<MessageSuccessResponse> => {
      const response = await fetch(`${baseUrl}/threads/${threadId}/archive`, {
        method: 'POST',
        headers,
      });
      return handleResponse(response);
    },

    /**
     * Un-archive a thread for the current user only (BIT-182).
     */
    unarchiveThread: async (threadId: string): Promise<MessageSuccessResponse> => {
      const response = await fetch(`${baseUrl}/threads/${threadId}/archive`, {
        method: 'DELETE',
        headers,
      });
      return handleResponse(response);
    },

    // ========================================================================
    // Block Operations
    // ========================================================================

    /**
     * List blocked users
     */
    listBlockedUsers: async (): Promise<BlockedUsersResponse> => {
      const response = await fetch(`${baseUrl}/users/blocked`, { headers });
      return handleResponse(response);
    },

    /**
     * Block a user
     */
    blockUser: async (userId: string): Promise<MessageSuccessResponse> => {
      const response = await fetch(`${baseUrl}/users/${userId}/block`, {
        method: 'POST',
        headers,
      });
      return handleResponse(response);
    },

    /**
     * Unblock a user
     */
    unblockUser: async (userId: string): Promise<MessageSuccessResponse> => {
      const response = await fetch(`${baseUrl}/users/${userId}/block`, {
        method: 'DELETE',
        headers,
      });
      return handleResponse(response);
    },

    // ========================================================================
    // Stats
    // ========================================================================

    /**
     * Get unread message count
     */
    getUnreadCount: async (): Promise<UnreadMessagesResponse> => {
      const response = await fetch(`${baseUrl}/unread-count`, { headers });
      return handleResponse(response);
    },
  };
};

export type MessagingApi = ReturnType<typeof createMessagingApi>;
