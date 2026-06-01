/**
 * MfaWrapper (admin-web port).
 *
 * Mounts `<MfaChallengeProvider>` from `@ppt/admin-ui` so the api-client's
 * mfa_required interceptor has somewhere to send the challenge. Adapted
 * from ppt-web's MfaWrapper — uses `useAdminAuth().token` for the bearer
 * instead of ppt-web's `AuthContext.getAccessToken`.
 *
 * Endpoint: POST /api/v1/admin/mfa/verify (api-server). This is the TOTP
 * *step-up* endpoint for already-enrolled platform principals — it inserts a
 * `two_factor_auth_verifications` row so the RECENT_MFA_WINDOW (15 min)
 * recency gate opens, and the api-client retries the original mutation
 * transparently.
 *
 * NOTE: do NOT point this at `/api/v1/auth/mfa/verify` — that endpoint is the
 * setup-completion flow (enables MFA, issues recovery codes, 409s if already
 * enrolled) and never opens the recency window. Pointing here was issue #764.
 */

import { MfaChallengeProvider } from '@ppt/admin-ui';
import { setMfaChallengeHandler } from '@ppt/api-client';
import type { ReactNode } from 'react';
import { useTranslation } from 'react-i18next';

import { useAdminAuth } from '../auth/AdminAuthContext';

export function MfaWrapper({ children }: { children: ReactNode }) {
  const { token } = useAdminAuth();
  const { t } = useTranslation();

  const mfaLabels = {
    title: t('admin.mfa.title'),
    description: t('admin.mfa.description'),
    descriptionForAction: t('admin.mfa.descriptionForAction', { action: '{action}' }),
    codeLabel: t('admin.mfa.label'),
    verify: t('admin.mfa.verify'),
    verifying: t('admin.mfa.verifying'),
    cancel: t('admin.mfa.cancel'),
    invalidFormat: t('admin.mfa.invalidFormat'),
    invalidCode: t('admin.mfa.invalidCode'),
    verificationFailed: t('admin.mfa.verificationFailed'),
  };

  return (
    <MfaChallengeProvider
      registerHandler={setMfaChallengeHandler}
      labels={mfaLabels}
      verify={async (code: string) => {
        const headers: Record<string, string> = { 'Content-Type': 'application/json' };
        if (token) headers.Authorization = `Bearer ${token}`;
        try {
          const response = await fetch('/api/v1/admin/mfa/verify', {
            method: 'POST',
            headers,
            body: JSON.stringify({ code }),
          });
          return response.ok;
        } catch {
          return false;
        }
      }}
    >
      {children}
    </MfaChallengeProvider>
  );
}
