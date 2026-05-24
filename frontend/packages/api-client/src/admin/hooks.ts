/**
 * Admin TanStack Query hooks — Phase 5 / Epic 10A-2.
 *
 * Mirrors the buildings module's "direct hook" pattern: function components
 * call `useAgencies(params)` and get a typed `useQuery` result. The hook
 * uses the shared token provider for auth (no per-call config plumbing).
 */

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import {
  getOAuthClient,
  listAgencies,
  listOAuthClients,
  regenerateOAuthClientSecret,
  registerOAuthClient,
  revokeOAuthClient,
  updateOAuthClient,
} from './api';
import type {
  ListAgenciesParams,
  RegisterOAuthClientRequest,
  UpdateOAuthClientRequest,
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
