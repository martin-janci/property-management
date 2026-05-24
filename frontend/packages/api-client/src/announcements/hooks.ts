/**
 * Announcement TanStack Query Hooks
 *
 * React hooks for managing announcements with server state caching.
 */

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { getToken } from '../auth';
import type { AnnouncementsApi } from './api';
import type {
  AddAttachmentRequest,
  Announcement,
  AnnouncementSummary,
  CreateAnnouncementRequest,
  CreateCommentRequest,
  ListAnnouncementsParams,
  ListCommentsParams,
  PaginatedResponse,
  PinAnnouncementRequest,
  ScheduleAnnouncementRequest,
  UpdateAnnouncementRequest,
} from './types';

// ============================================================================
// Standalone fetch helpers (mirrors announcements/api.ts but avoids needing
// an injected ApiConfig — auth tokens are handled by the axios interceptors
// configured in ppt-web/src/lib/api.ts which patches window.fetch via the
// same base path /api/v1).
// ============================================================================

const ANNOUNCEMENTS_BASE = '/api/v1/announcements';

function getAuthHeaders(): HeadersInit {
  const token = getToken();
  return token ? { Authorization: `Bearer ${token}` } : {};
}

async function fetchJson<T>(url: string, init?: RequestInit): Promise<T> {
  const response = await fetch(url, {
    ...init,
    headers: {
      'Content-Type': 'application/json',
      ...getAuthHeaders(),
      ...init?.headers,
    },
  });
  if (!response.ok) {
    const err = await response.json().catch(() => ({ message: 'Unknown error' }));
    throw new Error((err as { message?: string }).message || `HTTP ${response.status}`);
  }
  if (response.status === 204) return undefined as unknown as T;
  return response.json() as Promise<T>;
}

function buildAnnouncementQuery(params?: ListAnnouncementsParams): string {
  if (!params) return '';
  const sp = new URLSearchParams();
  if (params.page) sp.set('page', params.page.toString());
  if (params.pageSize) sp.set('pageSize', params.pageSize.toString());
  if (params.status) sp.set('status', params.status);
  if (params.targetType) sp.set('targetType', params.targetType);
  if (params.authorId) sp.set('authorId', params.authorId);
  if (params.pinned !== undefined) sp.set('pinned', params.pinned.toString());
  if (params.fromDate) sp.set('fromDate', params.fromDate);
  if (params.toDate) sp.set('toDate', params.toDate);
  const q = sp.toString();
  return q ? `?${q}` : '';
}

// Query keys factory for cache management
export const announcementKeys = {
  all: ['announcements'] as const,
  lists: () => [...announcementKeys.all, 'list'] as const,
  list: (params?: ListAnnouncementsParams) => [...announcementKeys.lists(), params] as const,
  published: () => [...announcementKeys.all, 'published'] as const,
  details: () => [...announcementKeys.all, 'detail'] as const,
  detail: (id: string) => [...announcementKeys.details(), id] as const,
  attachments: (id: string) => [...announcementKeys.detail(id), 'attachments'] as const,
  acknowledgments: (id: string) => [...announcementKeys.detail(id), 'acknowledgments'] as const,
  comments: (id: string, params?: ListCommentsParams) =>
    [...announcementKeys.detail(id), 'comments', params] as const,
  statistics: () => [...announcementKeys.all, 'statistics'] as const,
  unreadCount: () => [...announcementKeys.all, 'unread-count'] as const,
};

