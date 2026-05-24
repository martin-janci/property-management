/**
 * MFA TanStack Query Hooks
 *
 * React hooks for Multi-Factor Authentication (UC-14.10, Epic 9 Story 9.1).
 * Wraps the raw MFA API functions with TanStack Query for caching and
 * mutation lifecycle management.
 */

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import {
  disableMfa,
  getMfaStatus,
  regenerateBackupCodes,
  setupMfa,
  verifyMfaSetup,
} from './api';
import type {
  DisableMfaRequest,
  RegenerateBackupCodesRequest,
  VerifyMfaRequest,
} from './types';

// ============================================
// Query Key Factory
// ============================================

export const mfaKeys = {
  all: ['mfa'] as const,
  status: () => [...mfaKeys.all, 'status'] as const,
};

// ============================================
// Query Hooks
// ============================================

/**
 * Hook to get the current user's MFA status.
 * GET /api/v1/auth/mfa/status
 */
export function useMfaStatus() {
  return useQuery({
    queryKey: mfaKeys.status(),
    queryFn: () => getMfaStatus(),
    staleTime: 60_000,
  });
}

// ============================================
// Mutation Hooks
// ============================================

/**
 * Hook to initiate MFA setup.
 * Returns TOTP secret, QR URI, and one-time backup codes.
 * POST /api/v1/auth/mfa/setup
 */
export function useMfaSetup() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: () => setupMfa(),
    onSuccess: () => {
      // Invalidate status — backend now has a pending TOTP secret
      queryClient.invalidateQueries({ queryKey: mfaKeys.status() });
    },
  });
}

/**
 * Hook to verify a TOTP code and activate MFA.
 * POST /api/v1/auth/mfa/verify
 */
export function useMfaVerify() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (request: VerifyMfaRequest) => verifyMfaSetup(request),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: mfaKeys.status() });
    },
  });
}

/**
 * Hook to disable MFA (requires a current TOTP or backup code).
 * POST /api/v1/auth/mfa/disable
 */
export function useMfaDisable() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (request: DisableMfaRequest) => disableMfa(request),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: mfaKeys.status() });
    },
  });
}

/**
 * Hook to regenerate backup codes (invalidates all existing codes).
 * POST /api/v1/auth/mfa/backup-codes/regenerate
 */
export function useMfaRegenerateBackupCodes() {
  return useMutation({
    mutationFn: (request: RegenerateBackupCodesRequest) => regenerateBackupCodes(request),
  });
}
