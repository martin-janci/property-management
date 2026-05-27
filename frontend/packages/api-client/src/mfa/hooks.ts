/**
 * MFA TanStack Query Hooks
 *
 * React hooks for Two-Factor Authentication (UC-14, Epic 9, Story 9.1).
 */

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { disableMfa, getMfaStatus, regenerateBackupCodes, setupMfa, verifyMfaSetup } from './api';
import type { DisableMfaRequest, RegenerateBackupCodesRequest, VerifyMfaRequest } from './types';

// ============================================
// Query Key Factory
// ============================================

export const mfaKeys = {
  all: ['mfa'] as const,
  status: () => [...mfaKeys.all, 'status'] as const,
};

// ============================================
// Hooks
// ============================================

/**
 * Fetch the current MFA status for the authenticated user.
 *
 * Returns whether MFA is enabled, when it was enabled, and how many
 * backup codes remain.
 */
export function useMfaStatus() {
  return useQuery({
    queryKey: mfaKeys.status(),
    queryFn: () => getMfaStatus(),
    staleTime: 30_000,
  });
}

/**
 * Initiate MFA setup.
 *
 * On success the mutation result contains the TOTP secret, QR URI, and
 * one-time backup codes. Invalidates the MFA status cache so UI reflects
 * the pending state immediately.
 */
export function useMfaSetup() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: () => setupMfa(),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: mfaKeys.status() });
    },
  });
}

/**
 * Verify the TOTP code and complete MFA setup.
 *
 * After verification the MFA status is refreshed so the page can
 * transition to the "enabled" state without a manual reload.
 */
export function useMfaVerify() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (data: VerifyMfaRequest) => verifyMfaSetup(data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: mfaKeys.status() });
    },
  });
}

/**
 * Disable MFA.
 *
 * Requires the user to confirm with their current TOTP code or a backup
 * code. On success the MFA status cache is invalidated.
 */
export function useMfaDisable() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (data: DisableMfaRequest) => disableMfa(data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: mfaKeys.status() });
    },
  });
}

/**
 * Regenerate backup codes.
 *
 * Requires the user to confirm with their current TOTP code. The new codes
 * are returned in the mutation result — they must be shown to the user once
 * and are not stored in plain text on the server.
 */
export function useMfaRegenerateBackupCodes() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (data: RegenerateBackupCodesRequest) => regenerateBackupCodes(data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: mfaKeys.status() });
    },
  });
}