export const createAnnouncementHooks = (api: AnnouncementsApi) => ({
  /**
   * List announcements with filters
   */
  useList: (params?: ListAnnouncementsParams) =>
    useQuery({
      queryKey: announcementKeys.list(params),
      queryFn: () => api.list(params),
    }),

  /**
   * List published announcements
   */
  useListPublished: (params?: { page?: number; pageSize?: number }) =>
    useQuery({
      queryKey: announcementKeys.published(),
      queryFn: () => api.listPublished(params),
    }),

  /**
   * Get announcement details
   */
  useGet: (id: string, enabled = true) =>
    useQuery({
      queryKey: announcementKeys.detail(id),
      queryFn: () => api.get(id),
      enabled: enabled && !!id,
    }),

  /**
   * Get announcement statistics
   */
  useStatistics: () =>
    useQuery({
      queryKey: announcementKeys.statistics(),
      queryFn: () => api.getStatistics(),
    }),

  /**
   * Get unread announcement count
   */
  useUnreadCount: () =>
    useQuery({
      queryKey: announcementKeys.unreadCount(),
      queryFn: () => api.getUnreadCount(),
      refetchInterval: 60000, // Refetch every minute
    }),

  /**
   * Create announcement mutation
   */
  useCreate: () => {
    const queryClient = useQueryClient();
    return useMutation({
      mutationFn: (data: CreateAnnouncementRequest) => api.create(data),
      onSuccess: () => {
        queryClient.invalidateQueries({ queryKey: announcementKeys.lists() });
        queryClient.invalidateQueries({ queryKey: announcementKeys.statistics() });
      },
    });
  },

  /**
   * Update announcement mutation
   */
  useUpdate: () => {
    const queryClient = useQueryClient();
    return useMutation({
      mutationFn: ({ id, data }: { id: string; data: UpdateAnnouncementRequest }) =>
        api.update(id, data),
      onSuccess: (_, { id }) => {
        queryClient.invalidateQueries({ queryKey: announcementKeys.detail(id) });
        queryClient.invalidateQueries({ queryKey: announcementKeys.lists() });
      },
    });
  },

  /**
   * Delete announcement mutation
   */
  useDelete: () => {
    const queryClient = useQueryClient();
    return useMutation({
      mutationFn: (id: string) => api.delete(id),
      onSuccess: () => {
        queryClient.invalidateQueries({ queryKey: announcementKeys.lists() });
        queryClient.invalidateQueries({ queryKey: announcementKeys.statistics() });
      },
    });
  },

  /**
   * Publish announcement mutation
   */
  usePublish: () => {
    const queryClient = useQueryClient();
    return useMutation({
      mutationFn: (id: string) => api.publish(id),
      onSuccess: (_data: unknown, id: string) => {
        queryClient.invalidateQueries({ queryKey: announcementKeys.detail(id) });
        queryClient.invalidateQueries({ queryKey: announcementKeys.lists() });
        queryClient.invalidateQueries({ queryKey: announcementKeys.published() });
        queryClient.invalidateQueries({ queryKey: announcementKeys.statistics() });
      },
    });
  },

  /**
   * Schedule announcement mutation
   */
  useSchedule: () => {
    const queryClient = useQueryClient();
    return useMutation({
      mutationFn: ({ id, data }: { id: string; data: ScheduleAnnouncementRequest }) =>
        api.schedule(id, data),
      onSuccess: (_, { id }) => {
        queryClient.invalidateQueries({ queryKey: announcementKeys.detail(id) });
        queryClient.invalidateQueries({ queryKey: announcementKeys.lists() });
        queryClient.invalidateQueries({ queryKey: announcementKeys.statistics() });
      },
    });
  },

  /**
   * Archive announcement mutation
   */
  useArchive: () => {
    const queryClient = useQueryClient();
    return useMutation({
      mutationFn: (id: string) => api.archive(id),
      onSuccess: (_data: unknown, id: string) => {
        queryClient.invalidateQueries({ queryKey: announcementKeys.detail(id) });
        queryClient.invalidateQueries({ queryKey: announcementKeys.lists() });
        queryClient.invalidateQueries({ queryKey: announcementKeys.published() });
        queryClient.invalidateQueries({ queryKey: announcementKeys.statistics() });
      },
    });
  },

  /**
   * Pin/unpin announcement mutation
   */
  usePin: () => {
    const queryClient = useQueryClient();
    return useMutation({
      mutationFn: ({ id, data }: { id: string; data: PinAnnouncementRequest }) => api.pin(id, data),
      onSuccess: (_, { id }) => {
        queryClient.invalidateQueries({ queryKey: announcementKeys.detail(id) });
        queryClient.invalidateQueries({ queryKey: announcementKeys.lists() });
        queryClient.invalidateQueries({ queryKey: announcementKeys.published() });
      },
    });
  },

  /**
   * Add attachment mutation
   */
  useAddAttachment: () => {
    const queryClient = useQueryClient();
    return useMutation({
      mutationFn: ({ id, data }: { id: string; data: AddAttachmentRequest }) =>
        api.addAttachment(id, data),
      onSuccess: (_, { id }) => {
        queryClient.invalidateQueries({ queryKey: announcementKeys.detail(id) });
        queryClient.invalidateQueries({ queryKey: announcementKeys.attachments(id) });
      },
    });
  },

  /**
   * Delete attachment mutation
   */
  useDeleteAttachment: () => {
    const queryClient = useQueryClient();
    return useMutation({
      mutationFn: ({ id, attachmentId }: { id: string; attachmentId: string }) =>
        api.deleteAttachment(id, attachmentId),
      onSuccess: (_, { id }) => {
        queryClient.invalidateQueries({ queryKey: announcementKeys.detail(id) });
        queryClient.invalidateQueries({ queryKey: announcementKeys.attachments(id) });
      },
    });
  },

  /**
   * Mark announcement as read mutation
   */
  useMarkRead: () => {
    const queryClient = useQueryClient();
    return useMutation({
      mutationFn: (id: string) => api.markRead(id),
      onSuccess: (_data: unknown, id: string) => {
        queryClient.invalidateQueries({ queryKey: announcementKeys.detail(id) });
        queryClient.invalidateQueries({ queryKey: announcementKeys.unreadCount() });
      },
    });
  },

  /**
   * Acknowledge announcement mutation
   */
  useAcknowledge: () => {
    const queryClient = useQueryClient();
    return useMutation({
      mutationFn: (id: string) => api.acknowledge(id),
      onSuccess: (_data: unknown, id: string) => {
        queryClient.invalidateQueries({ queryKey: announcementKeys.detail(id) });
        queryClient.invalidateQueries({ queryKey: announcementKeys.acknowledgments(id) });
      },
    });
  },

  /**
   * Get acknowledgment statistics for an announcement (Story 6.2)
   */
  useAcknowledgmentStats: (id: string, enabled = true) =>
    useQuery({
      queryKey: announcementKeys.acknowledgments(id),
      queryFn: () => api.getAcknowledgmentStats(id),
      enabled: enabled && !!id,
    }),

  // ========================================================================
  // Comments (Story 6.3)
  // ========================================================================

  /**
   * List comments for an announcement
   */
  useComments: (id: string, params?: ListCommentsParams, enabled = true) =>
    useQuery({
      queryKey: announcementKeys.comments(id, params),
      queryFn: () => api.listComments(id, params),
      enabled: enabled && !!id,
    }),

  /**
   * Create comment mutation
   */
  useCreateComment: () => {
    const queryClient = useQueryClient();
    return useMutation({
      mutationFn: ({ id, data }: { id: string; data: CreateCommentRequest }) =>
        api.createComment(id, data),
      onSuccess: (_, { id }) => {
        queryClient.invalidateQueries({ queryKey: announcementKeys.comments(id) });
        queryClient.invalidateQueries({ queryKey: announcementKeys.detail(id) });
      },
    });
  },

  /**
   * Delete comment mutation
   */
  useDeleteComment: () => {
    const queryClient = useQueryClient();
    return useMutation({
      mutationFn: ({
        announcementId,
        commentId,
        reason,
      }: {
        announcementId: string;
        commentId: string;
        reason?: string;
      }) => api.deleteComment(announcementId, commentId, reason ? { reason } : undefined),
      onSuccess: (_, { announcementId }) => {
        queryClient.invalidateQueries({ queryKey: announcementKeys.comments(announcementId) });
        queryClient.invalidateQueries({ queryKey: announcementKeys.detail(announcementId) });
      },
    });
  },
});

