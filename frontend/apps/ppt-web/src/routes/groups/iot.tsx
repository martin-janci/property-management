/**
 * IoT / Smart-Building route group (Epic 14 — FR71-75).
 *
 * Owns the IoT route-wrapper component and the `<Route>` table fragment.
 * Wires `@ppt/api-client` IoT hooks to the presentational dashboard page,
 * mirroring the outages/faults route-group convention.
 */
import type {
  CreateSensorRequest,
  CreateSensorThresholdRequest,
  Sensor,
  SensorAlert,
  SensorThreshold,
  UpdateSensorRequest,
  UpdateSensorThresholdRequest,
} from '@ppt/api-client';
import {
  useAcknowledgeSensorAlert,
  useAllAlerts,
  useBuildings,
  useCreateSensor,
  useCreateThreshold,
  useDeleteSensor,
  useDeleteThreshold,
  useIotDashboard,
  useResolveSensorAlert,
  useSensor,
  useSensorReadings,
  useSensors,
  useThresholds,
  useUpdateSensor,
  useUpdateThreshold,
} from '@ppt/api-client';
import { useMemo, useState } from 'react';
import { Route, useNavigate, useParams } from 'react-router-dom';
import { AuthRequiredGate, useToast } from '../../components';
import { useAuth } from '../../contexts';
import type { SensorFormValues } from '../../features/iot';
import {
  IotDashboardPage,
  IotSensorAlertsPage,
  IotSensorFormPage,
  IotSensorListPage,
  IotSensorThresholdPage,
} from '../lazyRoutes';
import { transformBuildingForUI } from '../shared';

/** Map a numeric-string form field to an integer, or null when blank. */
function toIntOrNull(value: string): number | null {
  const trimmed = value.trim();
  if (trimmed === '') {
    return null;
  }
  const parsed = Number(trimmed);
  return Number.isFinite(parsed) ? Math.round(parsed) : null;
}

/** Map a numeric-string form field to a float, or null when blank. */
function toFloatOrNull(value: string): number | null {
  const trimmed = value.trim();
  if (trimmed === '') {
    return null;
  }
  const parsed = Number(trimmed);
  return Number.isFinite(parsed) ? parsed : null;
}

/** Empty-string → null helper for optional text fields. */
function orNull(value: string): string | null {
  const trimmed = value.trim();
  return trimmed === '' ? null : trimmed;
}

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

/**
 * Route wrapper for the standalone sensor registry / management list (FR71).
 * Wires list + delete and navigation to register/edit.
 */
function SensorListPageRoute() {
  const { user } = useAuth();
  const navigate = useNavigate();
  const { showToast } = useToast();

  const { data: sensorsData, isLoading } = useSensors();
  const deleteSensor = useDeleteSensor();

  const sensors = useMemo(() => sensorsData?.sensors ?? [], [sensorsData]);

  if (!user?.organizationId) {
    return <AuthRequiredGate />;
  }

  const handleDelete = async (sensor: Sensor) => {
    if (
      !window.confirm(
        `Delete sensor "${sensor.name}"? This removes its readings and alerts and cannot be undone.`
      )
    ) {
      return;
    }
    try {
      await deleteSensor.mutateAsync(sensor.id);
      showToast({ type: 'success', title: 'Sensor deleted', message: sensor.name });
    } catch (error) {
      showToast({
        type: 'error',
        title: 'Failed to delete sensor',
        message: error instanceof Error ? error.message : 'Unexpected error',
      });
    }
  };

  return (
    <IotSensorListPage
      sensors={sensors}
      isLoading={isLoading}
      deletingSensorId={deleteSensor.isPending ? (deleteSensor.variables ?? null) : null}
      onNavigateToDashboard={() => navigate('/iot')}
      onNavigateToRegister={() => navigate('/iot/sensors/new')}
      onNavigateToEdit={(id) => navigate(`/iot/sensors/${id}/edit`)}
      onNavigateToThresholds={(id) => navigate(`/iot/sensors/${id}/thresholds`)}
      onNavigateToAlerts={() => navigate('/iot/alerts')}
      onDelete={handleDelete}
    />
  );
}

