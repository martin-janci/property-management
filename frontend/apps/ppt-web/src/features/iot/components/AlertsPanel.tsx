/**
 * Alerts panel (Epic 14 — FR74). Recent threshold alerts with acknowledge /
 * resolve actions.
 */
import type { SensorAlert } from '@ppt/api-client';
import type { JSX } from 'react';
import { useTranslation } from 'react-i18next';
import { SensorStatusBadge } from './SensorStatusBadge';

export interface AlertsPanelProps {
  alerts: SensorAlert[];
  isLoading: boolean;
  pendingAlertId: string | null;
  onAcknowledge: (alertId: string) => void;
  onResolve: (alertId: string) => void;
}

function formatTimestamp(iso: string): string {
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? iso : d.toLocaleString();
}

export function AlertsPanel({
  alerts,
  isLoading,
  pendingAlertId,
  onAcknowledge,
  onResolve,
}: AlertsPanelProps): JSX.Element {
  const { t } = useTranslation();

  return (
    <section className="rounded-lg border border-gray-200 bg-white shadow-sm">
      <header className="border-b border-gray-100 px-4 py-3">
        <h2 className="text-sm font-semibold text-gray-700">
          {t('iot.recentAlerts', { defaultValue: 'Recent alerts' })}
          <span className="ml-2 text-xs font-normal text-gray-400">{alerts.length}</span>
        </h2>
      </header>

      {isLoading ? (
        <div className="flex items-center justify-center py-12">
          <div className="h-6 w-6 animate-spin rounded-full border-b-2 border-blue-600" />
        </div>
      ) : alerts.length === 0 ? (
        <p className="px-4 py-8 text-center text-sm text-gray-400">
          {t('iot.noAlerts', { defaultValue: 'No active alerts. All clear.' })}
        </p>
      ) : (
        <ul className="divide-y divide-gray-100">
          {alerts.map((alert) => {
            const resolved = alert.resolved_at != null;
            const acknowledged = alert.acknowledged_at != null;
            const busy = pendingAlertId === alert.id;
            return (
              <li key={alert.id} className="px-4 py-3">
                <div className="flex items-start justify-between gap-3">
                  <div className="min-w-0">
                    <div className="flex items-center gap-2">
                      <SensorStatusBadge value={alert.severity} />
                      {resolved && (
                        <span className="text-xs text-gray-400">
                          {t('iot.resolved', { defaultValue: 'resolved' })}
                        </span>
                      )}
                    </div>
                    <p className="mt-1 truncate text-sm text-gray-800">
                      {alert.message ??
                        t('iot.thresholdBreach', {
                          defaultValue: 'Threshold breach',
                        })}
                    </p>
                    <p className="text-xs text-gray-400">
                      {t('iot.triggered', { defaultValue: 'Triggered' })}:{' '}
                      {formatTimestamp(alert.triggered_at)} · {alert.triggered_value} /{' '}
                      {alert.threshold_value}
                    </p>
                  </div>
                  {!resolved && (
                    <div className="flex shrink-0 flex-col gap-1">
                      {!acknowledged && (
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
                </div>
              </li>
            );
          })}
        </ul>
      )}
    </section>
  );
}
