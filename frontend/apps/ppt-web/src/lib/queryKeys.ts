/**
 * Query Key Factory Functions
 *
 * Centralized query key management for TanStack Query.
 * Follows the factory pattern for consistent cache management (Story 79.1).
 *
 * @see https://tanstack.com/query/latest/docs/react/guides/query-keys
 */

// ============================================================================
// Filter Types
// ============================================================================

/**
 * Common pagination parameters.
 */
export interface PaginationParams {
  page?: number;
  pageSize?: number;
}

/**
 * Announcement list filters.
 */
export interface AnnouncementFilters extends PaginationParams {
  status?: 'draft' | 'scheduled' | 'published' | 'archived';
  targetType?: 'all' | 'building' | 'units' | 'roles';
  authorId?: string;
  pinned?: boolean;
  fromDate?: string;
  toDate?: string;
}

/**
 * Fault list filters.
 */
export interface FaultFilters extends PaginationParams {
  status?: 'open' | 'in_progress' | 'resolved' | 'closed';
  priority?: 'low' | 'medium' | 'high' | 'urgent';
  category?: string;
  assigneeId?: string;
  reporterId?: string;
  buildingId?: string;
  fromDate?: string;
  toDate?: string;
}

/**
 * Document list filters.
 */
export interface DocumentFilters extends PaginationParams {
  folderId?: string;
  category?: string;
  search?: string;
  ocrStatus?: 'pending' | 'processing' | 'completed' | 'failed' | 'not_applicable';
  hasSummary?: boolean;
}

/**
 * Document search filters.
 */
export interface DocumentSearchFilters {
  query: string;
  categories?: string[];
  dateFrom?: string;
  dateTo?: string;
  limit?: number;
  offset?: number;
}

/**
 * Vote list filters.
 */
export interface VoteFilters extends PaginationParams {
  status?: 'draft' | 'active' | 'closed' | 'cancelled';
  type?: 'binary' | 'multiple_choice' | 'ranked' | 'weighted';
  buildingId?: string;
  fromDate?: string;
  toDate?: string;
}

/**
 * Message thread list filters.
 */
export interface MessageFilters extends PaginationParams {
  unreadOnly?: boolean;
}

/**
 * Neighbor list filters.
 */
export interface NeighborFilters extends PaginationParams {
  buildingId?: string;
  floor?: number;
  search?: string;
}

/**
 * Form list filters.
 */
export interface FormFilters extends PaginationParams {
  status?: 'draft' | 'published' | 'archived';
  category?: string;
  search?: string;
}

/**
 * Form submission filters.
 */
export interface FormSubmissionFilters extends PaginationParams {
  status?: 'pending' | 'reviewed' | 'approved' | 'rejected';
  userId?: string;
  fromDate?: string;
  toDate?: string;
}

// ============================================================================
// Query Keys Factory
// ============================================================================

/**
 * Centralized query keys for all API resources.
 *
 * Usage:
 * ```typescript
 * // In a query hook
 * useQuery({
 *   queryKey: queryKeys.announcements.list({ status: 'published' }),
 *   queryFn: () => api.announcements.list({ status: 'published' }),
 * });
 *
 * // For cache invalidation
 * queryClient.invalidateQueries({
 *   queryKey: queryKeys.announcements.all,
 * });
 * ```
 */