export type AnnouncementHooks = ReturnType<typeof createAnnouncementHooks>;

// ============================================================================
// Standalone hooks — wire directly into App.tsx route wrappers without
// needing an instantiated AnnouncementsApi config object.
// ============================================================================

/** List announcements with optional filters */
export function useAnnouncements(params?: ListAnnouncementsParams) {
  return useQuery<PaginatedResponse<AnnouncementSummary>>({
    queryKey: announcementKeys.list(params),
    queryFn: () =>
      fetchJson<PaginatedResponse<AnnouncementSummary>>(
        `${ANNOUNCEMENTS_BASE}${buildAnnouncementQuery(params)}`
      ),
  });
}

/** Delete an announcement (draft only) */
export function useDeleteAnnouncement() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) =>
      fetchJson<void>(`${ANNOUNCEMENTS_BASE}/${id}`, { method: 'DELETE' }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: announcementKeys.lists() });
      queryClient.invalidateQueries({ queryKey: announcementKeys.statistics() });
    },
  });
}

/** Publish an announcement immediately */
export function usePublishAnnouncement() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) =>
      fetchJson<{ message: string; announcement: Announcement }>(
        `${ANNOUNCEMENTS_BASE}/${id}/publish`,
        {
          method: 'POST',
        }
      ),
    onSuccess: (_data, id) => {
      queryClient.invalidateQueries({ queryKey: announcementKeys.detail(id) });
      queryClient.invalidateQueries({ queryKey: announcementKeys.lists() });
      queryClient.invalidateQueries({ queryKey: announcementKeys.published() });
      queryClient.invalidateQueries({ queryKey: announcementKeys.statistics() });
    },
  });
}

