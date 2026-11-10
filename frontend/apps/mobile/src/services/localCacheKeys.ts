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

// `useDashboardLayout` / layout registry — server-driven resolved screen layout,
// persisted across restarts so the last-known layout activates at launch before
// the background fetch completes (next-launch activation pattern).
//
// The resolved layout embeds the org's tenant override (which sections are
// hidden/reordered — the org-admin's customization), so — like
// `WIDGET_CONFIG_KEY` above — it is tenant-scoped data. It MUST therefore be
// (a) keyed by the logged-in user/org id so a foreign tenant can never read a
// prior tenant's layout, and (b) purged on session change. Because the exact
// key is a *function* of a dynamic `screen` (and now a dynamic `scopeId`),
// `resetLocalData` sweeps every key under `LAYOUT_PREFIX` rather than a fixed
// list (issue #2486, follow-up to PR #2432 / #2399).
export const LAYOUT_PREFIX = 'ppt_layout_';
export const LAYOUT_CACHE_KEY = (scopeId: string, screen: string) =>
  `${LAYOUT_PREFIX}${scopeId}_${screen.replace(/\//g, '_')}`;

// Feature caches that persist tenant-scoped history/queues across restarts and
// so must also be purged on a session change (issue #2947, follow-up to
// #2361/#2399). The owning managers (`NFCCredentialManager`, `QRCodeScanner`,
// `FeedbackManager`, `HelpCenter`) keep the reads/writes behind these keys; the
// literals are mirrored here so the purge set stays a single registry and a new
// feature cache is covered by adding it here rather than by editing
// `resetLocalData`.
//
// - `@ppt/access_log`     — NFC building-access log (a prior tenant's physical
//   access history would otherwise survive a handoff on a shared device).
// - `@ppt/qr_scan_history`— QR scan history.
// - `@ppt/feedback_drafts`/`@ppt/pending_feedback` — offline feedback drafts +
//   the pending-flush queue, which would otherwise replay under the NEXT user's
//   authenticated token (write under the wrong identity).
// - `@ppt/faq_votes`      — per-user FAQ vote record.
export const ACCESS_LOG_KEY = '@ppt/access_log';
export const QR_SCAN_HISTORY_KEY = '@ppt/qr_scan_history';
export const FEEDBACK_DRAFTS_KEY = '@ppt/feedback_drafts';
export const PENDING_FEEDBACK_KEY = '@ppt/pending_feedback';
export const FAQ_VOTES_KEY = '@ppt/faq_votes';

/**
 * Every fixed (non-prefix) tenant-scoped key `resetLocalData` deletes by exact
 * match. Prefix-swept namespaces (`CACHE_PREFIX`, `LAYOUT_PREFIX`) are handled
 * separately because their concrete keys embed a dynamic suffix.
 */
export const TENANT_SCOPED_EXACT_KEYS: readonly string[] = [
  QUEUE_KEY,
  LAST_SYNC_KEY,
  WIDGET_CONFIG_KEY,
  WIDGET_DATA_KEY,
  ACCESS_LOG_KEY,
  QR_SCAN_HISTORY_KEY,
  FEEDBACK_DRAFTS_KEY,
  PENDING_FEEDBACK_KEY,
  FAQ_VOTES_KEY,
];
