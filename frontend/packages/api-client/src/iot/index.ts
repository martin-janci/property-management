/**
 * IoT / Smart-Building Module (Epic 14 — FR71-75).
 *
 * API client, hooks and types for sensors, telemetry readings and alerts.
 */

export {
  acknowledgeSensorAlert,
  getIotDashboard,
  getSensor,
  listSensorAlerts,
  listSensorReadings,
  listSensors,
  resolveSensorAlert,
} from './api';
export {
  iotKeys,
  useAcknowledgeSensorAlert,
  useIotDashboard,
  useResolveSensorAlert,
  useSensor,
  useSensorAlerts,
  useSensorReadings,
  useSensors,
} from './hooks';
export type {
  AggregatedReading,
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
} from './types';
