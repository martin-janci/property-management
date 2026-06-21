/**
 * Org-wide IoT alerts page (Epic 14 — FR74).
 *
 * Shows a filterable list of all sensor alerts across the organisation with
 * acknowledge and resolve actions. Presentational — the route wrapper in
 * `routes/groups/iot.tsx` wires hooks and passes data + handlers in.
 */
import type { SensorAlert } from '@ppt/api-client';
import type { JSX } from 'react';
import { useTranslation } from 'react-i18next';

export interface SensorAlertsPageProps {
  alerts: SensorAlert[];
  isLoading: boolean;
  acknowledgingId: string | null;
  resolvingId: string | null;
  onNavigateToDashboard: () => void;
  onNavigateToSensors: () => void;
  onAcknowledge: (alert: SensorAlert) => void;
  onResolve: (alert: SensorAlert) => void;
}

function formatTimestamp(value: string | null | undefined): string {
  if (!value) return '—';
  const d = new Date(value);
  return Number.isNaN(d.getTime()) ? '—' : d.toLocaleString();
}

function SeverityBadge({ severity }: { severity: string }): JSX.Element {
  const classes =
    severity === 'critical'
      ? 'bg-red-100 text-red-700'
      : severity === 'warning'
        ? 'bg-yellow-100 text-yellow-700'
        : 'bg-gray-100 text-gray-600';
  return (
    <span
      className={`inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium ${classes}`}
    >
      {severity}
    </span>
  );
}

function AlertStatusBadge({ alert }: { alert: SensorAlert }): JSX.Element {
  if (alert.resolved_at) {
    return (
      <span className="inline-flex items-center rounded-full bg-green-100 px-2 py-0.5 text-xs font-medium text-green-700">
        Resolved
      </span>
    );
  }
  if (alert.acknowledged_at) {
    return (
      <span className="inline-flex items-center rounded-full bg-blue-100 px-2 py-0.5 text-xs font-medium text-blue-700">
        Acknowledged
      </span>
    );
  }
  return (
    <span className="inline-flex items-center rounded-full bg-red-100 px-2 py-0.5 text-xs font-medium text-red-600">
      Open
    </span>
  );
}

export function SensorAlertsPage({
  alerts,
  isLoading,
  acknowledgingId,
  resolvingId,
  onNavigateToDashboard,
  onNavigateToSensors,
  onAcknowledge,
  onResolve,
}: SensorAlertsPageProps): JSX.Element {
  const { t } = useTranslation();

  return (
    <div className="mx-auto max-w-7xl px-4 py-6">
      <header className="mb-6 flex flex-wrap items-start justify-between gap-3">
        <div>
          <h1 className="text-2xl font-semibold text-gray-900">
            {t('iot.alertsTitle', { defaultValue: 'Sensor Alerts' })}
          </h1>
          <p className="mt-1 text-sm text-gray-500">
            {t('iot.alertsSubtitle', {
              defaultValue: 'All threshold alerts across your organisation.',
            })}
          </p>
        </div>
        <div className="flex items-center gap-2">
          <button
            type="button"
            onClick={onNavigateToSensors}
            className="rounded-md border border-gray-300 px-3 py-2 text-sm text-gray-700 hover:bg-gray-50"
          >
            {t('iot.sensorRegistry', { defaultValue: 'Sensor registry' })}
          </button>
          <button
            type="button"
            onClick={onNavigateToDashboard}
            className="rounded-md border border-gray-300 px-3 py-2 text-sm text-gray-700 hover:bg-gray-50"
          >
            {t('iot.dashboard', { defaultValue: 'Dashboard' })}
          </button>
        </div>
      </header>

      {isLoading ? (
        <p className="py-8 text-center text-sm text-gray-400">
          {t('common.loading', { defaultValue: 'Loading…' })}
        </p>
      ) : alerts.length === 0 ? (
        <div className="rounded-lg border border-dashed border-gray-300 py-16 text-center">
          <p className="text-base font-medium text-gray-500">
            {t('iot.noAlerts', { defaultValue: 'No alerts — all sensors are within thresholds.' })}
          </p>
        </div>
      ) : (
        <div className="overflow-x-auto rounded-lg border border-gray-200">
          <table className="min-w-full divide-y divide-gray-200 text-sm">
            <thead className="bg-gray-50">
              <tr>
                <th className="px-4 py-3 text-left font-medium text-gray-500">
                  {t('iot.severity', { defaultValue: 'Severity' })}
                </th>
                <th className="px-4 py-3 text-left font-medium text-gray-500">
                  {t('common.status', { defaultValue: 'Status' })}
                </th>
                <th className="px-4 py-3 text-left font-medium text-gray-500">
                  {t('iot.message', { defaultValue: 'Message' })}
                </th>
                <th className="px-4 py-3 text-right font-medium text-gray-500">
                  {t('iot.triggeredValue', { defaultValue: 'Value' })}
                </th>
                <th className="px-4 py-3 text-right font-medium text-gray-500">
                  {t('iot.thresholdValue', { defaultValue: 'Threshold' })}
                </th>
                <th className="px-4 py-3 text-left font-medium text-gray-500">
                  {t('iot.triggeredAt', { defaultValue: 'Triggered' })}
                </th>
                <th className="px-4 py-3 text-right font-medium text-gray-500">
                  {t('common.actions', { defaultValue: 'Actions' })}
                </th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-100 bg-white">
              {alerts.map((alert) => {
                const isActioning = acknowledgingId === alert.id || resolvingId === alert.id;
                return (
                  <tr key={alert.id} className="hover:bg-gray-50">
                    <td className="px-4 py-3">
                      <SeverityBadge severity={alert.severity} />
                    </td>
                    <td className="px-4 py-3">
                      <AlertStatusBadge alert={alert} />
                    </td>
                    <td className="max-w-xs px-4 py-3 text-gray-700">{alert.message || '—'}</td>
                    <td className="px-4 py-3 text-right font-mono text-gray-700">
                      {alert.triggered_value}
                    </td>
                    <td className="px-4 py-3 text-right font-mono text-gray-700">
                      {alert.threshold_value}
                    </td>
                    <td className="px-4 py-3 text-gray-500">
                      {formatTimestamp(alert.triggered_at)}
                    </td>
                    <td className="px-4 py-3 text-right">
                      <div className="flex justify-end gap-2">
                        {!alert.acknowledged_at && !alert.resolved_at && (
                          <button
                            type="button"
                            disabled={isActioning}
                            onClick={() => onAcknowledge(alert)}
                            className="text-blue-600 hover:underline disabled:opacity-50"
                          >
                            {acknowledgingId === alert.id
                              ? t('common.saving', { defaultValue: 'Saving…' })
                              : t('iot.acknowledge', { defaultValue: 'Ack' })}
                          </button>
                        )}
                        {!alert.resolved_at && (
                          <button
                            type="button"
                            disabled={isActioning}
                            onClick={() => onResolve(alert)}
                            className="text-green-600 hover:underline disabled:opacity-50"
                          >
                            {resolvingId === alert.id
                              ? t('common.saving', { defaultValue: 'Saving…' })
                              : t('iot.resolve', { defaultValue: 'Resolve' })}
                          </button>
                        )}
                        {alert.resolved_at && (
                          <span className="text-gray-400">
                            {formatTimestamp(alert.resolved_at)}
                          </span>
                        )}
                      </div>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
