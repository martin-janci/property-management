/**
 * ImpersonationListPage — integration tests for the active-session viewer.
 *
 * Covers the page-level contracts that, if broken, would silently fail to
 * surface or stop active impersonation sessions (a security primitive):
 *   - active sessions render from /admin/impersonation/active
 *   - Stop button issues DELETE to the right endpoint
 *   - Stop button is HIDDEN when users_impersonate capability is absent
 *     (this is the page-level "denied" affordance)
 *   - empty state renders
 *   - error state renders
 *   - timestamp formatting renders the expires_at value
 *   - 404 from the active endpoint is treated as empty (backend may not be
 *     implemented yet — page contract)
 *
 * Skipped (with reason):
 *   - "Stop confirmation modal": the current page (260 LoC) issues the DELETE
 *     directly on click — no destructive-confirm dialog wraps the action.
 *     Adding a test for non-existent behavior would just lock in the gap.
 *   - "ForbiddenPage": as with audit.tsx, this page renders no <ForbiddenPage>
 *     itself — capability gating happens upstream in <ProtectedRoute>. The
 *     page-internal gate hides the Stop column when the cap is missing; that
 *     behavior IS tested here.
 *   - "Relative time string" ("expires in X minutes"): the page uses
 *     `Intl.DateTimeFormat({ dateStyle: 'short', timeStyle: 'short' })` —
 *     it renders an absolute date, not a relative string. We assert the
 *     date-render contract instead.
 */

import { CapabilityProvider } from '@ppt/admin-ui';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import type { ReactNode } from 'react';
import { MemoryRouter } from 'react-router-dom';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { AdminAuthProvider } from '../auth/AdminAuthContext';
import { sessionTokenStore } from '../auth/tokenStore';
import { ToastProvider } from '../components/Toast';
import ImpersonationListPage from './ImpersonationListPage';

// ---------------------------------------------------------------------------
// i18next mock — page uses defaultValue for every key
// ---------------------------------------------------------------------------
vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, opts?: { defaultValue?: string }) => opts?.defaultValue ?? key,
    i18n: { changeLanguage: vi.fn() },
  }),
  initReactI18next: { type: '3rdParty', init: vi.fn() },
}));

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function makeQueryClient() {
  return new QueryClient({
    defaultOptions: { queries: { retry: false, retryDelay: 0 } },
  });
}

interface RenderOptions {
  capabilities?: string[];
}

function Providers({
  capabilities = ['users_impersonate'],
  children,
}: RenderOptions & { children: ReactNode }) {
  sessionStorage.clear();
  sessionTokenStore.set('test-token');
  const qc = makeQueryClient();
  return (
    <QueryClientProvider client={qc}>
      <MemoryRouter>
        <AdminAuthProvider>
          <CapabilityProvider
            value={{ isPlatformPrincipal: true, capabilities: capabilities as never[] }}
          >
            <ToastProvider>{children}</ToastProvider>
          </CapabilityProvider>
        </AdminAuthProvider>
      </MemoryRouter>
    </QueryClientProvider>
  );
}

function renderPage(opts: RenderOptions = {}) {
  return render(
    <Providers {...opts}>
      <ImpersonationListPage />
    </Providers>
  );
}

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

const SESSION_A = {
  token_id: 'tok-aaaa',
  target_user_id: 'user-1',
  target_email: 'target-a@example.com',
  started_by: 'admin-a@example.com',
  expires_at: '2026-05-17T11:00:00Z',
};

