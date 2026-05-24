/**
 * Admin TanStack Query hooks — Phase 5 / Epic 10A-2.
 *
 * Mirrors the buildings module's "direct hook" pattern: function components
 * call `useAgencies(params)` and get a typed `useQuery` result. The hook
 * uses the shared token provider for auth (no per-call config plumbing).
 */

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import {
  acknowledgeHealthAlert,
  fetchHealthAlerts,
  fetchHealthDashboard,
  fetchMetricHistory,
  getOAuthClient,
  listAgencies,
  listOAuthClients,
  regenerateOAuthClientSecret,
  registerOAuthClient,
  revokeOAuthClient,
  updateHealthThreshold,
  updateOAuthClient,
} from './api';
import type {
  ListAgenciesParams,
  RegisterOAuthClientRequest,
  TimeRange,
  UpdateOAuthClientRequest,
  UpdateThresholdRequest,
} from './types';

// ============================================
// Query Key Factory
// ============================================

export const adminKeys = {
  all: ['admin'] as const,
  agencies: () => [...adminKeys.all, 'agencies'] as const,
  agencyList: (params?: ListAgenciesParams) => [...adminKeys.agencies(), 'list', params] as const,
  oauthClients: () => [...adminKeys.all, 'oauth', 'clients'] as const,
  oauthClient: (id: string) => [...adminKeys.oauthClients(), id] as const,
  health: () => [...adminKeys.all, 'health'] as const,
  healthDashboard: () => [...adminKeys.health(), 'dashboard'] as const,
  healthAlerts: (activeOnly: boolean) => [...adminKeys.health(), 'alerts', activeOnly] as const,
  metricHistory: (name: string, range: TimeRange) =>
    [...adminKeys.health(), 'history', name, range] as const,
};

// ============================================
// Hooks
// ============================================

/**
 * List agencies with optional pagination + free-text query.
 */
export function useAgencies(params?: ListAgenciesParams) {
  return useQuery({
    queryKey: adminKeys.agencyList(params),
    queryFn: ({ signal }) => listAgencies(params, signal),
    staleTime: 30_000,
  });
}

// ============================================
// OAuth Client hooks (Epic 10A-2)
// ============================================

export function useOAuthClients() {
  return useQuery({
    queryKey: adminKeys.oauthClients(),
    queryFn: ({ signal }) => listOAuthClients(signal),
    staleTime: 30_000,
  });
}

export function useOAuthClient(id: string) {
  return useQuery({
    queryKey: adminKeys.oauthClient(id),
    queryFn: ({ signal }) => getOAuthClient(id, signal),
    staleTime: 30_000,
    enabled: !!id,
  });
}

export function useRegisterOAuthClient() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (request: RegisterOAuthClientRequest) => registerOAuthClient(request),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: adminKeys.oauthClients() });
    },
  });
}

export function useUpdateOAuthClient(id: string) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (data: UpdateOAuthClientRequest) => updateOAuthClient(id, data),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: adminKeys.oauthClients() });
      void qc.invalidateQueries({ queryKey: adminKeys.oauthClient(id) });
    },
  });
}

export function useRevokeOAuthClient() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => revokeOAuthClient(id),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: adminKeys.oauthClients() });
    },
  });
}

export function useRegenerateOAuthClientSecret() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => regenerateOAuthClientSecret(id),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: adminKeys.oauthClients() });
    },
  });
}

// ============================================================
// Platform Health Monitoring hooks (Epic 10B.3)
// ============================================================

export function useHealthDashboard() {
  return useQuery({
    queryKey: adminKeys.healthDashboard(),
    queryFn: ({ signal }) => fetchHealthDashboard(signal),
    staleTime: 30_000,
    refetchInterval: 60_000,
  });
}

export function useHealthAlerts(activeOnly: boolean) {
  return useQuery({
    queryKey: adminKeys.healthAlerts(activeOnly),
    queryFn: ({ signal }) => fetchHealthAlerts(activeOnly, signal),
    staleTime: 30_000,
  });
}

export function useAcknowledgeAlert() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (alertId: string) => acknowledgeHealthAlert(alertId),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: adminKeys.healthDashboard() });
      void qc.invalidateQueries({ queryKey: adminKeys.health() });
    },
  });
}

export function useMetricHistory(metricName: string, range: TimeRange) {
  return useQuery({
    queryKey: adminKeys.metricHistory(metricName, range),
    queryFn: ({ signal }) => fetchMetricHistory(metricName, range, signal),
    staleTime: 30_000,
    enabled: !!metricName,
  });
}

export function useUpdateHealthThreshold() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ metricName, data }: { metricName: string; data: UpdateThresholdRequest }) =>
      updateHealthThreshold(metricName, data),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: adminKeys.healthDashboard() });
    },
  });
}
