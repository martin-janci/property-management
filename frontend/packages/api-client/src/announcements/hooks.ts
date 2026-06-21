/**
 * Announcement TanStack Query Hooks
 *
 * React hooks for managing announcements with server state caching.
 */

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { authenticatedFetchJson } from '../lib/fetch';
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
// Standalone hooks share a centralized authenticated fetch helper (see #486).
// `authenticatedFetchJson` lives in `../lib/fetch.ts` so any future change to
// token handling / 401 refresh / telemetry happens in one place instead of
// being duplicated per hooks module.
// ============================================================================

const ANNOUNCEMENTS_BASE = '/api/v1/announcements';

const fetchJson = authenticatedFetchJson;

/**
 * Default page size used when a caller passes `page` but no explicit
 * `pageSize`, so `page`→`offset` translation has a stable basis.
 */
const DEFAULT_PAGE_SIZE = 10;

/**
 * Build the announcement list query string against the *backend* contract.
 *
 * The backend (`ListAnnouncementsQuery`) expects `limit` / `offset` /
 * `target_type` / `from_date` / `to_date` (snake_case, limit/offset paging),
 * not the UI-friendly `page` / `pageSize` / `targetType` / `fromDate` /
 * `toDate` we expose on `ListAnnouncementsParams`. Previously these were sent
 * verbatim, so the server ignored every filter and paging param (#751).
 */
export function buildAnnouncementQuery(params?: ListAnnouncementsParams): string {
  if (!params) return '';
  const sp = new URLSearchParams();
  const pageSize = params.pageSize ?? (params.page ? DEFAULT_PAGE_SIZE : undefined);
  if (pageSize !== undefined) sp.set('limit', pageSize.toString());
  if (params.page && pageSize !== undefined) {
    // 1-based page → 0-based offset
    const offset = (Math.max(1, params.page) - 1) * pageSize;
    sp.set('offset', offset.toString());
  }
  if (params.status) sp.set('status', params.status);
  if (params.targetType) sp.set('target_type', params.targetType);
  if (params.authorId) sp.set('author_id', params.authorId);
  if (params.pinned !== undefined) sp.set('pinned', params.pinned.toString());
  if (params.fromDate) sp.set('from_date', params.fromDate);
  if (params.toDate) sp.set('to_date', params.toDate);
  const q = sp.toString();
  return q ? `?${q}` : '';
}

/**
 * Raw backend list response shape (`AnnouncementListResponse`): announcements
 * are serialized snake_case (no `serde(rename_all = "camelCase")` on
 * `AnnouncementSummary`) and pagination is `{ count, total }` — not the
 * `{ items, page, pageSize, totalPages }` the UI consumes. We normalize here
 * so callers keep getting `PaginatedResponse<AnnouncementSummary>` (#751).
 */
interface BackendAnnouncementSummary {
  id: string;
  title: string;
  status: AnnouncementSummary['status'];
  target_type: AnnouncementSummary['targetType'];
  published_at?: string | null;
  pinned: boolean;
  comments_enabled: boolean;
  acknowledgment_required: boolean;
}

export interface BackendAnnouncementListResponse {
  announcements: BackendAnnouncementSummary[];
  count: number;
  total: number;
}

function mapSummary(s: BackendAnnouncementSummary): AnnouncementSummary {
  return {
    id: s.id,
    title: s.title,
    status: s.status,
    targetType: s.target_type,
    publishedAt: s.published_at ?? undefined,
    pinned: s.pinned,
    commentsEnabled: s.comments_enabled,
    acknowledgmentRequired: s.acknowledgment_required,
  };
}

/**
 * Normalize a raw backend list response into the UI's `PaginatedResponse`.
 * `page` / `pageSize` echo the request params (the backend reports neither);
 * `totalPages` is derived from `total` and the effective page size.
 */
export function toPaginatedResponse(
  raw: BackendAnnouncementListResponse,
  params?: ListAnnouncementsParams
): PaginatedResponse<AnnouncementSummary> {
  const pageSize = params?.pageSize ?? DEFAULT_PAGE_SIZE;
  const page = params?.page ?? 1;
  return {
    items: (raw.announcements ?? []).map(mapSummary),
    total: raw.total ?? 0,
    page,
    pageSize,
    totalPages: pageSize > 0 ? Math.ceil((raw.total ?? 0) / pageSize) : 0,
  };
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
    queryFn: async () => {
      const raw = await fetchJson<BackendAnnouncementListResponse>(
        `${ANNOUNCEMENTS_BASE}${buildAnnouncementQuery(params)}`
      );
      return toPaginatedResponse(raw, params);
    },
  });
}

/**
 * Fetch only pinned published announcements (Story 6.4).
 *
 * Uses a fixed `pinned: true` filter so the query key is stable and won't
 * collide with the main list cache. Refreshes every 5 minutes — pinned items
 * change infrequently and do not need a tight polling loop.
 */
export function usePinnedAnnouncements() {
  const params: ListAnnouncementsParams = { pinned: true, status: 'published', pageSize: 20 };
  return useQuery<PaginatedResponse<AnnouncementSummary>>({
    queryKey: announcementKeys.list(params),
    queryFn: async () => {
      const raw = await fetchJson<BackendAnnouncementListResponse>(
        `${ANNOUNCEMENTS_BASE}${buildAnnouncementQuery(params)}`
      );
      return toPaginatedResponse(raw, params);
    },
    staleTime: 5 * 60 * 1000,
  });
}