export const queryKeys = {
  // ========================================================================
  // Accounting (native accounting MVP — PAP-232)
  // ========================================================================
  accounting: {
    all: ['accounting'] as const,
    invoices: () => [...queryKeys.accounting.all, 'invoices'] as const,
    contacts: () => [...queryKeys.accounting.all, 'contacts'] as const,
    statements: () => [...queryKeys.accounting.all, 'statements'] as const,
    statementLines: (statementId: string) =>
      [...queryKeys.accounting.all, 'statements', statementId, 'lines'] as const,
    lineMatches: (lineId: string) =>
      [...queryKeys.accounting.all, 'lines', lineId, 'matches'] as const,
  },

  // ========================================================================
  // Announcements (UC-06)
  // ========================================================================
  announcements: {
    all: ['announcements'] as const,
    lists: () => [...queryKeys.announcements.all, 'list'] as const,
    list: (filters?: AnnouncementFilters) => [...queryKeys.announcements.lists(), filters] as const,
    published: () => [...queryKeys.announcements.all, 'published'] as const,
    details: () => [...queryKeys.announcements.all, 'detail'] as const,
    detail: (id: string) => [...queryKeys.announcements.details(), id] as const,
    attachments: (id: string) => [...queryKeys.announcements.detail(id), 'attachments'] as const,
    acknowledgments: (id: string) =>
      [...queryKeys.announcements.detail(id), 'acknowledgments'] as const,
    comments: (id: string) => [...queryKeys.announcements.detail(id), 'comments'] as const,
    statistics: () => [...queryKeys.announcements.all, 'statistics'] as const,
    unreadCount: () => [...queryKeys.announcements.all, 'unread-count'] as const,
  },

  // ========================================================================
  // Faults (UC-09)
  // ========================================================================
  faults: {
    all: ['faults'] as const,
    lists: () => [...queryKeys.faults.all, 'list'] as const,
    list: (filters?: FaultFilters) => [...queryKeys.faults.lists(), filters] as const,
    details: () => [...queryKeys.faults.all, 'detail'] as const,
    detail: (id: string) => [...queryKeys.faults.details(), id] as const,
    comments: (id: string) => [...queryKeys.faults.detail(id), 'comments'] as const,
    attachments: (id: string) => [...queryKeys.faults.detail(id), 'attachments'] as const,
    history: (id: string) => [...queryKeys.faults.detail(id), 'history'] as const,
    statistics: () => [...queryKeys.faults.all, 'statistics'] as const,
    categories: () => [...queryKeys.faults.all, 'categories'] as const,
  },

  // ========================================================================
  // Documents (UC-10, Epic 28, Epic 39)
  // ========================================================================
  documents: {
    all: ['documents'] as const,
    lists: () => [...queryKeys.documents.all, 'list'] as const,
    list: (filters?: DocumentFilters) => [...queryKeys.documents.lists(), filters] as const,
    details: () => [...queryKeys.documents.all, 'detail'] as const,
    detail: (id: string) => [...queryKeys.documents.details(), id] as const,
    download: (id: string) => [...queryKeys.documents.detail(id), 'download'] as const,
    folders: () => [...queryKeys.documents.all, 'folders'] as const,
    folder: (id: string) => [...queryKeys.documents.folders(), id] as const,
    folderTree: () => [...queryKeys.documents.folders(), 'tree'] as const,
    search: (filters: DocumentSearchFilters) =>
      [...queryKeys.documents.all, 'search', filters] as const,
    intelligenceStats: () => [...queryKeys.documents.all, 'intelligence-stats'] as const,
    categories: () => [...queryKeys.documents.all, 'categories'] as const,
  },

  // ========================================================================
  // Voting (UC-07)
  // ========================================================================
  votes: {
    all: ['votes'] as const,
    lists: () => [...queryKeys.votes.all, 'list'] as const,
    list: (filters?: VoteFilters) => [...queryKeys.votes.lists(), filters] as const,
    active: () => [...queryKeys.votes.all, 'active'] as const,
    details: () => [...queryKeys.votes.all, 'detail'] as const,
    detail: (id: string) => [...queryKeys.votes.details(), id] as const,
    results: (id: string) => [...queryKeys.votes.detail(id), 'results'] as const,
    myVote: (id: string) => [...queryKeys.votes.detail(id), 'my-vote'] as const,
    statistics: () => [...queryKeys.votes.all, 'statistics'] as const,
  },

  // ========================================================================
  // Messages (Story 6.5)
  // ========================================================================
  messages: {
    all: ['messages'] as const,
    threads: () => [...queryKeys.messages.all, 'threads'] as const,
    threadList: (filters?: MessageFilters) => [...queryKeys.messages.threads(), filters] as const,
    thread: (id: string) => [...queryKeys.messages.threads(), id] as const,
    threadMessages: (id: string) => [...queryKeys.messages.thread(id), 'messages'] as const,
    unreadCount: () => [...queryKeys.messages.all, 'unread-count'] as const,
    blockedUsers: () => [...queryKeys.messages.all, 'blocked-users'] as const,
  },

  // ========================================================================
  // Neighbors (Story 6.6)
  // ========================================================================
  neighbors: {
    all: ['neighbors'] as const,
    lists: () => [...queryKeys.neighbors.all, 'list'] as const,
    list: (filters?: NeighborFilters) => [...queryKeys.neighbors.lists(), filters] as const,
    detail: (userId: string) => [...queryKeys.neighbors.all, 'detail', userId] as const,
    privacySettings: () => [...queryKeys.neighbors.all, 'privacy-settings'] as const,
  },

  // ========================================================================
  // Forms (Epic 54)
  // ========================================================================
  forms: {
    all: ['forms'] as const,
    lists: () => [...queryKeys.forms.all, 'list'] as const,
    list: (filters?: FormFilters) => [...queryKeys.forms.lists(), filters] as const,
    available: () => [...queryKeys.forms.all, 'available'] as const,
    details: () => [...queryKeys.forms.all, 'detail'] as const,
    detail: (id: string) => [...queryKeys.forms.details(), id] as const,
    fields: (id: string) => [...queryKeys.forms.detail(id), 'fields'] as const,
    submissions: (id: string, filters?: FormSubmissionFilters) =>
      [...queryKeys.forms.detail(id), 'submissions', filters] as const,
    submission: (formId: string, submissionId: string) =>
      [...queryKeys.forms.detail(formId), 'submission', submissionId] as const,
    mySubmissions: () => [...queryKeys.forms.all, 'my-submissions'] as const,
    statistics: () => [...queryKeys.forms.all, 'statistics'] as const,
  },

  // ========================================================================
  // Person Months / Self Readings (UC-13)
  // ========================================================================
  personMonths: {
    all: ['person-months'] as const,
    lists: () => [...queryKeys.personMonths.all, 'list'] as const,
    list: (filters?: PaginationParams & { year?: number; month?: number }) =>
      [...queryKeys.personMonths.lists(), filters] as const,
    current: () => [...queryKeys.personMonths.all, 'current'] as const,
    detail: (id: string) => [...queryKeys.personMonths.all, 'detail', id] as const,
    history: (unitId: string) => [...queryKeys.personMonths.all, 'history', unitId] as const,
  },

  selfReadings: {
    all: ['self-readings'] as const,
    lists: () => [...queryKeys.selfReadings.all, 'list'] as const,
    list: (filters?: PaginationParams & { year?: number; month?: number; meterId?: string }) =>
      [...queryKeys.selfReadings.lists(), filters] as const,
    current: () => [...queryKeys.selfReadings.all, 'current'] as const,
    detail: (id: string) => [...queryKeys.selfReadings.all, 'detail', id] as const,
    meters: () => [...queryKeys.selfReadings.all, 'meters'] as const,
  },

  // ========================================================================
  // User / Auth
  // ========================================================================
  user: {
    all: ['user'] as const,
    current: () => [...queryKeys.user.all, 'current'] as const,
    profile: () => [...queryKeys.user.all, 'profile'] as const,
    settings: () => [...queryKeys.user.all, 'settings'] as const,
    notifications: () => [...queryKeys.user.all, 'notifications'] as const,
    units: () => [...queryKeys.user.all, 'units'] as const,
  },

  // ========================================================================
  // Buildings / Units
  // ========================================================================
  buildings: {
    all: ['buildings'] as const,
    lists: () => [...queryKeys.buildings.all, 'list'] as const,
    list: (filters?: PaginationParams & { organizationId?: string }) =>
      [...queryKeys.buildings.lists(), filters] as const,
    detail: (id: string) => [...queryKeys.buildings.all, 'detail', id] as const,
    units: (buildingId: string) => [...queryKeys.buildings.detail(buildingId), 'units'] as const,
    residents: (buildingId: string) =>
      [...queryKeys.buildings.detail(buildingId), 'residents'] as const,
  },

  // ========================================================================
  // Notifications
  // ========================================================================
  notifications: {
    all: ['notifications'] as const,
    lists: () => [...queryKeys.notifications.all, 'list'] as const,
    list: (filters?: PaginationParams & { unreadOnly?: boolean }) =>
      [...queryKeys.notifications.lists(), filters] as const,
    unreadCount: () => [...queryKeys.notifications.all, 'unread-count'] as const,
    preferences: () => [...queryKeys.notifications.all, 'preferences'] as const,
    critical: () => [...queryKeys.notifications.all, 'critical'] as const,
  },
} as const;