/** Route wrapper for registering a new sensor (FR71). */
function RegisterSensorPageRoute() {
  const { user } = useAuth();
  const navigate = useNavigate();
  const { showToast } = useToast();

  const { data: buildingsData, isLoading: buildingsLoading } = useBuildings();
  const createSensor = useCreateSensor();

  if (!user?.organizationId || !user.id) {
    return <AuthRequiredGate />;
  }

  const organizationId = user.organizationId;
  const createdBy = user.id;

  const buildings = (buildingsData?.items ?? []).map(transformBuildingForUI);

  const handleSubmit = async (values: SensorFormValues) => {
    const payload: CreateSensorRequest = {
      organization_id: organizationId,
      building_id: values.buildingId,
      name: values.name.trim(),
      sensor_type: values.sensorType.trim(),
      location: orNull(values.location),
      location_description: orNull(values.locationDescription),
      connection_type: orNull(values.connectionType),
      unit_of_measurement: orNull(values.unitOfMeasurement),
      data_interval_seconds: toIntOrNull(values.dataIntervalSeconds),
      manufacturer: orNull(values.manufacturer),
      model: orNull(values.model),
      firmware_version: orNull(values.firmwareVersion),
      serial_number: orNull(values.serialNumber),
      created_by: createdBy,
    };
    try {
      await createSensor.mutateAsync(payload);
      showToast({ type: 'success', title: 'Sensor registered', message: payload.name });
      navigate('/iot/sensors');
    } catch (error) {
      showToast({
        type: 'error',
        title: 'Failed to register sensor',
        message: error instanceof Error ? error.message : 'Unexpected error',
      });
    }
  };

  return (
    <IotSensorFormPage
      mode="create"
      buildings={buildings}
      buildingsLoading={buildingsLoading}
      isSaving={createSensor.isPending}
      onSubmit={handleSubmit}
      onCancel={() => navigate('/iot/sensors')}
    />
  );
}

/** Map a loaded sensor onto the editable form value shape. */
function sensorToFormValues(sensor: Sensor): Partial<SensorFormValues> {
  return {
    buildingId: sensor.building_id,
    name: sensor.name,
    sensorType: sensor.sensor_type,
    location: sensor.location ?? '',
    locationDescription: sensor.location_description ?? '',
    connectionType: sensor.connection_type ?? '',
    unitOfMeasurement: sensor.unit_of_measurement ?? '',
    dataIntervalSeconds:
      sensor.data_interval_seconds != null ? String(sensor.data_interval_seconds) : '',
    manufacturer: sensor.manufacturer ?? '',
    model: sensor.model ?? '',
    firmwareVersion: sensor.firmware_version ?? '',
    serialNumber: sensor.serial_number ?? '',
    status: sensor.status,
  };
}

/** Route wrapper for editing an existing sensor (FR71). */
function EditSensorPageRoute() {
  const { user } = useAuth();
  const navigate = useNavigate();
  const { showToast } = useToast();
  const { sensorId } = useParams<{ sensorId: string }>();

  const { data: sensor, isLoading } = useSensor(sensorId ?? '');
  const { data: buildingsData, isLoading: buildingsLoading } = useBuildings();
  const updateSensor = useUpdateSensor();

  if (!user?.organizationId) {
    return <AuthRequiredGate />;
  }

  const buildings = (buildingsData?.items ?? []).map(transformBuildingForUI);

  const handleSubmit = async (values: SensorFormValues) => {
    if (!sensorId) {
      return;
    }
    const payload: UpdateSensorRequest = {
      name: values.name.trim(),
      location: orNull(values.location),
      location_description: orNull(values.locationDescription),
      connection_type: orNull(values.connectionType),
      unit_of_measurement: orNull(values.unitOfMeasurement),
      data_interval_seconds: toIntOrNull(values.dataIntervalSeconds),
      status: values.status,
      manufacturer: orNull(values.manufacturer),
      model: orNull(values.model),
      firmware_version: orNull(values.firmwareVersion),
    };
    try {
      await updateSensor.mutateAsync({ id: sensorId, data: payload });
      showToast({ type: 'success', title: 'Sensor updated', message: payload.name ?? '' });
      navigate('/iot/sensors');
    } catch (error) {
      showToast({
        type: 'error',
        title: 'Failed to update sensor',
        message: error instanceof Error ? error.message : 'Unexpected error',
      });
    }
  };

  return (
    <IotSensorFormPage
      mode="edit"
      buildings={buildings}
      buildingsLoading={buildingsLoading}
      isLoading={isLoading}
      initialValues={sensor ? sensorToFormValues(sensor) : undefined}
      isSaving={updateSensor.isPending}
      onSubmit={handleSubmit}
      onCancel={() => navigate('/iot/sensors')}
    />
  );
}

