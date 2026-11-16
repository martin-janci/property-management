/**
 * ImpersonationWrapper — stop-flow tests.
 *
 * Ending impersonation is a security primitive: the UI must never report (by
 * silently dropping the banner) that a session ended when the server-side stop
 * actually failed. These tests pin the honest-state contract:
 *   - A non-ok DELETE response surfaces an error toast and KEEPS the banner.
 *   - A `stopOverride` that resolves `false` is treated as a failure, not a
 *     success.
 *   - A successful stop clears the banner.
 *
 * The non-ok / `false` cases are the regression guard for the original bug,
 * where the catch/finally swallowed failures and cleared state unconditionally
 * (fetch never rejects on 4xx/5xx), leaving the operator believing they had
 * stopped impersonating while the token was still live.
 */

import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import type { ReactNode } from 'react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { AdminAuthProvider } from '../auth/AdminAuthContext';
import { sessionTokenStore } from '../auth/tokenStore';
import { ImpersonationWrapper, type StoredImpersonation } from './ImpersonationWrapper';
import { ToastProvider } from './Toast';

// i18next mock — component uses defaultValue for the toast key; other keys fall
// back to the raw key string, which is fine for these assertions.
vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, opts?: { defaultValue?: string }) => opts?.defaultValue ?? key,
    i18n: { changeLanguage: vi.fn() },
  }),
  initReactI18next: { type: '3rdParty', init: vi.fn() },
}));

const ACTIVE: StoredImpersonation = {
  tokenId: 'tok-aaaa',
  expiresAt: '2999-01-01T00:00:00Z',
  targetUserLabel: 'target@example.com',
};

function Providers({ children }: { children: ReactNode }) {
  sessionStorage.clear();
  sessionTokenStore.set('operator-token');
  return (
    <AdminAuthProvider>
      <ToastProvider>{children}</ToastProvider>
    </AdminAuthProvider>
  );
}

function renderWrapper(props: Parameters<typeof ImpersonationWrapper>[0] = {}) {
  return render(
    <Providers>
      <ImpersonationWrapper {...props} />
    </Providers>
  );
}

/** The banner's "End impersonation" button (label text varies with i18n). */
function endButton() {
  return within(screen.getByTestId('impersonation-banner')).getByRole('button');
}

describe('ImpersonationWrapper — stop flow', () => {
  beforeEach(() => {
    vi.spyOn(globalThis, 'fetch');
  });

  afterEach(() => {
    sessionStorage.clear();
    vi.restoreAllMocks();
  });

  it('surfaces an error and KEEPS the banner when the DELETE returns a non-ok status', async () => {
    const user = userEvent.setup();
    // fetch resolves (does NOT reject) with a 500 — the original bug path.
    vi.mocked(fetch).mockResolvedValue({ ok: false, status: 500 } as Response);

    renderWrapper({ initialState: ACTIVE });

    const banner = screen.getByTestId('impersonation-banner');
    expect(banner).toBeDefined();

    await user.click(endButton());

    // Error toast surfaces the failure…
    await waitFor(() => expect(screen.getByText('Stop failed')).toBeDefined());

    // …and the banner is still present: the session is still live server-side,
    // so the state must stay honest (not silently cleared).
    expect(screen.getByTestId('impersonation-banner')).toBeDefined();
  });

  it('treats a stopOverride that resolves false as a failure (keeps banner + toast)', async () => {
    const user = userEvent.setup();
    const stopOverride = vi.fn().mockResolvedValue(false);

    renderWrapper({ initialState: ACTIVE, stopOverride });

    await user.click(endButton());

    await waitFor(() => expect(screen.getByText('Stop failed')).toBeDefined());
    expect(stopOverride).toHaveBeenCalledWith(ACTIVE.tokenId);
    expect(screen.getByTestId('impersonation-banner')).toBeDefined();
  });

  it('clears the banner on a successful stop', async () => {
    const user = userEvent.setup();
    vi.mocked(fetch).mockResolvedValue({ ok: true, status: 204 } as Response);

    renderWrapper({ initialState: ACTIVE });

    await user.click(endButton());

    await waitFor(() => expect(screen.queryByTestId('impersonation-banner')).toBeNull());
  });
});
