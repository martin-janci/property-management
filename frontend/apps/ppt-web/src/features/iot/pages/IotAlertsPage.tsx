/**
 * IotAlertsPage — standalone full-page threshold-alerts view (Epic 14 — FR74).
 *
 * Lists recent / open sensor alerts with severity, acknowledge + resolve
 * actions and client-side filtering by state (open / acknowledged / resolved)
 * and severity. Presentational only — the route wrapper in
 * `routes/groups/iot.tsx` wires the `@ppt/api-client` IoT hooks and passes the
 * alert list + handlers in, mirroring the dashboard / sensor-list convention.
 *
 * v1 sources alerts from `useIotDashboard().recent_alerts` (the dashboard
 * rollup). A dedicated cross-sensor alerts list endpoint can replace that
 * source later without changing this presentational component.
 */
import type { SensorAlert } from '@ppt/api-client';
import { type JSX, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { SensorStatusBadge } from '../components';

/** Alert lifecycle states the state filter can select. */
export type AlertStateFilter = '' | 'open' | 'acknowledged' | 'resolved';

export interface IotAlertsPageProps {
  alerts: SensorAlert[];
  /** sensor_id → display name, for resolving the sensor column. */
  sensorNames: Record<string, string>;
  isLoading: boolean;
  /** True when the alerts source query failed; renders an error panel. */
  isError?: boolean;
  /** Retry handler for the error panel (re-runs the alerts source query). */
  onRetry?: () => void;
  pendingAlertId: string | null;
  filterSeverity: string;
  filterState: AlertStateFilter;
  onFilterSeverityChange: (severity: string) => void;
  onFilterStateChange: (state: AlertStateFilter) => void;
  onAcknowledge: (alertId: string) => void;
  onResolve: (alertId: string) => void;
  onBackToDashboard: () => void;
}

const SEVERITIES = ['critical', 'warning', 'info'];

function formatTimestamp(iso: string): string {
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? iso : d.toLocaleString();
}

/** Derive the lifecycle state of an alert for badge + filtering. */
function alertState(alert: SensorAlert): Exclude<AlertStateFilter, ''> {
  if (alert.resolved_at != null) {
    return 'resolved';
  }
  if (alert.acknowledged_at != null) {
    return 'acknowledged';
  }
  return 'open';
}

export function IotAlertsPage({
  alerts,
  sensorNames,
  isLoading,
  isError = false,
  onRetry,
  pendingAlertId,
  filterSeverity,
  filterState,
  onFilterSeverityChange,
  onFilterStateChange,
  onAcknowledge,
  onResolve,
  onBackToDashboard,
}: IotAlertsPageProps): JSX.Element {
  const { t } = useTranslation();

  const visibleAlerts = useMemo(
    () =>
      alerts.filter((alert) => {
        if (filterSeverity && alert.severity?.toLowerCase() !== filterSeverity) {
          return false;
        }
        if (filterState && alertState(alert) !== filterState) {
          return false;
        }
        return true;
      }),
    [alerts, filterSeverity, filterState]
  );

  return (
    <div className="mx-auto max-w-7xl px-4 py-6">
      <header className="mb-6 flex flex-wrap items-start justify-between gap-3">
        <div>
          <h1 className="text-2xl font-semibold text-gray-900">
            {t('iot.alerts.title', { defaultValue: 'Sensor Alerts' })}
          </h1>
          <p className="mt-1 text-sm text-gray-500">
            {t('iot.alerts.subtitle', {
              defaultValue: 'Threshold breaches across your buildings.',
            })}
          </p>
        </div>
        <button
          type="button"
          onClick={onBackToDashboard}
          className="rounded-lg border border-gray-300 px-4 py-2 text-sm font-medium text-gray-700 hover:bg-gray-50"
        >
          {t('iot.alerts.backToDashboard', { defaultValue: 'Back to dashboard' })}
        </button>
      </header>

      {/* Filter bar */}
      <div className="mb-4 flex flex-wrap gap-3">
        <select
          value={filterState}
          onChange={(e) => onFilterStateChange(e.target.value as AlertStateFilter)}
          className="rounded-md border border-gray-300 px-3 py-2 text-sm shadow-sm focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
        >
          <option value="">
            {t('iot.alerts.filter.allStates', { defaultValue: 'All states' })}
          </option>
          <option value="open">{t('iot.alerts.state.open', { defaultValue: 'Open' })}</option>
          <option value="acknowledged">
            {t('iot.alerts.state.acknowledged', { defaultValue: 'Acknowledged' })}
          </option>
          <option value="resolved">
            {t('iot.alerts.state.resolved', { defaultValue: 'Resolved' })}
          </option>
        </select>
        <select
          value={filterSeverity}
          onChange={(e) => onFilterSeverityChange(e.target.value)}
          className="rounded-md border border-gray-300 px-3 py-2 text-sm shadow-sm focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
        >
          <option value="">
            {t('iot.alerts.filter.allSeverities', { defaultValue: 'All severities' })}
          </option>
          {SEVERITIES.map((s) => (
            <option key={s} value={s}>
              {t(`iot.severity.${s}`, { defaultValue: s })}
            </option>
          ))}
        </select>
      </div>

      <section className="overflow-hidden rounded-lg border border-gray-200 bg-white shadow-sm">
        {isLoading ? (
          <div className="flex items-center justify-center py-16">
            <div className="h-6 w-6 animate-spin rounded-full border-b-2 border-blue-600" />
          </div>
        ) : isError ? (
          <div className="px-4 py-16 text-center" role="alert">
            <p className="text-sm font-medium text-red-600">
              {t('iot.alerts.loadError', {
                defaultValue: 'Failed to load alerts. Please try again.',
              })}
            </p>
            {onRetry && (
              <button
                type="button"
                onClick={onRetry}
                className="mt-3 rounded-lg border border-gray-300 px-4 py-2 text-sm font-medium text-gray-700 hover:bg-gray-50"
              >
                {t('common.retry', { defaultValue: 'Retry' })}
              </button>
            )}
          </div>
        ) : visibleAlerts.length === 0 ? (
          <div className="px-4 py-16 text-center">
            <p className="text-sm text-gray-400">
              {t('iot.alerts.empty', { defaultValue: 'No alerts match these filters. All clear.' })}
            </p>
          </div>
        ) : (
          <div className="overflow-x-auto">
            <table className="min-w-full divide-y divide-gray-100 text-sm">
              <thead className="bg-gray-50 text-left text-xs font-semibold uppercase tracking-wide text-gray-500">
                <tr>
                  <th className="px-4 py-3">
                    {t('iot.alerts.columns.severity', { defaultValue: 'Severity' })}
                  </th>
                  <th className="px-4 py-3">
                    {t('iot.alerts.columns.sensor', { defaultValue: 'Sensor' })}
                  </th>
                  <th className="px-4 py-3">
                    {t('iot.alerts.columns.message', { defaultValue: 'Message' })}
                  </th>
                  <th className="px-4 py-3">
                    {t('iot.alerts.columns.value', { defaultValue: 'Value / Threshold' })}
                  </th>
                  <th className="px-4 py-3">
                    {t('iot.alerts.columns.triggered', { defaultValue: 'Triggered' })}
                  </th>
                  <th className="px-4 py-3">
                    {t('iot.alerts.columns.state', { defaultValue: 'State' })}
                  </th>
                  <th className="px-4 py-3 text-right">
                    {t('iot.alerts.columns.actions', { defaultValue: 'Actions' })}
                  </th>
                </tr>
              </thead>
              <tbody className="divide-y divide-gray-100">
                {visibleAlerts.map((alert) => {
                  const state = alertState(alert);
                  const busy = pendingAlertId === alert.id;
                  return (
                    <tr key={alert.id} className="hover:bg-gray-50">
                      <td className="px-4 py-3">
                        <SensorStatusBadge value={alert.severity} />
                      </td>
                      <td className="px-4 py-3 text-gray-700">
                        {sensorNames[alert.sensor_id] ?? alert.sensor_id}
                      </td>
                      <td className="px-4 py-3 text-gray-800">
                        {alert.message ??
                          t('iot.thresholdBreach', { defaultValue: 'Threshold breach' })}
                      </td>
                      <td className="px-4 py-3 text-gray-600">
                        {alert.triggered_value} / {alert.threshold_value}
                      </td>
                      <td className="px-4 py-3 text-gray-500">
                        {formatTimestamp(alert.triggered_at)}
                      </td>
                      <td className="px-4 py-3 text-gray-600">
                        {t(`iot.alerts.state.${state}`, { defaultValue: state })}
                      </td>
                      <td className="px-4 py-3 text-right">
                        {state === 'resolved' ? (
                          <span className="text-xs text-gray-400">
                            {t('iot.resolved', { defaultValue: 'resolved' })}
                          </span>
                        ) : (
                          <div className="flex justify-end gap-1">
                            {state === 'open' && (
                              <button
                                type="button"
                                disabled={busy}
                                onClick={() => onAcknowledge(alert.id)}
                                className="rounded border border-gray-300 px-2 py-1 text-xs font-medium text-gray-700 hover:bg-gray-50 disabled:opacity-50"
                              >
                                {t('iot.acknowledge', { defaultValue: 'Acknowledge' })}
                              </button>
                            )}
                            <button
                              type="button"
                              disabled={busy}
                              onClick={() => onResolve(alert.id)}
                              className="rounded bg-blue-600 px-2 py-1 text-xs font-medium text-white hover:bg-blue-700 disabled:opacity-50"
                            >
                              {t('iot.resolve', { defaultValue: 'Resolve' })}
                            </button>
                          </div>
                        )}
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        )}
      </section>
    </div>
  );
}
