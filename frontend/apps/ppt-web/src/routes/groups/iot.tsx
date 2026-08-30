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
  SensorThreshold,
  UpdateSensorRequest,
  UpdateSensorThresholdRequest,
} from '@ppt/api-client';
import {
  useAcknowledgeSensorAlert,
  useApplyTemplate,
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
  useThresholdTemplates,
  useUpdateSensor,
  useUpdateThreshold,
} from '@ppt/api-client';
import { useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Route, useNavigate, useParams } from 'react-router-dom';
import { AuthRequiredGate, useToast } from '../../components';
import { useAuth } from '../../contexts';
import type { AlertStateFilter, SensorFormValues, ThresholdFormValues } from '../../features/iot';
import { useIotWebSocket } from '../../features/iot';
import {
  IotAlertsPage,
  IotDashboardPage,
  IotSensorFormPage,
  IotSensorListPage,
  IotThresholdConfigPage,
} from '../lazyRoutes';
import { transformBuildingForUI } from '../shared';

/** Map a numeric-string form field to a number, or null when blank. */
function toIntOrNull(value: string): number | null {
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
 * Real-time readings from the BIT-145 WS channel are merged with the
 * REST-polled snapshot so the telemetry chart updates as readings arrive.
 */
function IotDashboardPageRoute() {
  const { t } = useTranslation();
  const { user } = useAuth();
  const { showToast } = useToast();
  const [selectedSensorId, setSelectedSensorId] = useState<string | null>(null);

  const { data: dashboard, isLoading: dashboardLoading } = useIotDashboard();
  const { data: sensorsData, isLoading: sensorsLoading } = useSensors();
  const { data: readingsData, isLoading: readingsLoading } = useSensorReadings(
    selectedSensorId ?? '',
    { limit: 50 }
  );

  const { liveReadings, isConnected } = useIotWebSocket(
    selectedSensorId,
    user?.organizationId ?? null
  );

  const acknowledgeAlert = useAcknowledgeSensorAlert();
  const resolveAlert = useResolveSensorAlert();

  const sensors = useMemo(() => sensorsData?.sensors ?? [], [sensorsData]);
  const selectedSensor = useMemo(
    () => sensors.find((s) => s.id === selectedSensorId) ?? null,
    [sensors, selectedSensorId]
  );

  const mergedReadings = useMemo(() => {
    const polled = readingsData?.readings ?? [];
    if (liveReadings.length === 0) return polled;
    const seen = new Set(polled.map((r) => r.id));
    const fresh = liveReadings.filter((r) => !seen.has(r.id));
    return [...fresh, ...polled].slice(0, 100);
  }, [liveReadings, readingsData]);

  if (!user?.organizationId) {
    return <AuthRequiredGate />;
  }

  const pendingAlertId = acknowledgeAlert.isPending
    ? (acknowledgeAlert.variables ?? null)
    : resolveAlert.isPending
      ? (resolveAlert.variables?.alertId ?? null)
      : null;

  const handleAcknowledgeAlert = async (alertId: string) => {
    try {
      await acknowledgeAlert.mutateAsync(alertId);
    } catch (error) {
      showToast({
        type: 'error',
        title: t('iot.acknowledgeFailed', { defaultValue: 'Failed to acknowledge alert' }),
        message: error instanceof Error ? error.message : t('auth.unexpectedError'),
      });
    }
  };

  const handleResolveAlert = async (alertId: string) => {
    try {
      await resolveAlert.mutateAsync({ alertId });
    } catch (error) {
      showToast({
        type: 'error',
        title: t('iot.resolveFailed', { defaultValue: 'Failed to resolve alert' }),
        message: error instanceof Error ? error.message : t('auth.unexpectedError'),
      });
    }
  };

  return (
    <IotDashboardPage
      dashboard={dashboard}
      dashboardLoading={dashboardLoading}
      sensors={sensors}
      sensorsLoading={sensorsLoading}
      selectedSensorId={selectedSensorId}
      selectedSensor={selectedSensor}
      readings={mergedReadings}
      readingsLoading={readingsLoading && !!selectedSensorId}
      alerts={dashboard?.recent_alerts ?? []}
      pendingAlertId={pendingAlertId}
      isLive={isConnected}
      onSelectSensor={setSelectedSensorId}
      onAcknowledgeAlert={handleAcknowledgeAlert}
      onResolveAlert={handleResolveAlert}
    />
  );
}

/**
 * Route wrapper for the standalone sensor registry / management list (FR71).
 * Wires list + delete and navigation to register/edit.
 */
function SensorListPageRoute() {
  const { t } = useTranslation();
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
        t('iot.sensorDeleteConfirm', {
          name: sensor.name,
          defaultValue:
            'Delete sensor "{{name}}"? This removes its readings and alerts and cannot be undone.',
        })
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

/**
 * Route wrapper for the standalone alerts page (FR74).
 *
 * v1 sources alerts from the dashboard rollup (`recent_alerts`); a dedicated
 * cross-sensor alerts list endpoint can replace that source later. Sensor names
 * come from `useSensors()` so the table can label each alert's sensor.
 */
function IotAlertsPageRoute() {
  const { t } = useTranslation();
  const { user } = useAuth();
  const navigate = useNavigate();
  const { showToast } = useToast();
  const [filterSeverity, setFilterSeverity] = useState('');
  const [filterState, setFilterState] = useState<AlertStateFilter>('');

  const {
    data: dashboard,
    isLoading: dashboardLoading,
    isError: dashboardError,
    refetch: refetchDashboard,
  } = useIotDashboard();
  const { data: sensorsData } = useSensors();

  const acknowledgeAlert = useAcknowledgeSensorAlert();
  const resolveAlert = useResolveSensorAlert();

  const sensorNames = useMemo(() => {
    const map: Record<string, string> = {};
    for (const sensor of sensorsData?.sensors ?? []) {
      map[sensor.id] = sensor.name;
    }
    return map;
  }, [sensorsData]);

  if (!user?.organizationId) {
    return <AuthRequiredGate />;
  }

  const pendingAlertId = acknowledgeAlert.isPending
    ? (acknowledgeAlert.variables ?? null)
    : resolveAlert.isPending
      ? (resolveAlert.variables?.alertId ?? null)
      : null;

  const handleAcknowledge = async (alertId: string) => {
    try {
      await acknowledgeAlert.mutateAsync(alertId);
    } catch (error) {
      showToast({
        type: 'error',
        title: t('iot.acknowledgeFailed', { defaultValue: 'Failed to acknowledge alert' }),
        message: error instanceof Error ? error.message : t('auth.unexpectedError'),
      });
    }
  };

  const handleResolve = async (alertId: string) => {
    try {
      await resolveAlert.mutateAsync({ alertId });
    } catch (error) {
      showToast({
        type: 'error',
        title: t('iot.resolveFailed', { defaultValue: 'Failed to resolve alert' }),
        message: error instanceof Error ? error.message : t('auth.unexpectedError'),
      });
    }
  };

  return (
    <IotAlertsPage
      alerts={dashboard?.recent_alerts ?? []}
      sensorNames={sensorNames}
      isLoading={dashboardLoading}
      isError={dashboardError}
      onRetry={() => refetchDashboard()}
      pendingAlertId={pendingAlertId}
      filterSeverity={filterSeverity}
      filterState={filterState}
      onFilterSeverityChange={setFilterSeverity}
      onFilterStateChange={setFilterState}
      onAcknowledge={handleAcknowledge}
      onResolve={handleResolve}
      onBackToDashboard={() => navigate('/iot')}
    />
  );
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

/** Map threshold form values onto the api-client create-request shape. */
function formToCreateThreshold(
  sensorId: string,
  values: ThresholdFormValues
): CreateSensorThresholdRequest {
  return {
    sensor_id: sensorId,
    metric: orNull(values.metric),
    comparison: values.comparison.trim(),
    warning_value: toFloatOrNull(values.warningValue),
    warning_high: toFloatOrNull(values.warningHigh),
    critical_value: toFloatOrNull(values.criticalValue),
    critical_high: toFloatOrNull(values.criticalHigh),
    alert_cooldown_minutes: toIntOrNull(values.cooldownMinutes),
  };
}

/** Map threshold form values onto the api-client update-request shape. */
function formToUpdateThreshold(values: ThresholdFormValues): UpdateSensorThresholdRequest {
  return {
    comparison: values.comparison.trim(),
    warning_value: toFloatOrNull(values.warningValue),
    warning_high: toFloatOrNull(values.warningHigh),
    critical_value: toFloatOrNull(values.criticalValue),
    critical_high: toFloatOrNull(values.criticalHigh),
    enabled: values.enabled,
    alert_cooldown_minutes: toIntOrNull(values.cooldownMinutes),
  };
}

/** Route wrapper for per-sensor threshold configuration (FR73). */
function ThresholdConfigPageRoute() {
  const { t } = useTranslation();
  const { user } = useAuth();
  const navigate = useNavigate();
  const { showToast } = useToast();
  const { sensorId } = useParams<{ sensorId: string }>();
  const id = sensorId ?? '';

  const { data: sensor, isLoading: sensorLoading } = useSensor(id);
  const { data: thresholdsData, isLoading: thresholdsLoading } = useThresholds(id);
  const { data: templatesData, isLoading: templatesLoading } = useThresholdTemplates(
    sensor?.sensor_type
  );

  const createThreshold = useCreateThreshold(id);
  const updateThreshold = useUpdateThreshold(id);
  const toggleThreshold = useUpdateThreshold(id);
  const deleteThreshold = useDeleteThreshold(id);
  const applyTemplate = useApplyTemplate(id);

  if (!user?.organizationId) {
    return <AuthRequiredGate />;
  }

  const thresholds = thresholdsData?.thresholds ?? [];
  const templates = templatesData?.templates ?? [];

  const handleCreate = async (values: ThresholdFormValues) => {
    try {
      await createThreshold.mutateAsync(formToCreateThreshold(id, values));
      showToast({
        type: 'success',
        title: t('iot.thresholdCreated', { defaultValue: 'Threshold created' }),
        message: '',
      });
    } catch (error) {
      showToast({
        type: 'error',
        title: t('iot.thresholdCreateFailed', { defaultValue: 'Failed to create threshold' }),
        message: error instanceof Error ? error.message : t('auth.unexpectedError'),
      });
      throw error;
    }
  };

  const handleUpdate = async (thresholdId: string, values: ThresholdFormValues) => {
    try {
      await updateThreshold.mutateAsync({ id: thresholdId, data: formToUpdateThreshold(values) });
      showToast({
        type: 'success',
        title: t('iot.thresholdUpdated', { defaultValue: 'Threshold updated' }),
        message: '',
      });
    } catch (error) {
      showToast({
        type: 'error',
        title: t('iot.thresholdUpdateFailed', { defaultValue: 'Failed to update threshold' }),
        message: error instanceof Error ? error.message : t('auth.unexpectedError'),
      });
      throw error;
    }
  };

  const handleDelete = async (threshold: SensorThreshold) => {
    if (
      !window.confirm(
        t('iot.thresholdDeleteConfirm', {
          defaultValue: 'Delete this threshold? This cannot be undone.',
        })
      )
    ) {
      return;
    }
    try {
      await deleteThreshold.mutateAsync(threshold.id);
      showToast({
        type: 'success',
        title: t('iot.thresholdDeleted', { defaultValue: 'Threshold deleted' }),
        message: '',
      });
    } catch (error) {
      showToast({
        type: 'error',
        title: t('iot.thresholdDeleteFailed', { defaultValue: 'Failed to delete threshold' }),
        message: error instanceof Error ? error.message : t('auth.unexpectedError'),
      });
    }
  };

  const handleToggle = async (threshold: SensorThreshold) => {
    try {
      await toggleThreshold.mutateAsync({
        id: threshold.id,
        data: { enabled: !threshold.enabled },
      });
    } catch (error) {
      showToast({
        type: 'error',
        title: t('iot.thresholdUpdateFailed', { defaultValue: 'Failed to update threshold' }),
        message: error instanceof Error ? error.message : t('auth.unexpectedError'),
      });
    }
  };

  const handleApplyTemplate = async (templateId: string) => {
    try {
      await applyTemplate.mutateAsync(templateId);
      showToast({
        type: 'success',
        title: t('iot.templateApplied', { defaultValue: 'Template applied' }),
        message: '',
      });
    } catch (error) {
      showToast({
        type: 'error',
        title: t('iot.templateApplyFailed', { defaultValue: 'Failed to apply template' }),
        message: error instanceof Error ? error.message : t('auth.unexpectedError'),
      });
    }
  };

  return (
    <IotThresholdConfigPage
      sensor={sensor ?? null}
      sensorLoading={sensorLoading}
      thresholds={thresholds}
      thresholdsLoading={thresholdsLoading}
      templates={templates}
      templatesLoading={templatesLoading}
      isSaving={createThreshold.isPending || updateThreshold.isPending}
      togglingId={toggleThreshold.isPending ? (toggleThreshold.variables?.id ?? null) : null}
      deletingId={deleteThreshold.isPending ? (deleteThreshold.variables ?? null) : null}
      applyingTemplate={applyTemplate.isPending}
      onCreate={handleCreate}
      onUpdate={handleUpdate}
      onDelete={handleDelete}
      onToggleEnabled={handleToggle}
      onApplyTemplate={handleApplyTemplate}
      onBack={() => navigate('/iot/sensors')}
    />
  );
}

/** IoT / Smart-Building routes (Epic 14 / FR71-75). */
export function iotRoutes() {
  return (
    <>
      <Route path="/iot" element={<IotDashboardPageRoute />} />
      <Route path="/iot/alerts" element={<IotAlertsPageRoute />} />
      <Route path="/iot/sensors" element={<SensorListPageRoute />} />
      <Route path="/iot/sensors/new" element={<RegisterSensorPageRoute />} />
      <Route path="/iot/sensors/:sensorId/edit" element={<EditSensorPageRoute />} />
      <Route path="/iot/sensors/:sensorId/thresholds" element={<ThresholdConfigPageRoute />} />
    </>
  );
}
