/**
 * Granular Notification Triggers Module (Story 84.4 — Notification Trigger System,
 * PM gap 84-4).
 *
 * Per-event-type notification preferences over the granular
 * `/notification-preferences/granular/events` API. Sibling of the category-level
 * `advanced-notifications` module (Epic 40); both build on `authenticatedFetchJson`.
 */

export * from './api';
export * from './hooks';
export * from './types';
