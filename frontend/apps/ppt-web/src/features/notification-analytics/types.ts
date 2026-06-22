/**
 * Notification delivery analytics types (Story 2B-C.3, PM #969 gap 4 / BIT-214).
 *
 * Mirrors the `GET /api/v1/admin/notifications/analytics` response shape shipped
 * by the backend (BIT-180, PR #1710). The endpoint is capability-gated
 * (`audit_read`) and is NOT in the generated `@ppt/api-client`, so these types
 * are hand-authored alongside the direct-REST hook (the `admin/audit` precedent).
 */

/** Channels the operator can filter by (the `channel` query param). */
export type NotificationChannel = 'push' | 'email' | 'in_app';

/** Relative time-window aliases accepted by the `after` query param. */
export type AnalyticsWindow = '15m' | '24h' | '7d' | '30d';

/**
 * Per-channel (or aggregate) delivery counts. `channel` is `"all"` on the
 * `totals` row, otherwise one of {@link NotificationChannel}.
 */
export interface ChannelAnalytics {
  channel: string;
  sent: number;
  delivered: number;
  failed: number;
  skipped: number;
  opened: number;
  clicked: number;
  /** `sent + failed` — deliveries that resolved to success or failure. */
  attempted: number;
  /** `failed / attempted` in `[0,1]`; `0` for an empty window. */
  failure_rate: number;
}

/** The >5% failure-rate alert state for the window. */
export interface FailureRateAlert {
  /** `true` when `attempted >= min_attempts` and `failure_rate > threshold`. */
  firing: boolean;
  threshold: number;
  failure_rate: number;
  attempted: number;
  min_attempts: number;
}

/** Full analytics response for a window. */
export interface NotificationAnalyticsResponse {
  from: string;
  to: string;
  /** Echoes the `channel` filter, if any. */
  channel_filter: string | null;
  /** Aggregate across all channels in the window. */
  totals: ChannelAnalytics;
  /** One row per channel that had events in the window. */
  by_channel: ChannelAnalytics[];
  alert: FailureRateAlert;
}

/** Operator-selected filters that drive the analytics query. */
export interface NotificationAnalyticsFilters {
  window: AnalyticsWindow;
  channel?: NotificationChannel;
}
