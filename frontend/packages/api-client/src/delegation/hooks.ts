/**
 * Delegation TanStack Query hooks (Epic 3, Story 3.4).
 *
 * React hooks over the delegation API. List queries are cached under
 * `delegationKeys`; every mutation invalidates the affected lists so the UI
 * reflects accept/decline/revoke/create without a manual reload.
 */

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import {
  acceptDelegation,
  createDelegation,
  declineDelegation,
  getDelegation,
  listMyDelegations,
  listReceivedDelegations,
  revokeDelegation,
} from './api';
import type { CreateDelegationRequest, ListDelegationsQuery } from './types';

// ============================================
// Query Key Factory
// ============================================

export const delegationKeys = {
  all: ['delegations'] as const,
  mine: (query?: ListDelegationsQuery) => [...delegationKeys.all, 'mine', query ?? {}] as const,
  received: () => [...delegationKeys.all, 'received'] as const,
  detail: (id: string) => [...delegationKeys.all, 'detail', id] as const,
};

// ============================================
// Queries
// ============================================

/** Delegations the current user created (as owner). */
export function useMyDelegations(query: ListDelegationsQuery = {}) {
  return useQuery({
    queryKey: delegationKeys.mine(query),
    queryFn: () => listMyDelegations(query),
    staleTime: 30_000,
  });
}

/** Delegations the current user received (as delegate). */
export function useReceivedDelegations() {
  return useQuery({
    queryKey: delegationKeys.received(),
    queryFn: () => listReceivedDelegations(),
    staleTime: 30_000,
  });
}

/** A single delegation by id. */
export function useDelegation(id: string | undefined) {
  return useQuery({
    queryKey: delegationKeys.detail(id ?? ''),
    queryFn: () => getDelegation(id as string),
    enabled: Boolean(id),
  });
}

// ============================================
// Mutations
// ============================================

/** Invalidate every delegation list + detail after a mutation. */
function useInvalidateDelegations() {
  const queryClient = useQueryClient();
  return () => queryClient.invalidateQueries({ queryKey: delegationKeys.all });
}

/** Create a delegation (owner action). Invalidates the "mine" list. */
export function useCreateDelegation() {
  const invalidate = useInvalidateDelegations();
  return useMutation({
    mutationFn: (request: CreateDelegationRequest) => createDelegation(request),
    onSuccess: invalidate,
  });
}

/** Accept a received delegation (delegate action). */
export function useAcceptDelegation() {
  const invalidate = useInvalidateDelegations();
  return useMutation({
    mutationFn: (id: string) => acceptDelegation(id),
    onSuccess: invalidate,
  });
}

/** Decline a received delegation (delegate action). */
export function useDeclineDelegation() {
  const invalidate = useInvalidateDelegations();
  return useMutation({
    mutationFn: (id: string) => declineDelegation(id),
    onSuccess: invalidate,
  });
}

/** Revoke a delegation you created (owner action). */
export function useRevokeDelegation() {
  const invalidate = useInvalidateDelegations();
  return useMutation({
    mutationFn: (id: string) => revokeDelegation(id),
    onSuccess: invalidate,
  });
}
