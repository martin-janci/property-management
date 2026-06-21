/**
 * IoT / Smart-Building Types (Epic 14 — FR71-75).
 *
 * Mirrors the api-server JSON shapes from `routes/iot.rs` /
 * `crates/db/src/models/sensor.rs`. The backend serializes with the default
 * serde casing (snake_case), so fields are snake_case to match the wire.
 */

/** A registered IoT sensor / device (FR71). */
export interface Sensor {
  id: string;
  organization_id: string;
  building_id: string;
  unit_id: string | null;
  name: string;
  sensor_type: string;
  location: string | null;
  location_description: string | null;
  connection_type: string;
  connection_config: unknown;
  unit_of_measurement: string | null;
  data_interval_seconds: number | null;
  status: string;
  last_seen_at: string | null;
  last_reading_at: string | null;
  last_error: string | null;
  error_count: number | null;
  manufacturer: string | null;
  model: string | null;
  firmware_version: string | null;
  serial_number: string | null;
  installed_at: string | null;
  metadata: unknown;
  created_at: string;
  updated_at: string;
  created_by: string;
}

/** A single telemetry reading from a sensor (FR72). */
export interface SensorReading {
  id: string;
  sensor_id: string;
  value: number;
  unit: string;
  quality: string | null;
  raw_data: unknown;
  timestamp: string;
}

/** An aggregated telemetry bucket (FR72). */
export interface AggregatedReading {
  period: string;
  min_value: number;
  max_value: number;
  avg_value: number;
  count: number;
}

/** A threshold breach alert (FR74). */
export interface SensorAlert {
  id: string;
  sensor_id: string;
  threshold_id: string;
  severity: string;
  triggered_value: number;
  threshold_value: number;
  message: string | null;
  triggered_at: string;
  resolved_at: string | null;
  resolved_value: number | null;
  acknowledged_by: string | null;
  acknowledged_at: string | null;
  notification_sent: boolean;
}

/** Sensor count grouped by type (dashboard rollup). */
export interface SensorTypeCount {
  sensor_type: string;
  count: number;
}

/** Smart-building dashboard rollup (FR75). */
export interface SensorDashboard {
  total_sensors: number;
  active_sensors: number;
  offline_sensors: number;
  unresolved_alerts: number;
  sensors_by_type: SensorTypeCount[];
  recent_alerts: SensorAlert[];
}

export interface ListSensorsParams {
  building_id?: string;
  unit_id?: string;
  sensor_type?: string;
  status?: string;
  search?: string;
  limit?: number;
  offset?: number;
}

export interface ListReadingsParams {
  from_time?: string;
  to_time?: string;
  aggregation?: string;
  limit?: number;
}

export interface ListAlertsParams {
  severity?: string;
  resolved?: boolean;
  acknowledged?: boolean;
  from_time?: string;
  to_time?: string;
  limit?: number;
  offset?: number;
}

export interface ListSensorsResponse {
  sensors: Sensor[];
}

export interface ListReadingsResponse {
  readings: SensorReading[];
}

export interface ListAlertsResponse {
  alerts: SensorAlert[];
}

export interface ResolveSensorAlertRequest {
  resolved_value?: number | null;
}

/**
 * Register-sensor request (FR71). Mirrors `CreateSensor` in the api-server.
 * `organization_id` is pinned to the caller's tenant server-side, but the
 * field is still required on the wire — send the authenticated org id.
 */
export interface CreateSensorRequest {
  organization_id: string;
  building_id: string;
  unit_id?: string | null;
  name: string;
  sensor_type: string;
  location?: string | null;
  location_description?: string | null;
  connection_type?: string | null;
  connection_config?: unknown;
  unit_of_measurement?: string | null;
  data_interval_seconds?: number | null;
  manufacturer?: string | null;
  model?: string | null;
  firmware_version?: string | null;
  serial_number?: string | null;
  installed_at?: string | null;
  metadata?: unknown;
  created_by: string;
}

/** Edit-sensor request (FR71). Mirrors `UpdateSensor`; all fields optional. */
export interface UpdateSensorRequest {
  name?: string | null;
  location?: string | null;
  location_description?: string | null;
  connection_type?: string | null;
  connection_config?: unknown;
  unit_of_measurement?: string | null;
  data_interval_seconds?: number | null;
  status?: string | null;
  manufacturer?: string | null;
  model?: string | null;
  firmware_version?: string | null;
  metadata?: unknown;
}

// ============================================================================
// Thresholds (FR73)
// ============================================================================

export interface SensorThreshold {
  id: string;
  sensor_id: string;
  metric: string;
  comparison: string;
  warning_value: number | null;
  warning_high: number | null;
  critical_value: number | null;
  critical_high: number | null;
  enabled: boolean;
  alert_cooldown_minutes: number | null;
  created_at: string;
  updated_at: string;
}

export interface ListThresholdsResponse {
  thresholds: SensorThreshold[];
}

export interface CreateSensorThresholdRequest {
  sensor_id: string;
  metric?: string | null;
  comparison: string;
  warning_value?: number | null;
  warning_high?: number | null;
  critical_value?: number | null;
  critical_high?: number | null;
  alert_cooldown_minutes?: number | null;
}

export interface UpdateSensorThresholdRequest {
  comparison?: string | null;
  warning_value?: number | null;
  warning_high?: number | null;
  critical_value?: number | null;
  critical_high?: number | null;
  enabled?: boolean | null;
  alert_cooldown_minutes?: number | null;
}
