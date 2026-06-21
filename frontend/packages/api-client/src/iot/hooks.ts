/**
 * IoT / Smart-Building TanStack Query Hooks (Epic 14 — FR71-75).
 *
 * React hooks for sensors, telemetry and alerts with server-state caching,
 * following the `buildings` direct-hooks conventions.
 */

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import {
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
import type {
  CreateSensorRequest,
  ListAlertsParams,
  ListReadingsParams,
  ListSensorsParams,
  UpdateSensorRequest,
} from './types';

// ============================================
// Query Key Factory
// ============================================

export const iotKeys = {
  all: ['iot'] as const,
  dashboard: (buildingId?: string) => [...iotKeys.all, 'dashboard', buildingId ?? null] as const,
  sensors: () => [...iotKeys.all, 'sensors'] as const,
  sensorList: (params?: ListSensorsParams) => [...iotKeys.sensors(), 'list', params] as const,
  sensor: (id: string) => [...iotKeys.sensors(), 'detail', id] as const,
  readings: (sensorId: string, params?: ListReadingsParams) =>
    [...iotKeys.sensor(sensorId), 'readings', params] as const,
  alerts: (sensorId: string, params?: ListAlertsParams) =>
    [...iotKeys.sensor(sensorId), 'alerts', params] as const,
};

// ============================================
// Queries
// ============================================

/** Smart-building dashboard rollup (FR75). */
export function useIotDashboard(buildingId?: string) {
  return useQuery({
    queryKey: iotKeys.dashboard(buildingId),
    queryFn: ({ signal }) => getIotDashboard(buildingId, signal),
    staleTime: 30_000,
  });
}

/** List sensors / devices (FR71). */
export function useSensors(params?: ListSensorsParams) {
  return useQuery({
    queryKey: iotKeys.sensorList(params),
    queryFn: ({ signal }) => listSensors(params, signal),
    staleTime: 30_000,
  });
}

/** Single sensor (FR71). */
export function useSensor(id: string) {
  return useQuery({
    queryKey: iotKeys.sensor(id),
    queryFn: ({ signal }) => getSensor(id, signal),
    enabled: !!id,
    staleTime: 60_000,
  });
}

/** Telemetry readings for a sensor (FR72). */
export function useSensorReadings(sensorId: string, params?: ListReadingsParams) {
  return useQuery({
    queryKey: iotKeys.readings(sensorId, params),
    queryFn: ({ signal }) => listSensorReadings(sensorId, params, signal),
    enabled: !!sensorId,
    staleTime: 15_000,
  });
}

/** Alerts for a sensor (FR74). */
export function useSensorAlerts(sensorId: string, params?: ListAlertsParams) {
  return useQuery({
    queryKey: iotKeys.alerts(sensorId, params),
    queryFn: ({ signal }) => listSensorAlerts(sensorId, params, signal),
    enabled: !!sensorId,
    staleTime: 15_000,
  });
}

// ============================================
// Mutations
// ============================================

/** Acknowledge an alert (FR74). Refreshes IoT caches on success. */
export function useAcknowledgeSensorAlert() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (alertId: string) => acknowledgeSensorAlert(alertId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: iotKeys.all });
    },
  });
}

/** Resolve an alert (FR74). Refreshes IoT caches on success. */
export function useResolveSensorAlert() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ alertId, resolvedValue }: { alertId: string; resolvedValue?: number | null }) =>
      resolveSensorAlert(alertId, resolvedValue),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: iotKeys.all });
    },
  });
}

/** Register a new sensor (FR71). Refreshes sensor lists + dashboard on success. */
export function useCreateSensor() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (req: CreateSensorRequest) => createSensor(req),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: iotKeys.all });
    },
  });
}

/** Update a sensor (FR71). Refreshes the affected sensor + lists on success. */
export function useUpdateSensor() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, data }: { id: string; data: UpdateSensorRequest }) => updateSensor(id, data),
    onSuccess: (_result, { id }) => {
      queryClient.invalidateQueries({ queryKey: iotKeys.sensor(id) });
      queryClient.invalidateQueries({ queryKey: iotKeys.sensors() });
      queryClient.invalidateQueries({ queryKey: iotKeys.dashboard() });
    },
  });
}

/** Delete a sensor (FR71). Refreshes sensor lists + dashboard on success. */
export function useDeleteSensor() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => deleteSensor(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: iotKeys.all });
    },
  });
}