/** Route wrapper for threshold configuration on a single sensor (FR73). */
function SensorThresholdPageRoute() {
  const { user } = useAuth();
  const navigate = useNavigate();
  const { showToast } = useToast();
  const { sensorId } = useParams<{ sensorId: string }>();

  const id = sensorId ?? '';
  const { data: sensor } = useSensor(id);
  const { data: thresholdsData, isLoading } = useThresholds(id);
  const createThreshold = useCreateThreshold(id);
  const updateThreshold = useUpdateThreshold(id);
  const deleteThreshold = useDeleteThreshold(id);

  if (!user?.organizationId) {
    return <AuthRequiredGate />;
  }

  const thresholds = thresholdsData?.thresholds ?? [];

  const handleCreate = async (values: import('../../features/iot').ThresholdFormValues) => {
    const payload: CreateSensorThresholdRequest = {
      sensor_id: id,
      metric: values.metric.trim() || null,
      comparison: values.comparison,
      warning_value: toFloatOrNull(values.warningValue),
      warning_high: toFloatOrNull(values.warningHigh),
      critical_value: toFloatOrNull(values.criticalValue),
      critical_high: toFloatOrNull(values.criticalHigh),
      alert_cooldown_minutes: toIntOrNull(values.alertCooldownMinutes),
    };
    try {
      await createThreshold.mutateAsync(payload);
      showToast({ type: 'success', title: 'Threshold added' });
    } catch (error) {
      showToast({
        type: 'error',
        title: 'Failed to add threshold',
        message: error instanceof Error ? error.message : 'Unexpected error',
      });
    }
  };

  const handleUpdate = async (
    thresholdId: string,
    values: Partial<import('../../features/iot').ThresholdFormValues> & { enabled?: boolean }
  ) => {
    const payload: UpdateSensorThresholdRequest = {
      ...(values.comparison != null ? { comparison: values.comparison } : {}),
      ...(values.warningValue != null ? { warning_value: toFloatOrNull(values.warningValue) } : {}),
      ...(values.warningHigh != null ? { warning_high: toFloatOrNull(values.warningHigh) } : {}),
      ...(values.criticalValue != null
        ? { critical_value: toFloatOrNull(values.criticalValue) }
        : {}),
      ...(values.criticalHigh != null ? { critical_high: toFloatOrNull(values.criticalHigh) } : {}),
      ...(values.alertCooldownMinutes != null
        ? { alert_cooldown_minutes: toIntOrNull(values.alertCooldownMinutes) }
        : {}),
      ...(values.enabled != null ? { enabled: values.enabled } : {}),
    };
    try {
      await updateThreshold.mutateAsync({ id: thresholdId, data: payload });
    } catch (error) {
      showToast({
        type: 'error',
        title: 'Failed to update threshold',
        message: error instanceof Error ? error.message : 'Unexpected error',
      });
    }
  };

  const handleDelete = async (threshold: SensorThreshold) => {
    if (!window.confirm('Delete this threshold rule?')) return;
    try {
      await deleteThreshold.mutateAsync(threshold.id);
      showToast({ type: 'success', title: 'Threshold deleted' });
    } catch (error) {
      showToast({
        type: 'error',
        title: 'Failed to delete threshold',
        message: error instanceof Error ? error.message : 'Unexpected error',
      });
    }
  };

  return (
    <IotSensorThresholdPage
      sensor={sensor ?? null}
      thresholds={thresholds}
      isLoading={isLoading}
      isSaving={createThreshold.isPending || updateThreshold.isPending}
      deletingThresholdId={deleteThreshold.isPending ? (deleteThreshold.variables ?? null) : null}
      onBack={() => navigate('/iot/sensors')}
      onCreate={handleCreate}
      onUpdate={handleUpdate}
      onDelete={handleDelete}
    />
  );
}

/** Route wrapper for the org-wide alerts page (FR74). */
function SensorAlertsPageRoute() {
  const { user } = useAuth();
  const navigate = useNavigate();
  const { showToast } = useToast();

  const { data: alertsData, isLoading } = useAllAlerts();
  const acknowledgeAlert = useAcknowledgeSensorAlert();
  const resolveAlert = useResolveSensorAlert();

  if (!user?.organizationId) {
    return <AuthRequiredGate />;
  }

  const handleAcknowledge = async (alert: SensorAlert) => {
    try {
      await acknowledgeAlert.mutateAsync(alert.id);
      showToast({ type: 'success', title: 'Alert acknowledged' });
    } catch (error) {
      showToast({
        type: 'error',
        title: 'Failed to acknowledge alert',
        message: error instanceof Error ? error.message : 'Unexpected error',
      });
    }
  };

  const handleResolve = async (alert: SensorAlert) => {
    try {
      await resolveAlert.mutateAsync({ alertId: alert.id });
      showToast({ type: 'success', title: 'Alert resolved' });
    } catch (error) {
      showToast({
        type: 'error',
        title: 'Failed to resolve alert',
        message: error instanceof Error ? error.message : 'Unexpected error',
      });
    }
  };

  return (
    <IotSensorAlertsPage
      alerts={alertsData?.alerts ?? []}
      isLoading={isLoading}
      acknowledgingId={acknowledgeAlert.isPending ? (acknowledgeAlert.variables ?? null) : null}
      resolvingId={resolveAlert.isPending ? (resolveAlert.variables?.alertId ?? null) : null}
      onNavigateToDashboard={() => navigate('/iot')}
      onNavigateToSensors={() => navigate('/iot/sensors')}
      onAcknowledge={handleAcknowledge}
      onResolve={handleResolve}
    />
  );
}

/** IoT / Smart-Building routes (Epic 14 / FR71-75). */
export function iotRoutes() {
  return (
    <>
      <Route path="/iot" element={<IotDashboardPageRoute />} />
      <Route path="/iot/sensors" element={<SensorListPageRoute />} />
      <Route path="/iot/sensors/new" element={<RegisterSensorPageRoute />} />
      <Route path="/iot/sensors/:sensorId/edit" element={<EditSensorPageRoute />} />
      <Route path="/iot/sensors/:sensorId/thresholds" element={<SensorThresholdPageRoute />} />
      <Route path="/iot/alerts" element={<SensorAlertsPageRoute />} />
    </>
  );
}
