/**
 * IoT / Smart-Building Module (Epic 14 — FR71-75).
 *
 * API client, hooks and types for sensors, telemetry readings, alerts and
 * thresholds.
 */

export {
  acknowledgeSensorAlert,
  applyTemplate,
  createSensor,
  createThreshold,
  deleteSensor,
  deleteThreshold,
  getIotDashboard,
  getSensor,
  listSensorAlerts,
  listSensorReadings,
  listSensors,
  listThresholds,
  listThresholdTemplates,
  resolveSensorAlert,
  updateSensor,
  updateThreshold,
} from './api';
export {
  iotKeys,
  useAcknowledgeSensorAlert,
  useApplyTemplate,
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
  useThresholdTemplates,
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
  ListThresholdTemplatesParams,
  ListThresholdTemplatesResponse,
  ResolveSensorAlertRequest,
  Sensor,
  SensorAlert,
  SensorDashboard,
  SensorReading,
  SensorThreshold,
  SensorTypeCount,
  ThresholdTemplate,
  UpdateSensorRequest,
  UpdateSensorThresholdRequest,
} from './types';
