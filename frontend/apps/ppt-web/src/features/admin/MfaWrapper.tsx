/**
 * MfaWrapper (Phase 6 B6).
 *
 * Gate that wraps the admin sub-tree. If the current platform principal has
 * not yet enrolled in MFA (`mfa_enabled_at IS NULL` on the server), this
 * wrapper intercepts navigation and renders `<MfaEnrollmentScreen>` until
 * enrollment is complete.
 *
 * Enrollment status is determined by calling
 * `GET /api/v1/auth/mfa/status` (the existing endpoint). When `enabled` is
 * true we render children immediately; when false we show enrollment.
 *
 * The fetch is intentionally simple (no TanStack Query) so this file has no
 * external data-layer coupling. Once the admin-client grows an MFA status
 * hook this can be swapped out.
 */

import { MfaEnrollmentScreen, type MfaEnrollmentLabels } from '@ppt/admin-ui';
import { type ReactElement, type ReactNode, useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useAuth } from '../../contexts';

type EnrollmentState = 'loading' | 'enrolled' | 'not_enrolled';

export interface MfaWrapperProps {
  /** Admin sub-tree to render when MFA is enrolled. */
  children: ReactNode;
}

export function MfaWrapper({ children }: MfaWrapperProps): ReactElement {
  const { t } = useTranslation();
  const { getAccessToken } = useAuth();
  const [state, setState] = useState<EnrollmentState>('loading');

  const authHeaders = useCallback((): Record<string, string> => {
    const token = getAccessToken();
    return token ? { Authorization: `Bearer ${token}` } : {};
  }, [getAccessToken]);

  // ── Check enrollment status on mount ──────────────────────────────────────
  useEffect(() => {
    let cancelled = false;
    fetch('/api/v1/auth/mfa/status', { headers: authHeaders() })
      .then((r) => r.json())
      .then((body: { enabled?: boolean }) => {
        if (!cancelled) {
          setState(body.enabled === true ? 'enrolled' : 'not_enrolled');
        }
      })
      .catch(() => {
        if (!cancelled) {
          // On fetch failure default to showing enrollment — safer than
          // allowing access to a capability-gated area without MFA.
          setState('not_enrolled');
        }
      });
    return () => { cancelled = true; };
  }, [authHeaders]);

  // ── Enrollment callbacks ──────────────────────────────────────────────────

  const handleStart = useCallback(async () => {
    const res = await fetch('/api/v1/admin/mfa/enroll/start', {
      method: 'POST',
      headers: authHeaders(),
    });
    if (!res.ok) {
      const body = (await res.json().catch(() => ({}))) as { error?: string };
      throw new Error(body.error ?? `Enroll start failed (${res.status})`);
    }
    return res.json() as Promise<{ uri: string; secret: string }>;
  }, [authHeaders]);

  const handleVerify = useCallback(
    async (code: string): Promise<{ recovery_codes: string[] } | false> => {
      const res = await fetch('/api/v1/admin/mfa/enroll/verify', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', ...authHeaders() },
        body: JSON.stringify({ code }),
      });
      if (res.status === 422) return false; // invalid code — show error in component
      if (!res.ok) {
        const body = (await res.json().catch(() => ({}))) as { error?: string };
        throw new Error(body.error ?? `Enroll verify failed (${res.status})`);
      }
      return res.json() as Promise<{ recovery_codes: string[] }>;
    },
    [authHeaders]
  );

  const handleComplete = useCallback(() => {
    setState('enrolled');
  }, []);

  // ── Labels ────────────────────────────────────────────────────────────────
  // All keys are optional — fall back to English defaults if translations are
  // not yet present.

  const labels: MfaEnrollmentLabels = {
    stepScanTitle: t('admin.mfa.enroll.stepScanTitle', {
      defaultValue: 'Set up two-factor authentication',
    }),
    stepScanDescription: t('admin.mfa.enroll.stepScanDescription', {
      defaultValue:
        'Scan the QR code below with your authenticator app (Google Authenticator, Authy, etc.).',
    }),
    manualEntryLabel: t('admin.mfa.enroll.manualEntryLabel', {
      defaultValue: 'Or enter this code manually:',
    }),
    next: t('admin.mfa.enroll.next', { defaultValue: 'Next' }),
    stepVerifyTitle: t('admin.mfa.enroll.stepVerifyTitle', {
      defaultValue: 'Verify your authenticator',
    }),
    stepVerifyDescription: t('admin.mfa.enroll.stepVerifyDescription', {
      defaultValue: 'Enter the 6-digit code from your authenticator app to confirm setup.',
    }),
    codeLabel: t('admin.mfa.enroll.codeLabel', { defaultValue: 'Verification code' }),
    verify: t('admin.mfa.enroll.verify', { defaultValue: 'Verify' }),
    verifying: t('admin.mfa.enroll.verifying', { defaultValue: 'Verifying…' }),
    invalidFormat: t('admin.mfa.enroll.invalidFormat', {
      defaultValue: 'Enter the 6-digit code from your authenticator app.',
    }),
    invalidCode: t('admin.mfa.enroll.invalidCode', {
      defaultValue: 'That code did not match. Try again.',
    }),
    verificationFailed: t('admin.mfa.enroll.verificationFailed', {
      defaultValue: 'Verification failed.',
    }),
    stepCodesTitle: t('admin.mfa.enroll.stepCodesTitle', {
      defaultValue: 'Save your recovery codes',
    }),
    stepCodesDescription: t('admin.mfa.enroll.stepCodesDescription', {
      defaultValue:
        'These 10 codes can each be used once if you lose access to your authenticator. Store them somewhere safe — they will not be shown again.',
    }),
    savedCodes: t('admin.mfa.enroll.savedCodes', {
      defaultValue: "I've saved these codes",
    }),
  };

  // ── Render ────────────────────────────────────────────────────────────────

  if (state === 'loading') {
    return (
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          minHeight: '100vh',
          fontFamily: 'system-ui, sans-serif',
          color: '#555',
        }}
      >
        Loading…
      </div>
    );
  }

  if (state === 'not_enrolled') {
    return (
      <MfaEnrollmentScreen
        onStart={handleStart}
        onVerify={handleVerify}
        onComplete={handleComplete}
        labels={labels}
      />
    );
  }

  // state === 'enrolled'
  return <>{children}</>;
}

MfaWrapper.displayName = 'MfaWrapper';
