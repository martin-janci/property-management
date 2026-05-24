/**
 * MFA Types
 *
 * Type definitions for the Multi-Factor Authentication API (UC-14.10, Epic 9).
 */

// ============================================
// MFA Setup (Story 9.1)
// ============================================

/** Response from POST /api/v1/auth/mfa/setup */
export interface MfaSetupResponse {
  /** The TOTP secret (only shown once, for manual entry) */
  secret: string;
  /** URI for QR code generation (otpauth:// scheme) */
  qrUri: string;
  /** Backup codes (only shown once, user must save them) */
  backupCodes: string[];
}

// ============================================
// MFA Verify (Story 9.1)
// ============================================

/** Request body for POST /api/v1/auth/mfa/verify */
export interface VerifyMfaRequest {
  /** The 6-digit TOTP code from authenticator app */
  code: string;
}

/** Response from POST /api/v1/auth/mfa/verify */
export interface VerifyMfaResponse {
  /** Success message */
  message: string;
  /** Whether MFA is now enabled */
  enabled: boolean;
}

// ============================================
// MFA Disable (Story 9.2)
// ============================================

/** Request body for POST /api/v1/auth/mfa/disable */
export interface DisableMfaRequest {
  /** Current TOTP code or backup code to confirm */
  code: string;
}

/** Response from POST /api/v1/auth/mfa/disable */
export interface DisableMfaResponse {
  /** Success message */
  message: string;
}

// ============================================
// MFA Status (Story 9.1)
// ============================================

/** Response from GET /api/v1/auth/mfa/status */
export interface MfaStatusResponse {
  /** Whether MFA is enabled */
  enabled: boolean;
  /** ISO-8601 timestamp when MFA was enabled (if enabled) */
  enabledAt: string | null;
  /** Remaining backup codes count */
  backupCodesRemaining: number;
}

// ============================================
// Backup Codes (Story 9.2)
// ============================================

/** Request body for POST /api/v1/auth/mfa/backup-codes/regenerate */
export interface RegenerateBackupCodesRequest {
  /** Current TOTP code to authorize regeneration */
  code: string;
}

/** Response from POST /api/v1/auth/mfa/backup-codes/regenerate */
export interface RegenerateBackupCodesResponse {
  /** New backup codes (only shown once) */
  backupCodes: string[];
  /** Success message */
  message: string;
}

// ============================================
// Error codes
// ============================================

export type MfaErrorCode =
  | 'MFA_ALREADY_ENABLED'
  | 'MFA_NOT_ENABLED'
  | 'MFA_NOT_SETUP'
  | 'INVALID_CODE'
  | 'ENCRYPTION_NOT_CONFIGURED'
  | 'UNKNOWN_ERROR';
