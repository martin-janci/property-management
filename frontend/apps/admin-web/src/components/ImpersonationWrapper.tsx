/**
 * ImpersonationWrapper (admin-web port).
 *
 * Ported from ppt-web's `features/admin/ImpersonationWrapper.tsx`. Reads the
 * operator's bearer token from `useAdminAuth` instead of ppt-web's
 * `AuthContext.getAccessToken`. Storage key and DELETE call unchanged.
 *
 * Stop-failure handling: ending impersonation is a security primitive, so the
 * UI must never report success it did not get. If the DELETE returns a non-ok
 * status (or `stopOverride` resolves `false`), the server-side session is still
 * live — we surface an error toast (matching the sibling `ImpersonationListPage`
 * convention) and KEEP the banner + stored state so the operator knows they are
 * still impersonating and can retry. Local state is cleared only once the
 * server confirms the session is gone.
 */

import { ImpersonationBanner, type ImpersonationBannerLabels } from '@ppt/admin-ui';
import { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';

import { useAdminAuth } from '../auth/AdminAuthContext';
import { useToast } from './Toast';

const STORAGE_KEY = 'ppt_impersonation';

export interface StoredImpersonation {
  tokenId: string;
  expiresAt: string;
  targetUserLabel?: string;
}

function readStored(): StoredImpersonation | null {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as StoredImpersonation;
    if (!parsed?.tokenId || !parsed?.expiresAt) return null;
    if (new Date(parsed.expiresAt).getTime() <= Date.now()) {
      localStorage.removeItem(STORAGE_KEY);
      return null;
    }
    return parsed;
  } catch {
    return null;
  }
}

function clearStored(): void {
  try {
    localStorage.removeItem(STORAGE_KEY);
  } catch {
    // storage unavailable — caller will still re-render via state
  }
}

export interface ImpersonationWrapperProps {
  initialState?: StoredImpersonation | null;
  stopOverride?: (tokenId: string) => Promise<boolean>;
}

export function ImpersonationWrapper({
  initialState,
  stopOverride,
}: ImpersonationWrapperProps = {}) {
  const { t } = useTranslation();
  const { token } = useAdminAuth();
  const { showToast } = useToast();
  const [state, setState] = useState<StoredImpersonation | null>(
    initialState !== undefined ? initialState : readStored()
  );

  useEffect(() => {
    if (initialState !== undefined) return undefined;
    const onStorage = (e: StorageEvent) => {
      if (e.key !== STORAGE_KEY) return;
      setState(readStored());
    };
    window.addEventListener('storage', onStorage);
    return () => window.removeEventListener('storage', onStorage);
  }, [initialState]);

  const handleEnd = useCallback(async () => {
    const current = state;
    if (!current) return;
    try {
      if (stopOverride) {
        const ok = await stopOverride(current.tokenId);
        if (!ok) throw new Error('Stop impersonation failed');
      } else {
        const headers: Record<string, string> = {};
        if (token) headers.Authorization = `Bearer ${token}`;
        const res = await fetch(`/api/v1/admin/impersonation/${current.tokenId}`, {
          method: 'DELETE',
          headers,
        });
        // `fetch` does not reject on 4xx/5xx — check the status explicitly so a
        // failed stop is never mistaken for success.
        if (!res.ok) {
          throw new Error(`Stop impersonation failed: ${res.status}`);
        }
      }
    } catch (err) {
      // The session is still live server-side. Surface the failure and KEEP the
      // banner + stored state — never silently clear a session the operator
      // believes has ended. They can retry via the banner's End button.
      showToast({
        type: 'error',
        title: t('admin.impersonation.toast.stopFailedTitle', { defaultValue: 'Stop failed' }),
        message: err instanceof Error ? err.message : 'Unknown error',
      });
      return;
    }
    // Server confirmed the session is gone — now it is safe to clear.
    clearStored();
    setState(null);
  }, [state, token, stopOverride, showToast, t]);

  const labels: ImpersonationBannerLabels = {
    label: t('admin.impersonation.label'),
    asUser: t('admin.impersonation.asUser', { name: '{name}' }),
    expires: t('admin.impersonation.expires', { time: '{time}' }),
    stop: t('admin.impersonation.stop'),
  };

  return (
    <ImpersonationBanner
      active={state !== null}
      targetUserLabel={state?.targetUserLabel}
      expiresAt={state?.expiresAt}
      onEnd={handleEnd}
      labels={labels}
    />
  );
}

ImpersonationWrapper.displayName = 'ImpersonationWrapper';
