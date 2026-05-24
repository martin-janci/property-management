/**
 * MFA API Client
 *
 * Direct API functions for the Multi-Factor Authentication endpoints (UC-14, Epic 9, Story 9.1).
 * Routes: /api/v1/auth/mfa/*
 */

import { getToken } from '../auth';
import type {
  MfaDisableRequest,
  MfaDisableResponse,
  MfaRegenerateBackupCodesRequest,
  MfaRegenerateBackupCodesResponse,
  MfaSetupResponse,
  MfaStatusResponse,
  MfaVerifyRequest,
  MfaVerifyResponse,
} from './types';

const _win = typeof window !== 'undefined' ? (window as unknown as Record<string, unknown>) : {};
const MFA_BASE = `${_win.__API_BASE_URL__ ? String(_win.__API_BASE_URL__) : ''}/api/v1/auth/mfa`;

function getAuthHeaders(): Record<string, string> {
  const token = getToken();
  return token
    ? { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' }
    : { 'Content-Type': 'application/json' };
}

async function mfaRequest<T>(url: string, options: RequestInit = {}): Promise<T> {
  const response = await fetch(url, {
    ...options,
    headers: {
      ...getAuthHeaders(),
      ...options.headers,
    },
  });

  if (!response.ok) {
    const error = await response.json().catch(() => ({}));
    throw new Error(error.message || `HTTP error ${response.status}`);
  }

  if (response.status === 204) {
    return undefined as T;
  }

  return response.json();
}

/**
 * Initiate MFA setup.
 * Returns a TOTP secret, QR code URI, and one-time backup codes.
 * POST /api/v1/auth/mfa/setup
 */
export async function mfaSetup(): Promise<MfaSetupResponse> {
  return mfaRequest<MfaSetupResponse>(MFA_BASE + '/setup', { method: 'POST' });
}

/**
 * Verify the TOTP code to complete MFA setup.
 * POST /api/v1/auth/mfa/verify
 */
export async function mfaVerify(data: MfaVerifyRequest): Promise<MfaVerifyResponse> {
  return mfaRequest<MfaVerifyResponse>(MFA_BASE + '/verify', {
    method: 'POST',
    body: JSON.stringify(data),
  });
}

/**
 * Disable MFA (requires current TOTP or backup code).
 * POST /api/v1/auth/mfa/disable
 */
export async function mfaDisable(data: MfaDisableRequest): Promise<MfaDisableResponse> {
  return mfaRequest<MfaDisableResponse>(MFA_BASE + '/disable', {
    method: 'POST',
    body: JSON.stringify(data),
  });
}

/**
 * Get current MFA status for the authenticated user.
 * GET /api/v1/auth/mfa/status
 */
export async function mfaStatus(signal?: AbortSignal): Promise<MfaStatusResponse> {
  return mfaRequest<MfaStatusResponse>(MFA_BASE + '/status', { method: 'GET', signal });
}

/**
 * Regenerate backup codes (requires current TOTP code).
 * POST /api/v1/auth/mfa/backup-codes/regenerate
 */
export async function mfaRegenerateBackupCodes(
  data: MfaRegenerateBackupCodesRequest
): Promise<MfaRegenerateBackupCodesResponse> {
  return mfaRequest<MfaRegenerateBackupCodesResponse>(MFA_BASE + '/backup-codes/regenerate', {
    method: 'POST',
    body: JSON.stringify(data),
  });
}
