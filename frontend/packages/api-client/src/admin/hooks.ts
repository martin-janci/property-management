/**
 * Admin TanStack Query hooks — Phase 5.
 *
 * Mirrors the buildings module's "direct hook" pattern: function components
 * call `useAgencies(params)` and get a typed `useQuery` result. The hook
 * uses the shared token provider for auth (no per-call config plumbing).
 */

import { useQuery } from '@tanstack/react-query';
import { listAgencies } from './api';
import type { ListAgenciesParams } from './types';

// ============================================
// Query Key Factory
// ============================================

export const adminKeys = {
  all: ['admin'] as const,
  agencies: () => [...adminKeys.all, 'agencies'] as const,
  agencyList: (params?: ListAgenciesParams) => [...adminKeys.agencies(), 'list', params] as const,
};

// ============================================
// Hooks
// ============================================

/**
 * List agencies with optional pagination + free-text query.
 *
 * Backed by `GET /api/v1/admin/agencies`. Capability-gated on the server
 * (`agencies_read`); the client surface assumes the caller already knows the
 * principal is platform-kind (the admin router enforces this).
 */
export function useAgencies(params?: ListAgenciesParams) {
  return useQuery({
    queryKey: adminKeys.agencyList(params),
    queryFn: ({ signal }) => listAgencies(params, signal),
    staleTime: 30_000,
  });
}
