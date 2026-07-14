# Story 9.1: TOTP Two-Factor Authentication Setup

Status: done

## Story

As a **security-conscious user**,
I want to **enable two-factor authentication**,
So that **my account is protected even if password is compromised**.

## Acceptance Criteria

1. **AC-1: Enable 2FA Setup**
   - Given a user enables 2FA
   - When they scan QR code with authenticator app
   - Then they enter verification code to confirm setup
   - And 2FA is activated for their account

2. **AC-2: Login with 2FA**
   - Given a user with 2FA logs in
   - When they enter correct password
   - Then they're prompted for TOTP code
   - And login only succeeds with valid code

3. **AC-3: Backup Codes Recovery**
   - Given a user loses their authenticator
   - When they use backup codes
   - Then they can access their account
   - And are prompted to set up 2FA again

## Tasks / Subtasks

- [ ] Task 1: Database Schema & Migrations (AC: 1, 2, 3)
  - [ ] 1.1 Create `user_2fa` table: id (UUID), user_id (FK), secret (encrypted), enabled (boolean), enabled_at, backup_codes (JSONB)
  - [ ] 1.2 Add unique constraint on user_id
  - [ ] 1.3 Add RLS policy - users can only access their own 2FA settings
  - [ ] 1.4 Create index on user_id for fast lookup

- [ ] Task 2: Backend Domain Models & Repository (AC: 1, 2, 3)
  - [ ] 2.1 Create TwoFactorAuth model with user_id, secret, enabled, enabled_at, backup_codes
  - [ ] 2.2 Create DTOs: SetupTwoFactorRequest, VerifyTwoFactorRequest, BackupCode, TwoFactorStatus
  - [ ] 2.3 Implement TwoFactorRepository: create, get_by_user_id, enable, disable, use_backup_code

- [ ] Task 3: TOTP Service Implementation (AC: 1, 2)
  - [ ] 3.1 Add totp-rs crate to dependencies
  - [ ] 3.2 Create TotpService for secret generation and code verification
  - [ ] 3.3 Implement generate_secret() returning base32 secret
  - [ ] 3.4 Implement verify_code(secret, code) with 30-second window
  - [ ] 3.5 Implement generate_qr_data(email, secret) for QR code URI

- [ ] Task 4: Backup Codes Service (AC: 3)
  - [ ] 4.1 Implement generate_backup_codes() returning 10 codes
  - [ ] 4.2 Hash backup codes before storage (like password hashing pattern)
  - [ ] 4.3 Implement verify_and_consume_backup_code()

- [ ] Task 5: Backend API Handlers (AC: 1, 2, 3)
  - [ ] 5.1 POST /api/v1/auth/mfa/setup - initiate 2FA setup (returns QR code URI + backup codes)
  - [ ] 5.2 POST /api/v1/auth/mfa/verify - verify TOTP code to complete setup
  - [ ] 5.3 POST /api/v1/auth/mfa/disable - disable 2FA (requires current code)
  - [ ] 5.4 GET /api/v1/auth/mfa/status - check if 2FA is enabled
  - [ ] 5.5 Update login endpoint to check for 2FA and require code
  - [ ] 5.6 POST /api/v1/auth/login/verify-mfa - verify MFA code during login

- [ ] Task 6: Frontend API Client (AC: 1, 2, 3)
  - [ ] 6.1 Create security/types.ts with TwoFactorSetupResponse, VerifyMfaRequest, etc.
  - [ ] 6.2 Create security/api.ts with fetch functions
  - [ ] 6.3 Create security/hooks.ts with useSetupMfa, useVerifyMfa, useMfaStatus

- [ ] Task 7: Frontend Components (AC: 1, 3)
  - [ ] 7.1 Create TwoFactorSetupPage component
  - [ ] 7.2 Create QRCodeDisplay component (using qrcode library)
  - [ ] 7.3 Create BackupCodesDisplay component
  - [ ] 7.4 Create TOTPCodeInput component (6-digit input)
  - [ ] 7.5 Create TwoFactorStatusCard for settings display

- [ ] Task 8: Frontend Login Flow Updates (AC: 2)
  - [ ] 8.1 Update login flow to handle MFA_REQUIRED response
  - [ ] 8.2 Create MfaVerificationStep component
  - [ ] 8.3 Handle backup code option in MFA step

## Dev Notes

### Architecture Requirements
- TOTP standard: RFC 6238 with 30-second time step
- Secret storage: encrypt at rest, only return during initial setup
- Backup codes: 10 codes, 8 characters each, hashed like passwords
- Rate limiting: max 5 failed MFA attempts before lockout

### Technical Specifications
- Backend: totp-rs crate for TOTP implementation
- Frontend: qrcode.react for QR generation
- Secret format: Base32 encoded, 160 bits
- Code format: 6 digits, 30-second validity

### Security Considerations
- Never log secrets or backup codes
- Secrets only returned once during setup
- Backup codes shown once, then hashed
- Session invalidation on 2FA enable/disable

### References
- [Source: _bmad-output/epics.md#Epic-9-Story-9.1]
- [RFC 6238: TOTP Algorithm](https://tools.ietf.org/html/rfc6238)

## Dev Agent Record

### Agent Model Used

Claude Opus 4.5 (claude-opus-4-5-20251101)

### Debug Log References

N/A

### Completion Notes List

(To be filled during implementation)

### File List

(To be filled during implementation)

## Change Log

| Date | Change |
|------|--------|
| 2025-12-21 | Story created |
