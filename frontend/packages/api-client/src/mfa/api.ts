/**
 * MFA API Client
 *
 * API functions for Multi-Factor Authentication (UC-14.10, Epic 9 Story 9.1).
 * Wires to /api/v1/auth/mfa/* on api-server.
 */

import { getToken } from '../auth';
import type {
  DisableMfaRequest,
  DisableMfaResponse,
  MfaSetupResponse,
  MfaStatusResponse,
  RegenerateBackupCodesRequest,
  RegenerateBackupCodesResponse,
  VerifyMfaRequest,
  VerifyMfaResponse,
} from './types';

const API_BASE = '/api/v1/auth/mfa';

function getAuthHeaders(): HeadersInit {
  const token = getToken();
  if (!token) {
    throw new Error('MFA API requires authentication');
  }
  return {
    'Content-Type': 'application/json',
    Authorization: `Bearer ${token}`,
  };
}

async function parseResponse<T>(response: Response): Promise<T> {
  if (!response.ok) {
    const body = await response.json().catch(() => ({ message: response.statusText }));
    const err = new Error(body.message || 'MFA API request failed') as Error & {
      status: number;
      code: string;
    };
    err.status = response.status;
    err.code = body.error ?? body.code ?? 'UNKNOWN_ERROR';
    throw err;
  }
  return response.json() as Promise<T>;
}

/** Initiate MFA setup. POST /api/v1/auth/mfa/setup */
export async function setupMfa(): Promise<MfaSetupResponse> {
  const response = await fetch(`${API_BASE}/setup`, {
    method: 'POST',
    headers: getAuthHeaders(),
  });
  return parseResponse<MfaSetupResponse>(response);
}

/** Verify a TOTP code and activate MFA. POST /api/v1/auth/mfa/verify */
export async function verifyMfaSetup(request: VerifyMfaRequest): Promise<VerifyMfaResponse> {
  const response = await fetch(`${API_BASE}/verify`, {
    method: 'POST',
    headers: getAuthHeaders(),
    body: JSON.stringify(request),
  });
  return parseResponse<VerifyMfaResponse>(response);
}

/** Disable MFA. POST /api/v1/auth/mfa/disable */
export async function disableMfa(request: DisableMfaRequest): Promise<DisableMfaResponse> {
  const response = await fetch(`${API_BASE}/disable`, {
    method: 'POST',
    headers: getAuthHeaders(),
    body: JSON.stringify(request),
  });
  return parseResponse<DisableMfaResponse>(response);
}

/** Get current user MFA status. GET /api/v1/auth/mfa/status */
export async function getMfaStatus(): Promise<MfaStatusResponse> {
  const response = await fetch(`${API_BASE}/status`, {
    method: 'GET',
    headers: getAuthHeaders(),
  });
  return parseResponse<MfaStatusResponse>(response);
}

/** Regenerate backup codes. POST /api/v1/auth/mfa/backup-codes/regenerate */
export async function regenerateBackupCodes(
  request: RegenerateBackupCodesRequest
): Promise<RegenerateBackupCodesResponse> {
  const response = await fetch(`${API_BASE}/backup-codes/regenerate`, {
    method: 'POST',
    headers: getAuthHeaders(),
    body: JSON.stringify(request),
  });
  return parseResponse<RegenerateBackupCodesResponse>(response);
}
