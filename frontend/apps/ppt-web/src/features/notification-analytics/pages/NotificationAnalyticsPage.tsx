/**
 * NotificationAnalyticsPage — operator dashboard for notification delivery
 * (Story 2B-C.3, PM #969 gap 4 / BIT-214).
 *
 * Operator-only. Shows per-channel delivery/failure rates over a selectable
 * window, with a prominent banner when the >5% failure-rate alert is firing.
 * Channel + time-window filters map to the `channel` / `after` query params.
 *
 * Presentational: the filter state is owned by the route wrapper
 * (NotificationAnalyticsPageRoute), which drives `useNotificationAnalytics`.
 * `opened`/`clicked` columns render now (currently 0 — engagement tracking is a
 * later increment, BIT-180 follow-up).
 */

import { useTranslation } from 'react-i18next';
import type {
  AnalyticsWindow,
  ChannelAnalytics,
  NotificationAnalyticsFilters,
  NotificationAnalyticsResponse,
  NotificationChannel,
} from '../types';

export interface NotificationAnalyticsPageProps {
  /** Operator-equivalent access (mirrors isManagerRole). */
  isOperator: boolean;
  data?: NotificationAnalyticsResponse;
  isLoading?: boolean;
  isError?: boolean;
  /** True when the backend rejected the request with 403 (missing capability). */
  isForbidden?: boolean;
  onRetry?: () => void;
  filters: NotificationAnalyticsFilters;
  onFiltersChange: (filters: NotificationAnalyticsFilters) => void;
}

const WINDOW_OPTIONS: AnalyticsWindow[] = ['15m', '24h', '7d', '30d'];
const CHANNEL_OPTIONS: NotificationChannel[] = ['push', 'email', 'in_app'];

/** Humanise a channel key for display (`in_app` → `In app`, `all` → `All`). */
function humaniseChannel(value: string): string {
  const s = value.replace(/_/g, ' ');
  return s.charAt(0).toUpperCase() + s.slice(1);
}

/** Render a `[0,1]` rate as a percentage with one decimal (`0.08` → `8.0%`). */
function formatRate(rate: number): string {
  return `${(rate * 100).toFixed(1)}%`;
}

