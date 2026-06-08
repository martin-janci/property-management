/**
 * IoT / Smart-Building route group (Epic 14 — FR71-75).
 *
 * Owns the IoT route-wrapper component and the `<Route>` table fragment.
 * Wires `@ppt/api-client` IoT hooks to the presentational dashboard page,
 * mirroring the outages/faults route-group convention.
 */
import { useSensorReadings, useSensors } from '@ppt/api-client';
import {
  useAcknowledgeSensorAlert,
  useIotDashboard,
  useResolveSensorAlert,
} from '@ppt/api-client';
import { useMemo, useState } from 'react';
import { Route } from 'react-router-dom';
import { AuthRequiredGate } from '../../components';
import { useAuth } from '../../contexts';
import { IotDashboardPage } from '../lazyRoutes';

/**
 * Route wrapper for the IoT dashboard (Epic 14 / FR71-75).
 * Manages sensor selection and wires the IoT API hooks.
 */
function IotDashboardPageRoute() {
  const { user } = useAuth();
  const [selectedSensorId, setSelectedSensorId] = useState<string | null>(null);

  const { data: dashboard, isLoading: dashboardLoading } = useIotDashboard();
  const { data: sensorsData, isLoading: sensorsLoading } = useSensors();
  const { data: readingsData, isLoading: readingsLoading } = useSensorReadings(
    selectedSensorId ?? '',
    { limit: 50 }
  );

  const acknowledgeAlert = useAcknowledgeSensorAlert();
  const resolveAlert = useResolveSensorAlert();

  const sensors = useMemo(() => sensorsData?.sensors ?? [], [sensorsData]);
  const selectedSensor = useMemo(
    () => sensors.find((s) => s.id === selectedSensorId) ?? null,
    [sensors, selectedSensorId]
  );

  if (!user?.organizationId) {
    return <AuthRequiredGate />;
  }

  const pendingAlertId = acknowledgeAlert.isPending
    ? (acknowledgeAlert.variables ?? null)
    : resolveAlert.isPending
      ? (resolveAlert.variables?.alertId ?? null)
      : null;

  return (
    <IotDashboardPage
      dashboard={dashboard}
      dashboardLoading={dashboardLoading}
      sensors={sensors}
      sensorsLoading={sensorsLoading}
      selectedSensorId={selectedSensorId}
      selectedSensor={selectedSensor}
      readings={readingsData?.readings ?? []}
      readingsLoading={readingsLoading && !!selectedSensorId}
      alerts={dashboard?.recent_alerts ?? []}
      pendingAlertId={pendingAlertId}
      onSelectSensor={setSelectedSensorId}
      onAcknowledgeAlert={(alertId) => acknowledgeAlert.mutate(alertId)}
      onResolveAlert={(alertId) => resolveAlert.mutate({ alertId })}
    />
  );
}

/** IoT / Smart-Building routes (Epic 14 / FR71-75). */
export function iotRoutes() {
  return (
    <>
      <Route path="/iot" element={<IotDashboardPageRoute />} />
    </>
  );
}