/** Create a new announcement (draft) */
export function useCreateAnnouncement() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (data: CreateAnnouncementRequest) =>
      fetchJson<{ id: string; message: string }>(ANNOUNCEMENTS_BASE, {
        method: 'POST',
        body: JSON.stringify(data),
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: announcementKeys.lists() });
      queryClient.invalidateQueries({ queryKey: announcementKeys.statistics() });
    },
  });
}

/** Update an announcement (draft/scheduled only) */
export function useUpdateAnnouncement(id: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (data: UpdateAnnouncementRequest) =>
      fetchJson<{ message: string; announcement: Announcement }>(`${ANNOUNCEMENTS_BASE}/${id}`, {
        method: 'PUT',
        body: JSON.stringify(data),
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: announcementKeys.detail(id) });
      queryClient.invalidateQueries({ queryKey: announcementKeys.lists() });
      queryClient.invalidateQueries({ queryKey: announcementKeys.published() });
    },
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
  return useQuery<{
    announcement: import('./types').AnnouncementWithDetails;
    attachments: import('./types').AnnouncementAttachment[];
  }>({
    queryKey: announcementKeys.detail(id),
    queryFn: () => fetchJson(`${ANNOUNCEMENTS_BASE}/${id}`),
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
      // Invalidate list cache so unread badges update immediately (Story 6.2 fix)
      queryClient.invalidateQueries({ queryKey: announcementKeys.lists() });
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
    queryFn: () => fetchJson(`${ANNOUNCEMENTS_BASE}/${id}/acknowledgments`),
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

/**
 * Variables accepted by {@link useCreateAnnouncementComment}.
 *
 * `optimisticAuthorName` is UI-only: when supplied, the mutation inserts a
 * temporary placeholder comment into the cache immediately (optimistic add)
 * so the thread feels instant. It is never sent to the server.
 */
export interface CreateAnnouncementCommentVars {
  announcementId: string;
  content: string;
  parentId?: string;
  aiTrainingConsent?: boolean;
  /** Display name for the optimistic placeholder. Omit to skip the optimistic insert. */
  optimisticAuthorName?: string;
  /** Current user id, stamped on the optimistic placeholder so delete affordances render. */
  optimisticUserId?: string;
}

type CommentWithAuthor = import('./types').CommentWithAuthor;
type CommentsResponse = import('./types').CommentsResponse;

/** Stable prefix for client-generated optimistic comment ids. */
export const OPTIMISTIC_COMMENT_PREFIX = 'optimistic-';

/**
 * Insert a (top-level or nested) comment into a comments tree immutably.
 * When `parentId` is set, the comment is appended to that parent's `replies`;
 * otherwise it is appended at the top level.
 */
export function insertCommentIntoTree(
  comments: CommentWithAuthor[],
  comment: CommentWithAuthor
): CommentWithAuthor[] {
  if (!comment.parentId) {
    return [...comments, comment];
  }
  return comments.map((c) => {
    if (c.id === comment.parentId) {
      return { ...c, replies: [...(c.replies ?? []), comment] };
    }
    if (c.replies && c.replies.length > 0) {
      return { ...c, replies: insertCommentIntoTree(c.replies, comment) };
    }
    return c;
  });
}

/** Create a comment on an announcement, with optimistic add + rollback. */
export function useCreateAnnouncementComment() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({
      announcementId,
      content,
      parentId,
      aiTrainingConsent,
    }: CreateAnnouncementCommentVars) =>
      fetchJson<import('./types').AnnouncementComment>(
        `${ANNOUNCEMENTS_BASE}/${announcementId}/comments`,
        {
          method: 'POST',
          body: JSON.stringify({ content, parentId, aiTrainingConsent }),
        }
      ),
    onMutate: async (vars) => {
      // Optimistic insert is opt-in: callers without an author name (e.g.
      // contract tests) keep the plain invalidate-on-success behaviour.
      if (!vars.optimisticAuthorName) return { key: undefined };
      const key = announcementKeys.comments(vars.announcementId, undefined);
      // Cancel in-flight refetches so they can't clobber our optimistic write.
      await queryClient.cancelQueries({ queryKey: key });
      const previous = queryClient.getQueryData<CommentsResponse>(key);
      const now = new Date().toISOString();
      const optimistic: CommentWithAuthor = {
        id: `${OPTIMISTIC_COMMENT_PREFIX}${now}-${Math.random().toString(36).slice(2)}`,
        announcementId: vars.announcementId,
        userId: vars.optimisticUserId ?? '',
        parentId: vars.parentId,
        content: vars.content,
        authorName: vars.optimisticAuthorName,
        isDeleted: false,
        createdAt: now,
        updatedAt: now,
        replies: [],
      };
      const base: CommentsResponse = previous ?? { comments: [], count: 0, total: 0 };
      queryClient.setQueryData<CommentsResponse>(key, {
        ...base,
        comments: insertCommentIntoTree(base.comments, optimistic),
        count: base.count + 1,
        total: base.total + 1,
      });
      return { key, previous };
    },
    onError: (_err, _vars, context) => {
      // Roll the cache back to the pre-mutation snapshot.
      if (context?.key) {
        queryClient.setQueryData(context.key, context.previous);
      }
    },
    onSettled: (_data, _err, { announcementId }) => {
      // Reconcile with the server (replaces the placeholder with the real row).
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
