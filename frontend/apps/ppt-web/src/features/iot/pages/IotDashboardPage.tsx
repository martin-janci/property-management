/**
 * IoT / Smart-Building Dashboard (Epic 14 — FR71-75).
 *
 * The manager-web front door for the IoT backend: KPI rollup (FR75), device
 * list (FR71), per-device telemetry (FR72) and threshold alerts (FR74).
 *
 * Presentational only — the route wrapper in `routes/groups/iot.tsx` wires the
 * `@ppt/api-client` hooks and passes data + handlers in, mirroring the
 * outages/faults route-group convention.
 */
import type { Sensor, SensorAlert, SensorDashboard, SensorReading } from '@ppt/api-client';
import type { JSX } from 'react';
import { useTranslation } from 'react-i18next';
import { AlertsPanel, IotStatCard, SensorListPanel, TelemetryPanel } from '../components';

export interface IotDashboardPageProps {
  dashboard?: SensorDashboard;
  dashboardLoading: boolean;
  sensors: Sensor[];
  sensorsLoading: boolean;
  selectedSensorId: string | null;
  selectedSensor: Sensor | null;
  readings: SensorReading[];
  readingsLoading: boolean;
  alerts: SensorAlert[];
  pendingAlertId: string | null;
  isLive?: boolean;
  onSelectSensor: (id: string) => void;
  onAcknowledgeAlert: (alertId: string) => void;
  onResolveAlert: (alertId: string) => void;
}

export function IotDashboardPage({
  dashboard,
  dashboardLoading,
  sensors,
  sensorsLoading,
  selectedSensorId,
  selectedSensor,
  readings,
  readingsLoading,
  alerts,
  pendingAlertId,
  isLive = false,
  onSelectSensor,
  onAcknowledgeAlert,
  onResolveAlert,
}: IotDashboardPageProps): JSX.Element {
  const { t } = useTranslation();

  return (
    <div className="mx-auto max-w-7xl px-4 py-6">
      <header className="mb-6 flex items-start justify-between gap-3">
        <div>
          <h1 className="text-2xl font-semibold text-gray-900">
            {t('iot.dashboardTitle', { defaultValue: 'Smart Building' })}
          </h1>
          <p className="mt-1 text-sm text-gray-500">
            {t('iot.dashboardSubtitle', {
              defaultValue: 'Sensors, telemetry and alerts across your buildings.',
            })}
          </p>
        </div>
        {isLive && (
          <span className="mt-1 inline-flex items-center gap-1.5 rounded-full bg-green-100 px-2.5 py-1 text-xs font-medium text-green-700">
            <span className="h-1.5 w-1.5 animate-pulse rounded-full bg-green-500" />
            {t('iot.live', { defaultValue: 'Live' })}
          </span>
        )}
      </header>

      {/* KPI rollup (FR75) */}
      <div className="mb-6 grid grid-cols-2 gap-4 md:grid-cols-4">
        <IotStatCard
          label={t('iot.totalDevices', { defaultValue: 'Total devices' })}
          value={dashboardLoading ? '—' : (dashboard?.total_sensors ?? 0)}
        />
        <IotStatCard
          label={t('iot.activeDevices', { defaultValue: 'Active' })}
          value={dashboardLoading ? '—' : (dashboard?.active_sensors ?? 0)}
          tone="success"
        />
        <IotStatCard
          label={t('iot.offlineDevices', { defaultValue: 'Offline' })}
          value={dashboardLoading ? '—' : (dashboard?.offline_sensors ?? 0)}
          tone={dashboard && dashboard.offline_sensors > 0 ? 'warning' : 'default'}
        />
        <IotStatCard
          label={t('iot.unresolvedAlerts', { defaultValue: 'Open alerts' })}
          value={dashboardLoading ? '—' : (dashboard?.unresolved_alerts ?? 0)}
          tone={dashboard && dashboard.unresolved_alerts > 0 ? 'danger' : 'default'}
        />
      </div>

      <div className="grid grid-cols-1 gap-6 lg:grid-cols-3">
        <div className="lg:col-span-1">
          <SensorListPanel
            sensors={sensors}
            isLoading={sensorsLoading}
            selectedSensorId={selectedSensorId}
            onSelectSensor={onSelectSensor}
          />
        </div>

        <div className="space-y-6 lg:col-span-2">
          <TelemetryPanel sensor={selectedSensor} readings={readings} isLoading={readingsLoading} />
          <AlertsPanel
            alerts={alerts}
            isLoading={dashboardLoading}
            pendingAlertId={pendingAlertId}
            onAcknowledge={onAcknowledgeAlert}
            onResolve={onResolveAlert}
          />
        </div>
      </div>
    </div>
  );
}
