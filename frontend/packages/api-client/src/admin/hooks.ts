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
  createSystemAnnouncement,
  deleteSystemAnnouncement,
  fetchHealthAlerts,
  fetchHealthDashboard,
  fetchMetricHistory,
  fetchSupportData,
  getAgency,
  getOAuthClient,
  getSystemAnnouncement,
  listAgencies,
  listOAuthClients,
  listSystemAnnouncements,
  regenerateOAuthClientSecret,
  registerOAuthClient,
  revokeOAuthClient,
  updateHealthThreshold,
  updateOAuthClient,
  updateSystemAnnouncement,
} from './api';
import type {
  CreateSystemAnnouncementRequest,
  ListAgenciesParams,
  ListSystemAnnouncementsParams,
  RegisterOAuthClientRequest,
  TimeRange,
  UpdateOAuthClientRequest,
  UpdateSystemAnnouncementRequest,
  UpdateThresholdRequest,
} from './types';

// ============================================
// Query Key Factory
// ============================================

export const adminKeys = {
  all: ['admin'] as const,
  agencies: () => [...adminKeys.all, 'agencies'] as const,
  agencyList: (params?: ListAgenciesParams) => [...adminKeys.agencies(), 'list', params] as const,
  agency: (id: string) => [...adminKeys.agencies(), 'detail', id] as const,
  oauthClients: () => [...adminKeys.all, 'oauth', 'clients'] as const,
  oauthClient: (id: string) => [...adminKeys.oauthClients(), id] as const,
  health: () => [...adminKeys.all, 'health'] as const,
  healthDashboard: () => [...adminKeys.health(), 'dashboard'] as const,
  healthAlerts: (activeOnly: boolean) => [...adminKeys.health(), 'alerts', activeOnly] as const,
  metricHistory: (name: string, range: TimeRange) =>
    [...adminKeys.health(), 'history', name, range] as const,
  systemAnnouncements: () => [...adminKeys.all, 'system-announcements'] as const,
  systemAnnouncementList: (params?: ListSystemAnnouncementsParams) =>
    [...adminKeys.systemAnnouncements(), 'list', params] as const,
  systemAnnouncement: (id: string) => [...adminKeys.systemAnnouncements(), id] as const,
  supportData: () => [...adminKeys.all, 'support-data'] as const,
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

/**
 * Detailed view of a single agency (org drill-in). Disabled until `id` is set.
 */
export function useAgency(id: string) {
  return useQuery({
    queryKey: adminKeys.agency(id),
    queryFn: ({ signal }) => getAgency(id, signal),
    staleTime: 30_000,
    enabled: !!id,
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

// ============================================================
// System Announcements hooks (Epic 10B.4)
// ============================================================

export function useSystemAnnouncements(params?: ListSystemAnnouncementsParams) {
  return useQuery({
    queryKey: adminKeys.systemAnnouncementList(params),
    queryFn: ({ signal }) => listSystemAnnouncements(params, signal),
    staleTime: 30_000,
  });
}

export function useSystemAnnouncement(id: string) {
  return useQuery({
    queryKey: adminKeys.systemAnnouncement(id),
    queryFn: ({ signal }) => getSystemAnnouncement(id, signal),
    staleTime: 30_000,
    enabled: !!id,
  });
}

export function useCreateSystemAnnouncement() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (data: CreateSystemAnnouncementRequest) => createSystemAnnouncement(data),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: adminKeys.systemAnnouncements() });
    },
  });
}

export function useUpdateSystemAnnouncement() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ id, data }: { id: string; data: UpdateSystemAnnouncementRequest }) =>
      updateSystemAnnouncement(id, data),
    onSuccess: (_result, { id }) => {
      void qc.invalidateQueries({ queryKey: adminKeys.systemAnnouncements() });
      void qc.invalidateQueries({ queryKey: adminKeys.systemAnnouncement(id) });
    },
  });
}

export function useDeleteSystemAnnouncement() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => deleteSystemAnnouncement(id),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: adminKeys.systemAnnouncements() });
    },
  });
}

// ============================================================
// Support Data hooks (Epic 10B.5)
// ============================================================

/**
 * Fetches platform-wide tenant diagnostics from
 * `GET /api/v1/platform-admin/support-data`.
 *
 * Requires `audit_read` capability. Refreshes every 60 s.
 */
export function useSupportData() {
  return useQuery({
    queryKey: adminKeys.supportData(),
    queryFn: ({ signal }) => fetchSupportData(signal),
    staleTime: 30_000,
    refetchInterval: 60_000,
  });
}