const SESSION_B = {
  token_id: 'tok-bbbb',
  target_user_id: 'user-2',
  target_email: 'target-b@example.com',
  started_by: 'admin-b@example.com',
  expires_at: '2026-05-17T12:30:00Z',
};

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe('ImpersonationListPage', () => {
  beforeEach(() => {
    vi.spyOn(globalThis, 'fetch');
    vi.spyOn(console, 'warn').mockImplementation(() => {});
  });

  afterEach(() => {
    sessionStorage.clear();
    vi.restoreAllMocks();
  });

  // ------------------------------------------------------------------
  // 1. Renders active sessions
  // ------------------------------------------------------------------
  it('renders rows for each active session from the API', async () => {
    vi.mocked(fetch).mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({ items: [SESSION_A, SESSION_B] }),
    } as Response);

    renderPage();

    await waitFor(() => expect(screen.getByText('target-a@example.com')).toBeDefined());
    expect(screen.getByText('target-b@example.com')).toBeDefined();
    expect(screen.getByText('admin-a@example.com')).toBeDefined();
    expect(screen.getByText('admin-b@example.com')).toBeDefined();
  });

  // ------------------------------------------------------------------
  // 2. Empty state
  // ------------------------------------------------------------------
  it('renders an empty state when no sessions are active', async () => {
    vi.mocked(fetch).mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({ items: [] }),
    } as Response);

    renderPage();

    await waitFor(() =>
      expect(screen.getByText(/No active impersonation sessions/i)).toBeDefined()
    );
  });

  // ------------------------------------------------------------------
  // 3. 404 from active endpoint → error alert. The endpoint now exists in
  //    api-server, so a 404 is a genuine failure (the old "not implemented
  //    yet → empty list" fallback was removed).
  // ------------------------------------------------------------------
  it('renders an error alert when the active endpoint returns 404', async () => {
    vi.mocked(fetch).mockResolvedValue({ ok: false, status: 404 } as Response);

    renderPage();

    await waitFor(() => expect(screen.getByRole('alert')).toBeDefined());
    expect(screen.getByText(/404/)).toBeDefined();
  });

  // ------------------------------------------------------------------
  // 4. Error state (non-404 failure)
  // ------------------------------------------------------------------
  it('renders an error alert when the active endpoint returns 500', async () => {
    vi.mocked(fetch).mockResolvedValue({ ok: false, status: 500 } as Response);

    renderPage();

    await waitFor(() => expect(screen.getByRole('alert')).toBeDefined());
    expect(screen.getByText(/500/)).toBeDefined();
  });

  // ------------------------------------------------------------------
  // 5. Stop click → DELETE /admin/impersonation/{token_id}
  // ------------------------------------------------------------------
  it('issues DELETE /admin/impersonation/{token_id} when Stop is clicked', async () => {
    const user = userEvent.setup();
    vi.mocked(fetch).mockImplementation(async (input, init) => {
      const url = input.toString();
      const method = (init?.method ?? 'GET').toUpperCase();
      if (url.includes('/admin/impersonation/active') && method === 'GET') {
        return {
          ok: true,
          status: 200,
          json: async () => ({ items: [SESSION_A] }),
        } as Response;
      }
      if (method === 'DELETE' && url.includes(`/admin/impersonation/${SESSION_A.token_id}`)) {
        return { ok: true, status: 204 } as Response;
      }
      return { ok: false, status: 404 } as Response;
    });

    renderPage();

    const stopBtn = await screen.findByRole('button', { name: /Stop/i });
    await user.click(stopBtn);

    await waitFor(() => {
      const deleteCall = vi
        .mocked(fetch)
        .mock.calls.find(
          ([input, init]) =>
            (init as RequestInit)?.method === 'DELETE' &&
            input.toString().includes(`/admin/impersonation/${SESSION_A.token_id}`)
        );
      expect(deleteCall).toBeDefined();
    });
  });

  // ------------------------------------------------------------------
  // 6. Stop column / button is HIDDEN when capability is absent
  // ------------------------------------------------------------------
  it('hides the Stop button when users_impersonate capability is absent', async () => {
    vi.mocked(fetch).mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({ items: [SESSION_A] }),
    } as Response);

    renderPage({ capabilities: [] });

    // Row still renders…
    await waitFor(() => expect(screen.getByText('target-a@example.com')).toBeDefined());

    // …but the Stop button does not.
    expect(screen.queryByRole('button', { name: /Stop/i })).toBeNull();
  });

  // ------------------------------------------------------------------
  // 7. expires_at renders as a formatted date string
  // ------------------------------------------------------------------
  it('formats expires_at as a localized date+time string', async () => {
    vi.mocked(fetch).mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({ items: [SESSION_A] }),
    } as Response);

    renderPage();

    await waitFor(() => expect(screen.getByText('target-a@example.com')).toBeDefined());

    // Intl.DateTimeFormat('en-GB', { dateStyle: 'short', timeStyle: 'short' })
    // for 2026-05-17T11:00:00Z renders the date portion "17/05/2026" — assert
    // on a stable fragment that's not timezone-sensitive (the date in UTC).
    const formatted = new Intl.DateTimeFormat('en-GB', {
      dateStyle: 'short',
      timeStyle: 'short',
    }).format(new Date(SESSION_A.expires_at));
    expect(screen.getByText(formatted)).toBeDefined();
  });
});