export function NotificationAnalyticsPage({
  isOperator,
  data,
  isLoading,
  isError,
  isForbidden,
  onRetry,
  filters,
  onFiltersChange,
}: NotificationAnalyticsPageProps) {
  const { t } = useTranslation();

  if (!isOperator || isForbidden) {
    return (
      <div className="max-w-5xl mx-auto px-4 py-8">
        <div
          role="alert"
          className="border border-amber-200 bg-amber-50 text-amber-800 rounded-lg p-6 text-center"
        >
          <p className="font-medium">
            {t('notificationAnalytics.forbidden', {
              defaultValue: 'You do not have permission to view notification delivery analytics.',
            })}
          </p>
        </div>
      </div>
    );
  }

  const alert = data?.alert;
  const rows: ChannelAnalytics[] = data?.by_channel ?? [];

  return (
    <div className="max-w-5xl mx-auto px-4 py-8">
      <header className="mb-6">
        <h1 className="text-2xl font-bold text-gray-900">
          {t('notificationAnalytics.title', { defaultValue: 'Notification Analytics' })}
        </h1>
        <p className="text-sm text-gray-500">
          {t('notificationAnalytics.subtitle', {
            defaultValue: 'Delivery and failure rates per notification channel.',
          })}
        </p>
      </header>

      {/* >5% failure-rate alert banner */}
      {alert?.firing && (
        <div
          role="alert"
          className="mb-6 flex items-start gap-3 border border-red-300 bg-red-50 text-red-800 rounded-lg p-4"
        >
          <span aria-hidden="true" className="text-xl leading-none">
            ⚠
          </span>
          <div>
            <p className="font-semibold">
              {t('notificationAnalytics.alertTitle', {
                defaultValue: 'Elevated notification failure rate',
              })}
            </p>
            <p className="text-sm">
              {t('notificationAnalytics.alertBody', {
                defaultValue:
                  'Failure rate {{rate}} exceeds the {{threshold}} threshold over {{attempted}} attempted deliveries.',
                rate: formatRate(alert.failure_rate),
                threshold: formatRate(alert.threshold),
                attempted: alert.attempted,
              })}
            </p>
          </div>
        </div>
      )}

      {/* Filters */}
      <div className="mb-6 grid grid-cols-1 sm:grid-cols-2 gap-4 p-4 bg-gray-50 border border-gray-200 rounded-lg">
        <label className="flex flex-col gap-1 text-sm">
          <span className="font-medium text-gray-700">
            {t('notificationAnalytics.window', { defaultValue: 'Time window' })}
          </span>
          <select
            value={filters.window}
            onChange={(e) =>
              onFiltersChange({ ...filters, window: e.target.value as AnalyticsWindow })
            }
            className="border border-gray-300 rounded-lg px-3 py-2"
          >
            {WINDOW_OPTIONS.map((w) => (
              <option key={w} value={w}>
                {t(`notificationAnalytics.windowOption.${w}`, { defaultValue: w })}
              </option>
            ))}
          </select>
        </label>
        <label className="flex flex-col gap-1 text-sm">
          <span className="font-medium text-gray-700">
            {t('notificationAnalytics.channel', { defaultValue: 'Channel' })}
          </span>
          <select
            value={filters.channel ?? ''}
            onChange={(e) =>
              onFiltersChange({
                ...filters,
                channel: (e.target.value || undefined) as NotificationChannel | undefined,
              })
            }
            className="border border-gray-300 rounded-lg px-3 py-2"
          >
            <option value="">
              {t('notificationAnalytics.allChannels', { defaultValue: 'All channels' })}
            </option>
            {CHANNEL_OPTIONS.map((c) => (
              <option key={c} value={c}>
                {humaniseChannel(c)}
              </option>
            ))}
          </select>
        </label>
      </div>

      {/* Loading */}
      {isLoading && (
        <div className="flex justify-center py-16" aria-live="polite" aria-busy="true">
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600" />
        </div>
      )}

      {/* Error */}
      {!isLoading && isError && (
        <div
          role="alert"
          className="text-center py-8 border border-red-200 bg-red-50 rounded-lg text-red-700"
        >
          <p className="font-medium">
            {t('notificationAnalytics.failedToLoad', {
              defaultValue: 'Failed to load notification analytics',
            })}
          </p>
          {onRetry && (
            <button
              type="button"
              onClick={onRetry}
              className="mt-3 px-3 py-1 border border-red-300 rounded hover:bg-red-100"
            >
              {t('common.retry', { defaultValue: 'Retry' })}
            </button>
          )}
        </div>
      )}

      {/* Content */}
      {!isLoading && !isError && data && (
        <>
          {/* Totals KPIs */}
          <div className="grid grid-cols-2 lg:grid-cols-4 gap-4 mb-8">
            <Kpi
              label={t('notificationAnalytics.kpiAttempted', { defaultValue: 'Attempted' })}
              value={data.totals.attempted}
            />
            <Kpi
              label={t('notificationAnalytics.kpiSent', { defaultValue: 'Sent' })}
              value={data.totals.sent}
              valueClassName="text-blue-600"
            />
            <Kpi
              label={t('notificationAnalytics.kpiFailed', { defaultValue: 'Failed' })}
              value={data.totals.failed}
              valueClassName={data.totals.failed > 0 ? 'text-red-600' : 'text-gray-900'}
            />
            <Kpi
              label={t('notificationAnalytics.kpiFailureRate', { defaultValue: 'Failure rate' })}
              value={formatRate(data.totals.failure_rate)}
              valueClassName={alert?.firing ? 'text-red-600' : 'text-gray-900'}
            />
          </div>

          {/* Per-channel table */}
          {rows.length === 0 ? (
            <div className="text-center py-12 text-gray-500">
              {t('notificationAnalytics.empty', {
                defaultValue: 'No notification activity in the selected window.',
              })}
            </div>
          ) : (
            <div className="overflow-x-auto border border-gray-200 rounded-lg">
              <table className="min-w-full text-sm">
                <caption className="sr-only">
                  {t('notificationAnalytics.tableCaption', {
                    defaultValue: 'Notification delivery metrics per channel',
                  })}
                </caption>
                <thead className="bg-gray-50 text-gray-600">
                  <tr>
                    <Th align="left">
                      {t('notificationAnalytics.colChannel', { defaultValue: 'Channel' })}
                    </Th>
                    <Th>{t('notificationAnalytics.colSent', { defaultValue: 'Sent' })}</Th>
                    <Th>
                      {t('notificationAnalytics.colDelivered', { defaultValue: 'Delivered' })}
                    </Th>
                    <Th>{t('notificationAnalytics.colFailed', { defaultValue: 'Failed' })}</Th>
                    <Th>{t('notificationAnalytics.colSkipped', { defaultValue: 'Skipped' })}</Th>
                    <Th>{t('notificationAnalytics.colOpened', { defaultValue: 'Opened' })}</Th>
                    <Th>{t('notificationAnalytics.colClicked', { defaultValue: 'Clicked' })}</Th>
                    <Th>
                      {t('notificationAnalytics.colFailureRate', { defaultValue: 'Failure rate' })}
                    </Th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-gray-100">
                  {rows.map((row) => (
                    <tr key={row.channel} className="text-gray-800">
                      <td className="px-3 py-2 text-left font-medium">
                        {humaniseChannel(row.channel)}
                      </td>
                      <Td>{row.sent}</Td>
                      <Td>{row.delivered}</Td>
                      <Td className={row.failed > 0 ? 'text-red-600 font-medium' : undefined}>
                        {row.failed}
                      </Td>
                      <Td>{row.skipped}</Td>
                      <Td>{row.opened}</Td>
                      <Td>{row.clicked}</Td>
                      <Td
                        className={
                          row.failure_rate > (alert?.threshold ?? 0.05)
                            ? 'text-red-600 font-semibold'
                            : undefined
                        }
                      >
                        {formatRate(row.failure_rate)}
                      </Td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </>
      )}
    </div>
  );
}

function Kpi({
  label,
  value,
  valueClassName = 'text-gray-900',
}: {
  label: string;
  value: string | number;
  valueClassName?: string;
}) {
  return (
    <div className="bg-white rounded-lg shadow p-4">
      <p className="text-sm font-medium text-gray-500">{label}</p>
      <p className={`mt-2 text-2xl font-bold ${valueClassName}`}>{value}</p>
    </div>
  );
}

function Th({
  children,
  align = 'right',
}: {
  children: React.ReactNode;
  align?: 'left' | 'right';
}) {
  return (
    <th
      scope="col"
      className={`px-3 py-2 font-semibold ${align === 'left' ? 'text-left' : 'text-right'}`}
    >
      {children}
    </th>
  );
}

function Td({ children, className }: { children: React.ReactNode; className?: string }) {
  return <td className={`px-3 py-2 text-right tabular-nums ${className ?? ''}`}>{children}</td>;
}
