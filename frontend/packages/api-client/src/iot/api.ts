/**
 * IoT / Smart-Building API Client (Epic 14 — FR71-75).
 *
 * Direct API functions for sensors, telemetry readings and threshold alerts,
 * using the shared token provider. Mirrors the `buildings` module conventions.
 * Base path: `/api/v1/iot/sensors` (see api-server `lib.rs`).
 */

import { getToken } from '../auth';
import type {
  CreateSensorRequest,
  CreateSensorThresholdRequest,
  ListAlertsParams,
  ListAlertsResponse,
  ListReadingsParams,
  ListReadingsResponse,
  ListSensorsParams,
  ListSensorsResponse,
  ListThresholdsResponse,
  Sensor,
  SensorAlert,
  SensorDashboard,
  SensorThreshold,
  UpdateSensorRequest,
  UpdateSensorThresholdRequest,
} from './types';

const _win = typeof window !== 'undefined' ? (window as unknown as Record<string, unknown>) : {};
const API_BASE = `${_win.__API_BASE_URL__ ? String(_win.__API_BASE_URL__) : ''}/api/v1/iot/sensors`;

function getAuthHeaders(): HeadersInit {
  const token = getToken();
  return token ? { Authorization: `Bearer ${token}` } : {};
}

async function apiRequest<T>(url: string, options: RequestInit = {}): Promise<T> {
  const response = await fetch(url, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...getAuthHeaders(),
      ...options.headers,
    },
  });

  if (!response.ok) {
    const error = await response.json().catch(() => ({}));
    throw new Error(error.message || `HTTP error ${response.status}`);
  }

  if (response.status === 204) {
    return undefined as T;
  }

  return response.json();
}

function buildQueryString(params: object): string {
  const searchParams = new URLSearchParams();
  for (const [key, value] of Object.entries(params)) {
    if (value !== undefined && value !== null) {
      searchParams.append(key, String(value));
    }
  }
  const queryString = searchParams.toString();
  return queryString ? `?${queryString}` : '';
}

/** Smart-building dashboard rollup, optionally scoped to a building (FR75). */
export async function getIotDashboard(
  buildingId?: string,
  signal?: AbortSignal
): Promise<SensorDashboard> {
  const qs = buildQueryString(buildingId ? { building_id: buildingId } : {});
  return apiRequest<SensorDashboard>(`${API_BASE}/dashboard${qs}`, { signal });
}

/** List sensors / devices with optional filters (FR71). */
export async function listSensors(
  params?: ListSensorsParams,
  signal?: AbortSignal
): Promise<ListSensorsResponse> {
  const qs = buildQueryString(params || {});
  return apiRequest<ListSensorsResponse>(`${API_BASE}${qs}`, { signal });
}

/** Get a single sensor by id (FR71). */
export async function getSensor(id: string, signal?: AbortSignal): Promise<Sensor> {
  return apiRequest<Sensor>(`${API_BASE}/${id}`, { signal });
}

/** Register a new sensor / device (FR71). */
export async function createSensor(req: CreateSensorRequest): Promise<Sensor> {
  return apiRequest<Sensor>(`${API_BASE}`, {
    method: 'POST',
    body: JSON.stringify(req),
  });
}

/** Update an existing sensor (FR71). */
export async function updateSensor(id: string, req: UpdateSensorRequest): Promise<Sensor> {
  return apiRequest<Sensor>(`${API_BASE}/${id}`, {
    method: 'PUT',
    body: JSON.stringify(req),
  });
}

/** Delete (decommission) a sensor (FR71). */
export async function deleteSensor(id: string): Promise<void> {
  return apiRequest<void>(`${API_BASE}/${id}`, { method: 'DELETE' });
}

/** List telemetry readings for a sensor (FR72). */
export async function listSensorReadings(
  sensorId: string,
  params?: ListReadingsParams,
  signal?: AbortSignal
): Promise<ListReadingsResponse> {
  const qs = buildQueryString(params || {});
  return apiRequest<ListReadingsResponse>(`${API_BASE}/${sensorId}/readings${qs}`, { signal });
}

/** List alerts for a sensor (FR74). */
export async function listSensorAlerts(
  sensorId: string,
  params?: ListAlertsParams,
  signal?: AbortSignal
): Promise<ListAlertsResponse> {
  const qs = buildQueryString(params || {});
  return apiRequest<ListAlertsResponse>(`${API_BASE}/${sensorId}/alerts${qs}`, { signal });
}

/** Acknowledge an alert (FR74). */
export async function acknowledgeSensorAlert(alertId: string): Promise<SensorAlert> {
  return apiRequest<SensorAlert>(`${API_BASE}/alerts/${alertId}/acknowledge`, {
    method: 'POST',
  });
}

/** Resolve an alert, optionally recording the value at resolution (FR74). */
export async function resolveSensorAlert(
  alertId: string,
  resolvedValue?: number | null
): Promise<SensorAlert> {
  return apiRequest<SensorAlert>(`${API_BASE}/alerts/${alertId}/resolve`, {
    method: 'POST',
    body: JSON.stringify({ resolved_value: resolvedValue ?? null }),
  });
}

// ============================================================================
// Thresholds (FR73)
// ============================================================================

/** List thresholds for a sensor (FR73). */
export async function listThresholds(
  sensorId: string,
  signal?: AbortSignal
): Promise<ListThresholdsResponse> {
  return apiRequest<ListThresholdsResponse>(`${API_BASE}/${sensorId}/thresholds`, { signal });
}

/** Create a threshold rule for a sensor (FR73). */
export async function createThreshold(
  sensorId: string,
  req: CreateSensorThresholdRequest
): Promise<SensorThreshold> {
  return apiRequest<SensorThreshold>(`${API_BASE}/${sensorId}/thresholds`, {
    method: 'POST',
    body: JSON.stringify(req),
  });
}

/** Update a threshold rule (FR73). */
export async function updateThreshold(
  thresholdId: string,
  req: UpdateSensorThresholdRequest
): Promise<SensorThreshold> {
  return apiRequest<SensorThreshold>(`${API_BASE}/thresholds/${thresholdId}`, {
    method: 'PUT',
    body: JSON.stringify(req),
  });
}

/** Delete a threshold rule (FR73). */
export async function deleteThreshold(thresholdId: string): Promise<void> {
  return apiRequest<void>(`${API_BASE}/thresholds/${thresholdId}`, { method: 'DELETE' });
}

// ============================================================================
// All-org alerts (FR74)
// ============================================================================

/** List alerts across all sensors for the org (FR74). */
export async function listAllAlerts(
  params?: ListAlertsParams,
  signal?: AbortSignal
): Promise<ListAlertsResponse> {
  const qs = buildQueryString(params || {});
  return apiRequest<ListAlertsResponse>(`${API_BASE}/alerts${qs}`, { signal });
}
