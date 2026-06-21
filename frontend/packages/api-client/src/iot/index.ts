/**
 * IoT / Smart-Building Module (Epic 14 — FR71-75).
 *
 * API client, hooks and types for sensors, telemetry readings and alerts.
 */

export {
  acknowledgeSensorAlert,
  createSensor,
  createThreshold,
  deleteSensor,
  deleteThreshold,
  getIotDashboard,
  getSensor,
  listAllAlerts,
  listSensorAlerts,
  listSensorReadings,
  listSensors,
  listThresholds,
  resolveSensorAlert,
  updateSensor,
  updateThreshold,
} from './api';
export {
  iotKeys,
  useAcknowledgeSensorAlert,
  useAllAlerts,
  useCreateSensor,
  useCreateThreshold,
  useDeleteSensor,
  useDeleteThreshold,
  useIotDashboard,
  useResolveSensorAlert,
  useSensor,
  useSensorAlerts,
  useSensorReadings,
  useSensors,
  useThresholds,
  useUpdateSensor,
  useUpdateThreshold,
} from './hooks';
export type {
  AggregatedReading,
  CreateSensorRequest,
  CreateSensorThresholdRequest,
  ListAlertsParams,
  ListAlertsResponse,
  ListReadingsParams,
  ListReadingsResponse,
  ListSensorsParams,
  ListSensorsResponse,
  ListThresholdsResponse,
  ResolveSensorAlertRequest,
  Sensor,
  SensorAlert,
  SensorDashboard,
  SensorReading,
  SensorThreshold,
  SensorTypeCount,
  UpdateSensorRequest,
  UpdateSensorThresholdRequest,
} from './types';