// ============================================================================
// Auth-Scoped Query Roots (used by AuthContext.logout)
// ============================================================================

/**
 * First-segment root keys of every query that is bound to the authenticated
 * session. On logout, `AuthContext` iterates this list and calls
 * `queryClient.removeQueries({ queryKey: [root] })` for each entry, scoping
 * the cache purge to user data while leaving any unrelated/non-session cache
 * (e.g. router-internal caches, future public/lookup queries) untouched.
 *
 * Covers the centralized {@link queryKeys} factory roots (including
 * `accounting`), the shared `@ppt/api-client` key-factory roots consumed by
 * ppt-web (`messages`, `meters`, `reports`), and the ad-hoc roots used
 * directly in feature hooks (`developer`, `ocr`, `actionQueue`,
 * `executionLogs`, `executionStats`, `ai-chat`, `predictive-maintenance`,
 * `sentiment`, `notification-analytics`, `notification-triggers`,
 * `financial`, `rentals`, and the `auth`-rooted active-sessions cache).
 *
 * When you add a new auth-scoped query root, add it here too — otherwise the
 * cached data will leak into the next user's session. This must cover ALL
 * tenant-/user-scoped caches; the financial roots (`accounting`, `financial`,
 * `reports`) are the most sensitive and must never survive logout on a shared
 * workstation.
 *
 * @see Issue #712 — logout `queryClient.clear()` was too aggressive
 */