/** Archive an announcement */
export function useArchiveAnnouncement() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) =>
      fetchJson<{ message: string; announcement: Announcement }>(
        `${ANNOUNCEMENTS_BASE}/${id}/archive`,
        {
          method: 'POST',
        }
      ),
    onSuccess: (_data, id) => {
      queryClient.invalidateQueries({ queryKey: announcementKeys.detail(id) });
      queryClient.invalidateQueries({ queryKey: announcementKeys.lists() });
      queryClient.invalidateQueries({ queryKey: announcementKeys.published() });
      queryClient.invalidateQueries({ queryKey: announcementKeys.statistics() });
    },
  });
}

/** Pin or unpin an announcement */
export function usePinAnnouncement() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, pinned }: { id: string; pinned: boolean }) =>
      fetchJson<{ message: string; announcement: Announcement }>(
        `${ANNOUNCEMENTS_BASE}/${id}/pin`,
        {
          method: 'POST',
          body: JSON.stringify({ pinned }),
        }
      ),
    onSuccess: (_data, { id }) => {
      queryClient.invalidateQueries({ queryKey: announcementKeys.detail(id) });
      queryClient.invalidateQueries({ queryKey: announcementKeys.lists() });
      queryClient.invalidateQueries({ queryKey: announcementKeys.published() });
    },
  });
}

/** Get a single announcement with its details and attachments (Story 6.2) */
export function useAnnouncement(id: string, enabled = true) {
  return useQuery<{ announcement: import('./types').AnnouncementWithDetails; attachments: import('./types').AnnouncementAttachment[] }>({
    queryKey: announcementKeys.detail(id),
    queryFn: () =>
      fetchJson(`${ANNOUNCEMENTS_BASE}/${id}`),
    enabled: enabled && !!id,
  });
}

