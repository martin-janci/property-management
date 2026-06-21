/**
 * IoT / Smart-Building Module (Epic 14 — FR71-75).
 *
 * API client, hooks and types for sensors, telemetry readings and alerts.
 */

export {
  acknowledgeSensorAlert,
  createSensor,
  deleteSensor,
  getIotDashboard,
  getSensor,
  listSensorAlerts,
  listSensorReadings,
  listSensors,
  resolveSensorAlert,
  updateSensor,
} from './api';
export {
  iotKeys,
  useAcknowledgeSensorAlert,
  useCreateSensor,
  useDeleteSensor,
  useIotDashboard,
  useResolveSensorAlert,
  useSensor,
  useSensorAlerts,
  useSensorReadings,
  useSensors,
  useUpdateSensor,
} from './hooks';
export type {
  AggregatedReading,
  CreateSensorRequest,
  ListAlertsParams,
  ListAlertsResponse,
  ListReadingsParams,
  ListReadingsResponse,
  ListSensorsParams,
  ListSensorsResponse,
  ResolveSensorAlertRequest,
  Sensor,
  SensorAlert,
  SensorDashboard,
  SensorReading,
  SensorTypeCount,
  UpdateSensorRequest,
} from './types';
