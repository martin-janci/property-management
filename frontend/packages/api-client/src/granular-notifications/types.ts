/**
 * Granular notification-trigger types (Story 84.4 — Notification Trigger System,
 * PM gap 84-4).
 *
 * Mirrors the granular notification-event API shipped by the backend (Epic 8B,
 * `backend/crates/db/src/models/granular_notification.rs`) at base
 * `/api/v1/users/me/notification-preferences/granular`. These per-event-type
 * preferences ARE the trigger surface: each event type (trigger) fans out to the
 * push / email / in-app channels according to the flags below.
 *
 * The granular endpoints are not catalogued in `@ppt/sitemap` and are absent from
 * the generated client, so these types are hand-authored alongside a fetch built
 * on the shared `authenticatedFetchJson` helper (see `../lib/fetch`, #486). JSON
 * is camelCase (backend `#[serde(rename_all = "camelCase")]`).
 */

/** Delivery channels a trigger can fan out to. */
export type TriggerChannel = 'push' | 'email' | 'inApp';

/** Event categories the backend groups triggers under (snake_case on the wire). */
export type NotificationEventCategory =
  | 'fault'
  | 'vote'
  | 'announcement'
  | 'document'
  | 'message'
  | 'critical'
  | 'finance'
  | 'facility';

/**
 * A single trigger (event type) with its per-channel delivery flags.
 * Mirrors `EventPreferenceWithDetails`.
 */
export interface EventTriggerPreference {
  eventType: string;
  category: NotificationEventCategory;
  displayName: string;
  description?: string;
  /** Priority triggers (e.g. emergencies) cannot be fully silenced. */
  isPriority: boolean;
  pushEnabled: boolean;
  emailEnabled: boolean;
  inAppEnabled: boolean;
  updatedAt?: string;
}

/** Per-category rollup. Mirrors `CategorySummary`. */
export interface CategorySummary {
  category: NotificationEventCategory;
  displayName: string;
  totalEvents: number;
  enabledEvents: number;
}

/** Full granular-events response. Mirrors `EventPreferencesResponse`. */
export interface EventTriggersResponse {
  preferences: EventTriggerPreference[];
  categories: CategorySummary[];
}

/** Partial channel update for a single trigger. Mirrors `UpdateEventPreferenceRequest`. */
export interface UpdateTriggerRequest {
  pushEnabled?: boolean;
  emailEnabled?: boolean;
  inAppEnabled?: boolean;
}

// ---------------------------------------------------------------------------
// In-app notification inbox (Epic 29 notification groups).
//
// In-app notifications are written by the backend NotificationService into
// `notification_groups` / `grouped_notifications`; these types mirror
// `NotificationGroup`, `GroupedNotification` and `GroupedNotificationsResponse`
// in `backend/crates/db/src/models/granular_notification.rs` (camelCase JSON).
// ---------------------------------------------------------------------------

/** One notification inside a group. */
export interface InboxNotification {
  id: string;
  groupId: string;
  userId: string;
  eventType: string;
  title: string;
  body?: string;
  data?: Record<string, unknown>;
  actorId?: string;
  actorName?: string;
  isRead: boolean;
  readAt?: string;
  createdAt: string;
}

/** A group of notifications about one entity (e.g. one fault). */
export interface InboxGroup {
  id: string;
  userId: string;
  groupKey: string;
  entityType: string;
  entityId: string;
  title: string;
  notificationCount: number;
  latestNotificationAt: string;
  isRead: boolean;
  readAt?: string;
  isExpanded: boolean;
  createdAt: string;
  updatedAt: string;
}

/** A group with its newest (up to 5) notifications. */
export interface InboxGroupWithNotifications {
  group: InboxGroup;
  notifications: InboxNotification[];
}

/** `GET .../granular/groups` response. */
export interface InboxResponse {
  groups: InboxGroupWithNotifications[];
  totalUnread: number;
}

/** Query for the inbox list. */
export interface InboxQuery {
  includeRead?: boolean;
  limit?: number;
  offset?: number;
}
