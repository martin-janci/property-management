/**
 * Notification trigger management types (Story 84.4 — Notification Trigger System,
 * PM gap 84-4).
 *
 * Mirrors the granular notification-event API shipped by the backend (Epic 8B,
 * `backend/crates/db/src/models/granular_notification.rs`) at base
 * `/api/v1/users/me/notification-preferences/granular`. These per-event-type
 * preferences ARE the trigger surface: each event type (trigger) fans out to the
 * push / email / in-app channels according to the flags below.
 *
 * The granular endpoints are NOT in the generated `@ppt/api-client`, so — as with
 * `features/notification-analytics` — these types are hand-authored alongside a
 * direct-REST hook. JSON is camelCase (backend `#[serde(rename_all = "camelCase")]`).
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
