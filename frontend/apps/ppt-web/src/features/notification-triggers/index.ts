/**
 * Notification trigger management feature barrel (Story 84.4 — Notification
 * Trigger System, PM gap 84-4). Manage which event types notify the user and on
 * which channels, over the granular `/notification-preferences/granular/events` API.
 *
 * The hooks + types now live in `@ppt/api-client` (`granular-notifications`) on
 * the shared `authenticatedFetchJson` helper; they are re-exported here so
 * existing feature-relative imports keep working. The presentational page stays
 * local to the app.
 */

export type {
  CategorySummary,
  EventTriggerPreference,
  EventTriggersResponse,
  NotificationEventCategory,
  TriggerChannel,
  UpdateTriggerRequest,
} from '@ppt/api-client';
export {
  notificationTriggerKeys,
  useNotificationTriggers,
  useResetNotificationTriggers,
  useUpdateNotificationTrigger,
} from '@ppt/api-client';
export * from './pages';
