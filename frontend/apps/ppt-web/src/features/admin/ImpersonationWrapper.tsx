/**
 * ImpersonationWrapper (Phase 5 — E2 wire-in).
 *
 * Mounts `<ImpersonationBanner>` at the top of the app shell so that
 * whenever the current session is impersonating a tenant user, a sticky
 * red bar is visible — leak #21 calls out impersonation as something
 * that must be impossible to *hide*.
 *
 * State source
 * -----------
 * `POST /api/v1/admin/impersonation/start` returns the issued token in
 * its JSON body (no Set-Cookie yet — the backend deliberately leaves
 * the storage decision to the client). We persist the issued token in
 * `localStorage` under `ppt_impersonation` so it survives reloads
 * within the 15-minute TTL, and we re-read it on mount so the banner
 * shows up the moment the start mutation resolves and the operator
 * navigates away from the admin tree.
 *
 * The same record carries `targetUserLabel` (display name / email of
 * the impersonatee) which the start handler is expected to capture
 * from the user-list row that triggered the impersonation. When the
 * label is unknown we still show the banner — the prefix alone is
 * enough to tell the operator they are not in their own session.
 *
 * Stop flow
 * ---------
 * The "End impersonation" button calls
 * `DELETE /api/v1/admin/impersonation/{tokenId}` with the operator's
 * bearer token (the bearer is the *operator's* JWT, not the
 * impersonation token — see admin/impersonation.rs). On success or
 * failure we clear the local record so the banner disappears either
 * way; the worst case on a network failure is an orphaned DB row that
 * the 15-minute TTL eventually retires.
 */

import { ImpersonationBanner, type ImpersonationBannerLabels } from '@ppt/admin-ui';
import { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useAuth } from '../../contexts';

const STORAGE_KEY = 'ppt_impersonation';

/** Shape persisted in localStorage. Kept narrow on purpose. */
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
    // Drop expired records on read so the banner doesn't lie.
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
  /**
   * Override the resolved state — purely for tests so they don't have
   * to touch `localStorage`. Production callers omit this and let the
   * wrapper read from storage.
   */
  initialState?: StoredImpersonation | null;
  /**
   * Override the stop-call so tests can assert behavior without a real
   * fetch. Receives the tokenId and returns a Promise<boolean> for OK.
   */
  stopOverride?: (tokenId: string) => Promise<boolean>;
}

export function ImpersonationWrapper({
  initialState,
  stopOverride,
}: ImpersonationWrapperProps = {}) {
  const { t } = useTranslation();
  const { getAccessToken } = useAuth();
  const [state, setState] = useState<StoredImpersonation | null>(
    initialState !== undefined ? initialState : readStored(),
  );

  // Re-sync when storage changes from another tab — covers the case
  // where the operator starts impersonation in one tab and has an
  // older tab open that should also flip into the impersonation view.
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
        const token = getAccessToken();
        const headers: Record<string, string> = {};
        if (token) headers.Authorization = `Bearer ${token}`;
        await fetch(`/api/v1/admin/impersonation/${current.tokenId}`, {
          method: 'DELETE',
          headers,
        });
      }
    } catch {
      // We still clear local state — the banner should never get
      // stuck on a transient network error.
    } finally {
      clearStored();
      setState(null);
    }
  }, [state, getAccessToken, stopOverride]);

  // Resolve labels once per render. `useTranslation` returns stable
  // refs across renders within a locale; the spread is cheap.
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
