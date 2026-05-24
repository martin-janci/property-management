/**
 * MFA Module
 *
 * API client, hooks, and types for Multi-Factor Authentication (UC-14.10, Epic 9).
 */

// Raw API functions
export {
  disableMfa,
  getMfaStatus,
  regenerateBackupCodes,
  setupMfa,
  verifyMfaSetup,
} from './api';

// TanStack Query hooks
export {
  mfaKeys,
  useMfaDisable,
  useMfaRegenerateBackupCodes,
  useMfaSetup,
  useMfaStatus,
  useMfaVerify,
} from './hooks';

// Types
export type {
  DisableMfaRequest,
  DisableMfaResponse,
  MfaErrorCode,
  MfaSetupResponse,
  MfaStatusResponse,
  RegenerateBackupCodesRequest,
  RegenerateBackupCodesResponse,
  VerifyMfaRequest,
  VerifyMfaResponse,
} from './types';
