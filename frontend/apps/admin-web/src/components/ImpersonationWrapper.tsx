/**
 * ImpersonationWrapper (admin-web port).
 *
 * Ported from ppt-web's `features/admin/ImpersonationWrapper.tsx`. Reads the
 * operator's bearer token from `useAdminAuth` instead of ppt-web's
 * `AuthContext.getAccessToken`. Behavior, storage key, and DELETE call
 * unchanged.
 */

import { ImpersonationBanner, type ImpersonationBannerLabels } from '@ppt/admin-ui';
import { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';

import { useAdminAuth } from '../auth/AdminAuthContext';

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
  const [state, setState] = useState<StoredImpersonation | null>(
    initialState !== undefined ? initialState : readStored(),
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
        await stopOverride(current.tokenId);
      } else {
        const headers: Record<string, string> = {};
        if (token) headers.Authorization = `Bearer ${token}`;
        await fetch(`/api/v1/admin/impersonation/${current.tokenId}`, {
          method: 'DELETE',
          headers,
        });
      }
    } catch {
      // Clear local state regardless — banner must never get stuck.
    } finally {
      clearStored();
      setState(null);
    }
  }, [state, token, stopOverride]);

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
