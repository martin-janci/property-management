/**
 * Announcement API Client
 *
 * API functions for managing announcements (UC-06).
 */

import type { ApiConfig } from '../index';
import type {
  AcknowledgmentStatsResponse,
  AddAttachmentRequest,
  Announcement,
  AnnouncementAttachment,
  AnnouncementComment,
  AnnouncementStatistics,
  AnnouncementSummary,
  AnnouncementWithDetails,
  CommentsResponse,
  CreateAnnouncementRequest,
  CreateCommentRequest,
  DeleteCommentRequest,
  ListAnnouncementsParams,
  ListCommentsParams,
  PaginatedResponse,
  PinAnnouncementRequest,
  ScheduleAnnouncementRequest,
  UpdateAnnouncementRequest,
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

export const createAnnouncementsApi = (config: ApiConfig) => {
  const baseUrl = `${config.baseUrl}/api/v1/announcements`;
  const headers = buildHeaders(config);

  return {
    /**
     * List announcements with filters (managers)
     */
    list: async (
      params?: ListAnnouncementsParams
    ): Promise<PaginatedResponse<AnnouncementSummary>> => {
      const searchParams = new URLSearchParams();
      if (params?.page) searchParams.set('page', params.page.toString());
      if (params?.pageSize) searchParams.set('pageSize', params.pageSize.toString());
      if (params?.status) searchParams.set('status', params.status);
      if (params?.targetType) searchParams.set('targetType', params.targetType);
      if (params?.authorId) searchParams.set('authorId', params.authorId);
      if (params?.pinned !== undefined) searchParams.set('pinned', params.pinned.toString());
      if (params?.fromDate) searchParams.set('fromDate', params.fromDate);
      if (params?.toDate) searchParams.set('toDate', params.toDate);

      const url = searchParams.toString() ? `${baseUrl}?${searchParams}` : baseUrl;
      const response = await fetch(url, { headers });
      return handleResponse(response);
    },

    /**
     * List published announcements (all users)
     */
    listPublished: async (params?: {
      page?: number;
      pageSize?: number;
    }): Promise<PaginatedResponse<AnnouncementSummary>> => {
      const searchParams = new URLSearchParams();
      if (params?.page) searchParams.set('page', params.page.toString());
      if (params?.pageSize) searchParams.set('pageSize', params.pageSize.toString());

      const url = searchParams.toString()
        ? `${baseUrl}/published?${searchParams}`
        : `${baseUrl}/published`;
      const response = await fetch(url, { headers });
      return handleResponse(response);
    },

    /**
     * Create a new announcement
     */
    create: async (data: CreateAnnouncementRequest): Promise<{ id: string; message: string }> => {
      const response = await fetch(baseUrl, {
        method: 'POST',
        headers,
        body: JSON.stringify(data),
      });
      return handleResponse(response);
    },

    /**
     * Get announcement details
     */
    get: async (
      id: string
    ): Promise<{
      announcement: AnnouncementWithDetails;
      attachments: AnnouncementAttachment[];
    }> => {
      const response = await fetch(`${baseUrl}/${id}`, { headers });
      return handleResponse(response);
    },

    /**
     * Update an announcement (draft/scheduled only)
     */
    update: async (
      id: string,
      data: UpdateAnnouncementRequest
    ): Promise<{ message: string; announcement: Announcement }> => {
      const response = await fetch(`${baseUrl}/${id}`, {
        method: 'PUT',
        headers,
        body: JSON.stringify(data),
      });
      return handleResponse(response);
    },

    /**
     * Delete an announcement (draft only)
     */
    delete: async (id: string): Promise<void> => {
      const response = await fetch(`${baseUrl}/${id}`, {
        method: 'DELETE',
        headers,
      });
      if (!response.ok) {
        const error = await response.json().catch(() => ({ message: 'Unknown error' }));
        throw new Error(error.message || `HTTP ${response.status}`);
      }
    },

    /**
     * Publish an announcement immediately
     */
    publish: async (id: string): Promise<{ message: string; announcement: Announcement }> => {
      const response = await fetch(`${baseUrl}/${id}/publish`, {
        method: 'POST',
        headers,
      });
      return handleResponse(response);
    },

    /**
     * Schedule an announcement for future publishing
     */
    schedule: async (
      id: string,
      data: ScheduleAnnouncementRequest
    ): Promise<{ message: string; announcement: Announcement }> => {
      const response = await fetch(`${baseUrl}/${id}/schedule`, {
        method: 'POST',
        headers,
        body: JSON.stringify(data),
      });
      return handleResponse(response);
    },

    /**
     * Archive an announcement
     */
    archive: async (id: string): Promise<{ message: string; announcement: Announcement }> => {
      const response = await fetch(`${baseUrl}/${id}/archive`, {
        method: 'POST',
        headers,
      });
      return handleResponse(response);
    },

    /**
     * Pin or unpin an announcement
     */
    pin: async (
      id: string,
      data: PinAnnouncementRequest
    ): Promise<{ message: string; announcement: Announcement }> => {
      const response = await fetch(`${baseUrl}/${id}/pin`, {
        method: 'POST',
        headers,
        body: JSON.stringify(data),
      });
      return handleResponse(response);
    },

    /**
     * List announcement attachments
     */
    listAttachments: async (id: string): Promise<AnnouncementAttachment[]> => {
      const response = await fetch(`${baseUrl}/${id}/attachments`, { headers });
      return handleResponse(response);
    },

    /**
     * Add an attachment to an announcement
     */
    addAttachment: async (
      id: string,
      data: AddAttachmentRequest
    ): Promise<AnnouncementAttachment> => {
      const response = await fetch(`${baseUrl}/${id}/attachments`, {
        method: 'POST',
        headers,
        body: JSON.stringify(data),
      });
      return handleResponse(response);
    },

    /**
     * Delete an attachment
     */
    deleteAttachment: async (id: string, attachmentId: string): Promise<void> => {
      const response = await fetch(`${baseUrl}/${id}/attachments/${attachmentId}`, {
        method: 'DELETE',
        headers,
      });
      if (!response.ok) {
        const error = await response.json().catch(() => ({ message: 'Unknown error' }));
        throw new Error(error.message || `HTTP ${response.status}`);
      }
    },

    /**
     * Mark an announcement as read
     */
    markRead: async (id: string): Promise<void> => {
      const response = await fetch(`${baseUrl}/${id}/read`, {
        method: 'POST',
        headers,
      });
      if (!response.ok) {
        const error = await response.json().catch(() => ({ message: 'Unknown error' }));
        throw new Error(error.message || `HTTP ${response.status}`);
      }
    },

    /**
     * Acknowledge an announcement
     */
    acknowledge: async (id: string): Promise<void> => {
      const response = await fetch(`${baseUrl}/${id}/acknowledge`, {
        method: 'POST',
        headers,
      });
      if (!response.ok) {
        const error = await response.json().catch(() => ({ message: 'Unknown error' }));
        throw new Error(error.message || `HTTP ${response.status}`);
      }
    },

    /**
     * Get announcement statistics
     */
    getStatistics: async (): Promise<AnnouncementStatistics> => {
      const response = await fetch(`${baseUrl}/statistics`, { headers });
      return handleResponse(response);
    },

    /**
     * Get unread announcement count
     */
    getUnreadCount: async (): Promise<{ unreadCount: number }> => {
      const response = await fetch(`${baseUrl}/unread-count`, { headers });
      return handleResponse(response);
    },

    /**
     * Get acknowledgment statistics for an announcement (Story 6.2)
     */
    getAcknowledgmentStats: async (id: string): Promise<AcknowledgmentStatsResponse> => {
      const response = await fetch(`${baseUrl}/${id}/acknowledgments`, { headers });
      return handleResponse(response);
    },

    // ========================================================================
    // Comments (Story 6.3)
    // ========================================================================

    /**
     * List comments for an announcement
     */
    listComments: async (id: string, params?: ListCommentsParams): Promise<CommentsResponse> => {
      const searchParams = new URLSearchParams();
      if (params?.limit) searchParams.set('limit', params.limit.toString());
      if (params?.offset) searchParams.set('offset', params.offset.toString());

      const url = searchParams.toString()
        ? `${baseUrl}/${id}/comments?${searchParams}`
        : `${baseUrl}/${id}/comments`;
      const response = await fetch(url, { headers });
      return handleResponse(response);
    },

    /**
     * Create a comment on an announcement
     */
    createComment: async (id: string, data: CreateCommentRequest): Promise<AnnouncementComment> => {
      const response = await fetch(`${baseUrl}/${id}/comments`, {
        method: 'POST',
        headers,
        body: JSON.stringify(data),
      });
      return handleResponse(response);
    },

    /**
     * Delete a comment (author or manager moderation)
     */
    deleteComment: async (
      announcementId: string,
      commentId: string,
      data?: DeleteCommentRequest
    ): Promise<AnnouncementComment> => {
      const response = await fetch(`${baseUrl}/${announcementId}/comments/${commentId}`, {
        method: 'DELETE',
        headers,
        ...(data && { body: JSON.stringify(data) }),
      });
      return handleResponse(response);
    },
  };
};

export type AnnouncementsApi = ReturnType<typeof createAnnouncementsApi>;
