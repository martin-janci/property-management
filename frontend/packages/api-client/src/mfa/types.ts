/**
 * MFA Types
 *
 * Type definitions for the Multi-Factor Authentication API (UC-14, Epic 9, Story 9.1).
 * Mirrors the backend MFA route structs in api-server/src/routes/mfa.rs.
 */

/** Response from POST /api/v1/auth/mfa/setup */
export interface MfaSetupResponse {
  /** The TOTP secret (only shown once, for manual QR/manual entry) */
  secret: string;
  /** otpauth:// URI for QR code generation */
  qrUri: string;
  /** Backup codes (only shown once — user must save them) */
  backupCodes: string[];
}

/** Request body for POST /api/v1/auth/mfa/verify */
export interface MfaVerifyRequest {
  /** The 6-digit TOTP code from the authenticator app */
  code: string;
}

/** Response from POST /api/v1/auth/mfa/verify */
export interface MfaVerifyResponse {
  message: string;
  enabled: boolean;
}

/** Request body for POST /api/v1/auth/mfa/disable */
export interface MfaDisableRequest {
  /** Current TOTP code or backup code to confirm intent */
  code: string;
}

/** Response from POST /api/v1/auth/mfa/disable */
export interface MfaDisableResponse {
  message: string;
}

/** Response from GET /api/v1/auth/mfa/status */
export interface MfaStatusResponse {
  enabled: boolean;
  /** ISO-8601 timestamp when MFA was enabled, or null if disabled */
  enabledAt: string | null;
  backupCodesRemaining: number;
}

/** Request body for POST /api/v1/auth/mfa/backup-codes/regenerate */
export interface MfaRegenerateBackupCodesRequest {
  /** Current TOTP code to authorize regeneration */
  code: string;
}

/** Response from POST /api/v1/auth/mfa/backup-codes/regenerate */
export interface MfaRegenerateBackupCodesResponse {
  /** New backup codes (only shown once) */
  backupCodes: string[];
  message: string;
}