/** Mark an announcement as read by the current user (Story 6.2) */
export function useMarkReadAnnouncement() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) =>
      fetchJson<void>(`${ANNOUNCEMENTS_BASE}/${id}/read`, { method: 'POST' }),
    onSuccess: (_data, id) => {
      queryClient.invalidateQueries({ queryKey: announcementKeys.detail(id) });
      queryClient.invalidateQueries({ queryKey: announcementKeys.unreadCount() });
    },
  });
}

/** Acknowledge an announcement (Story 6.2) */
export function useAcknowledgeAnnouncement() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) =>
      fetchJson<void>(`${ANNOUNCEMENTS_BASE}/${id}/acknowledge`, { method: 'POST' }),
    onSuccess: (_data, id) => {
      queryClient.invalidateQueries({ queryKey: announcementKeys.detail(id) });
      queryClient.invalidateQueries({ queryKey: announcementKeys.acknowledgments(id) });
    },
  });
}

/** Fetch acknowledgment stats for a given announcement (Story 6.2 — manager view) */
export function useAnnouncementAcknowledgmentStats(id: string, enabled = true) {
  return useQuery<import('./types').AcknowledgmentStatsResponse>({
    queryKey: announcementKeys.acknowledgments(id),
    queryFn: () =>
      fetchJson(`${ANNOUNCEMENTS_BASE}/${id}/acknowledgments`),
    enabled: enabled && !!id,
  });
}

// ============================================================================
// Standalone comment hooks — Story 6.3: Announcement Comments & Discussion
// ============================================================================

/** List comments for an announcement */
export function useAnnouncementComments(
  announcementId: string,
  params?: import('./types').ListCommentsParams,
  enabled = true
) {
  return useQuery<import('./types').CommentsResponse>({
    queryKey: announcementKeys.comments(announcementId, params),
    queryFn: () => {
      const sp = new URLSearchParams();
      if (params?.limit) sp.set('limit', params.limit.toString());
      if (params?.offset) sp.set('offset', params.offset.toString());
      const qs = sp.toString();
      const url = qs
        ? `${ANNOUNCEMENTS_BASE}/${announcementId}/comments?${qs}`
        : `${ANNOUNCEMENTS_BASE}/${announcementId}/comments`;
      return fetchJson<import('./types').CommentsResponse>(url);
    },
    enabled: enabled && !!announcementId,
  });
}

/** Create a comment on an announcement */
export function useCreateAnnouncementComment() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({
      announcementId,
      content,
      parentId,
      aiTrainingConsent,
    }: {
      announcementId: string;
      content: string;
      parentId?: string;
      aiTrainingConsent?: boolean;
    }) =>
      fetchJson<import('./types').AnnouncementComment>(
        `${ANNOUNCEMENTS_BASE}/${announcementId}/comments`,
        {
          method: 'POST',
          body: JSON.stringify({ content, parentId, aiTrainingConsent }),
        }
      ),
    onSuccess: (_, { announcementId }) => {
      queryClient.invalidateQueries({ queryKey: announcementKeys.comments(announcementId) });
      queryClient.invalidateQueries({ queryKey: announcementKeys.detail(announcementId) });
    },
  });
}

/** Delete (soft-delete) a comment on an announcement */
export function useDeleteAnnouncementComment() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({
      announcementId,
      commentId,
      reason,
    }: {
      announcementId: string;
      commentId: string;
      reason?: string;
    }) =>
      fetchJson<import('./types').AnnouncementComment>(
        `${ANNOUNCEMENTS_BASE}/${announcementId}/comments/${commentId}`,
        {
          method: 'DELETE',
          ...(reason ? { body: JSON.stringify({ reason }) } : {}),
        }
      ),
    onSuccess: (_, { announcementId }) => {
      queryClient.invalidateQueries({ queryKey: announcementKeys.comments(announcementId) });
      queryClient.invalidateQueries({ queryKey: announcementKeys.detail(announcementId) });
    },
  });
}
