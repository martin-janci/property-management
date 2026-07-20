/**
 * Layout Query Hooks
 *
 * TanStack Query hooks for resolved screen layouts and tenant overrides.
 */

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { getOrg } from '../auth';
import {
  fetchResolvedLayout,
  fetchTenantLayout,
  type ResolvedScreen,
  saveTenantLayoutOverride,
  type TenantLayoutEnvelope,
  type TenantOverride,
} from './api';

export const layoutKeys = {
  all: ['layout'] as const,
  // Keys include the active org so an org switch never serves the previous
  // org's cached layout (keys without the org would collide across orgs).
  resolved: (screen: string, platform: string) =>
    [...layoutKeys.all, getOrg() ?? 'no-org', 'resolved', screen, platform] as const,
  tenant: (screen: string) => [...layoutKeys.all, getOrg() ?? 'no-org', 'tenant', screen] as const,
};

export function useResolvedLayout(screen: string, platform: 'web' | 'mobile' = 'web') {
  return useQuery<ResolvedScreen>({
    queryKey: layoutKeys.resolved(screen, platform),
    queryFn: () => fetchResolvedLayout(screen, platform),
    staleTime: 60_000,
    retry: 1,
  });
}

export function useTenantLayout(screen: string) {
  return useQuery<TenantLayoutEnvelope>({
    queryKey: layoutKeys.tenant(screen),
    queryFn: () => fetchTenantLayout(screen),
    staleTime: 60_000,
    retry: 1,
  });
}

export function useSaveTenantLayoutOverride(screen: string) {
  const queryClient = useQueryClient();
  return useMutation<unknown, Error, TenantOverride>({
    mutationFn: (override: TenantOverride) => saveTenantLayoutOverride(screen, override),
    onSuccess: () => {
      // Invalidate ALL layout keys (tenant + resolved) — a saved override
      // changes what /resolved returns for this org too.
      queryClient.invalidateQueries({ queryKey: layoutKeys.all });
    },
  });
}
