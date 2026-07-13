/**
 * Notification trigger management feature barrel (Story 84.4 — Notification
 * Trigger System, PM gap 84-4). Manage which event types notify the user and on
 * which channels, over the granular `/notification-preferences/granular/events` API.
 */

export * from './hooks/useNotificationTriggers';
export * from './pages';
export * from './types';