export const AUTHED_QUERY_KEY_ROOTS = [
  // queryKeys factory roots
  'accounting',
  'announcements',
  'faults',
  'documents',
  'votes',
  'messages',
  'neighbors',
  'forms',
  'person-months',
  'self-readings',
  'user',
  'buildings',
  'notifications',
  // Shared @ppt/api-client key-factory roots consumed by ppt-web
  'meters',
  'reports',
  // Ad-hoc roots used directly in feature hooks
  'developer',
  'ocr',
  'actionQueue',
  'executionLogs',
  'executionStats',
  'ai-chat',
  'financial',
  'rentals',
  // Active-session management (SessionsPage — ['auth', 'sessions'])
  'auth',
  // Feature-local key factories (analytics dashboards — tenant-scoped)
  'predictive-maintenance',
  'sentiment',
  'notification-analytics',
  // Granular notification-trigger preferences (@ppt/api-client
  // granular-notifications — `notificationTriggerKeys`, Story 84.4). These are
  // per-user preference caches; without this root they leaked into the next
  // session (PR #2650 fix was incomplete — this root was missed).
  'notification-triggers',
] as const;

// ============================================================================
// Type Exports
// ============================================================================

/**
 * Type for all query keys.
 * Useful for type-safe query key references.
 */
export type QueryKeys = typeof queryKeys;
