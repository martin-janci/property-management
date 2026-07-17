/**
 * Single source of truth for the AsyncStorage key namespaces that hold
 * tenant-scoped local data (issue #2399).
 *
 * These caches persist across app restarts, so — unlike TanStack Query's
 * in-memory cache — they must be purged explicitly whenever the session
 * changes, or a prior org's data survives a login/logout (the exact
 * cross-tenant stale-data exposure #2361 set out to close). `resetLocalData`
 * consumes this list; `useOfflineSupport` and `WidgetDataProvider` own the
 * reads/writes behind it, so keeping the keys here keeps all three in step.
 */

// `useOfflineSupport` — cached API responses (`ppt_cache_<key>`), the queued
// offline mutation log, and the last-sync marker.
export const CACHE_PREFIX = 'ppt_cache_';
export const QUEUE_KEY = 'ppt_offline_queue';
export const LAST_SYNC_KEY = 'ppt_last_sync';

// `WidgetDataProvider` — per-building home-screen widget content and the
// widget layout config (which references building ids, i.e. tenant data).
export const WIDGET_CONFIG_KEY = '@ppt/widget_configs';
export const WIDGET_DATA_KEY = '@ppt/widget_data';
