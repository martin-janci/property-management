/**
 * OAuth User Grants TanStack Query Hooks
 *
 * React hooks for managing the current user's OAuth app authorizations
 * (Epic 10A, Story 10A-3).
 */

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { listUserGrants, revokeUserGrant } from './api';

// ============================================
// Query Key Factory
// ============================================

export const oauthGrantKeys = {
  all: ['oauth-grants'] as const,
  list: () => [...oauthGrantKeys.all, 'list'] as const,
};

// ============================================
// Hooks
// ============================================

/**
 * Fetch all active OAuth grants for the authenticated user.
 *
 * Each grant represents one third-party application that has been
 * authorized to access the user's account.
 */
export function useUserGrants() {
  return useQuery({
    queryKey: oauthGrantKeys.list(),
    queryFn: () => listUserGrants(),
    staleTime: 30_000,
  });
}

/**
 * Revoke a user's OAuth grant for a specific client.
 *
 * On success the grant list is invalidated so the UI reflects the
 * removal immediately without a manual reload.
 */
export function useRevokeUserGrant() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (clientId: string) => revokeUserGrant(clientId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: oauthGrantKeys.list() });
    },
  });
}
