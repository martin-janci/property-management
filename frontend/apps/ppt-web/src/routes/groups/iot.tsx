/**
 * IoT / Smart-Building route group (Epic 14 — FR71-75).
 *
 * Owns the IoT route-wrapper component and the `<Route>` table fragment.
 * Wires `@ppt/api-client` IoT hooks to the presentational dashboard page,
 * mirroring the outages/faults route-group convention.
 */
import type { UpdateSensorRequest } from '@ppt/api-client';
import {
  useAcknowledgeSensorAlert,
  useCreateSensor,
  useIotDashboard,
  useResolveSensorAlert,
  useSensor,
  useSensorReadings,
  useSensors,
  useUpdateSensor,
} from '@ppt/api-client';
import { useMemo, useState } from 'react';
import { Route, useNavigate, useParams } from 'react-router-dom';
import { AuthRequiredGate, useToast } from '../../components';
import { useAuth } from '../../contexts';
import type { SensorFormData } from '../../features/iot';
import {
  IotDashboardPage,
  IotSensorEditPage,
  IotSensorListPage,
  IotSensorRegisterPage,
} from '../lazyRoutes';

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

/** Route wrapper for IoT sensor list using the new IotSensorListPage (BIT-147). */
function IotSensorListPageRoute() {
  const { user } = useAuth();
  const navigate = useNavigate();
  const [page, setPage] = useState(1);
  const [filterType, setFilterType] = useState('');
  const [filterStatus, setFilterStatus] = useState('');
  const [filterSearch, setFilterSearch] = useState('');
  const pageSize = 20;

  const params = {
    sensor_type: filterType || undefined,
    status: filterStatus || undefined,
    search: filterSearch || undefined,
    limit: pageSize,
    offset: (page - 1) * pageSize,
  };

  const { data: sensorsData, isLoading } = useSensors(params);
  const sensors = useMemo(() => sensorsData?.sensors ?? [], [sensorsData]);

  if (!user?.organizationId) {
    return <AuthRequiredGate />;
  }

  return (
    <IotSensorListPage
      sensors={sensors}
      isLoading={isLoading}
      total={sensors.length + (page - 1) * pageSize}
      page={page}
      pageSize={pageSize}
      filterType={filterType}
      filterStatus={filterStatus}
      filterSearch={filterSearch}
      onPageChange={setPage}
      onFilterTypeChange={(t: string) => { setFilterType(t); setPage(1); }}
      onFilterStatusChange={(s: string) => { setFilterStatus(s); setPage(1); }}
      onFilterSearchChange={(s: string) => { setFilterSearch(s); setPage(1); }}
      onRegister={() => navigate('/iot/sensors/new')}
      onEdit={(id: string) => navigate(`/iot/sensors/${id}/edit`)}
    />
  );
}

/** Route wrapper for sensor register flow using the new IotSensorRegisterPage (BIT-147). */
function IotSensorRegisterPageRoute() {
  const { user } = useAuth();
  const navigate = useNavigate();
  const { showToast } = useToast();
  const createSensor = useCreateSensor();

  if (!user?.organizationId || !user.id) {
    return <AuthRequiredGate />;
  }

  const handleSubmit = async (data: SensorFormData) => {
    const payload = {
      organization_id: user.organizationId!,
      building_id: data.building_id,
      name: data.name.trim(),
      sensor_type: data.sensor_type,
      location: data.location.trim() || null,
      connection_type: data.connection_type || null,
      unit_of_measurement: data.unit_of_measurement.trim() || null,
      data_interval_seconds: data.data_interval_seconds
        ? (parseInt(data.data_interval_seconds, 10) || null)
        : null,
      manufacturer: data.manufacturer.trim() || null,
      model: data.model.trim() || null,
      serial_number: data.serial_number.trim() || null,
      created_by: user.id!,
    };
    try {
      await createSensor.mutateAsync(payload);
      showToast({
        type: 'success',
        title: 'Sensor registered',
        message: 'The sensor has been registered successfully.',
      });
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
    <IotSensorRegisterPage
      isLoading={createSensor.isPending}
      onSubmit={handleSubmit}
      onCancel={() => navigate('/iot/sensors')}
    />
  );
}

/** Route wrapper for sensor edit flow using the new IotSensorEditPage (BIT-147). */
function IotSensorEditPageRoute() {
  const { user } = useAuth();
  const navigate = useNavigate();
  const { showToast } = useToast();
  const { sensorId } = useParams<{ sensorId: string }>();
  const { data: sensor, isLoading: sensorLoading } = useSensor(sensorId ?? '');
  const updateSensor = useUpdateSensor();

  if (!user?.organizationId) {
    return <AuthRequiredGate />;
  }

  const handleSubmit = async (data: SensorFormData) => {
    if (!sensorId) return;
    const payload: UpdateSensorRequest = {
      name: data.name.trim(),
      location: data.location.trim() || null,
      connection_type: data.connection_type || null,
      unit_of_measurement: data.unit_of_measurement.trim() || null,
      data_interval_seconds: data.data_interval_seconds
        ? (parseInt(data.data_interval_seconds, 10) || null)
        : null,
      manufacturer: data.manufacturer.trim() || null,
      model: data.model.trim() || null,
    };
    try {
      await updateSensor.mutateAsync({ id: sensorId, data: payload });
      showToast({
        type: 'success',
        title: 'Sensor updated',
        message: 'The sensor has been updated successfully.',
      });
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
    <IotSensorEditPage
      sensor={sensor}
      isLoading={sensorLoading || updateSensor.isPending}
      onSubmit={handleSubmit}
      onCancel={() => navigate('/iot/sensors')}
    />
  );
}

/** IoT / Smart-Building routes (Epic 14 / FR71-75). */
export function iotRoutes() {
  return (
    <>
      <Route path="/iot" element={<IotDashboardPageRoute />} />
      <Route path="/iot/sensors" element={<IotSensorListPageRoute />} />
      <Route path="/iot/sensors/new" element={<IotSensorRegisterPageRoute />} />
      <Route path="/iot/sensors/:sensorId/edit" element={<IotSensorEditPageRoute />} />
    </>
  );
}
