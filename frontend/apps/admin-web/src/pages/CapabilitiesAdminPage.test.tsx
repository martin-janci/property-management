/**
 * CapabilitiesAdminPage — grant/revoke + 404 fallback tests.
 */

import { CAPABILITIES, CapabilityProvider } from '@ppt/admin-ui';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { AdminAuthProvider } from '../auth/AdminAuthContext';
import { sessionTokenStore } from '../auth/tokenStore';
import CapabilitiesAdminPage from './CapabilitiesAdminPage';

// ---------------------------------------------------------------------------
// Mock @ppt/admin-ui's useMfaChallenge so we can control MFA outcome
// ---------------------------------------------------------------------------

const mockPromptMfa: ReturnType<typeof vi.fn> & {
  mockResolvedValue: (val: boolean) => void;
} = vi.fn();

vi.mock('@ppt/admin-ui', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@ppt/admin-ui')>();
  return {
    ...actual,
    useMfaChallenge: () => ({ prompt: mockPromptMfa }),
  };
});

// ---------------------------------------------------------------------------
// i18next mock (Toast uses useTranslation, not imported here but just in case)
// ---------------------------------------------------------------------------
vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => key,
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
  initialEntries?: string[];
}

function renderPage({
  capabilities = ['memberships_grant'],
  initialEntries = ['/identity/capabilities'],
}: RenderOptions = {}) {
  sessionStorage.clear();
  sessionTokenStore.set('test-token');

  const qc = makeQueryClient();

  return render(
    <QueryClientProvider client={qc}>
      <MemoryRouter initialEntries={initialEntries}>
        <AdminAuthProvider>
          <CapabilityProvider
            value={{ isPlatformPrincipal: true, capabilities: capabilities as never[] }}
          >
            <Routes>
              <Route path="/identity/capabilities" element={<CapabilitiesAdminPage />} />
            </Routes>
          </CapabilityProvider>
        </AdminAuthProvider>
      </MemoryRouter>
    </QueryClientProvider>
  );
}

// ---------------------------------------------------------------------------
// Sample capability grant
// ---------------------------------------------------------------------------

const SAMPLE_GRANT = {
  id: 'grant-1',
  capability: 'agencies_read',
  granted_at: '2026-01-01T00:00:00Z',
  granted_by: 'root@example.com',
  expires_at: null,
  revoked_at: null,
  status: 'active' as const,
};

// Backend `GET /admin/capabilities/users/{id}` returns a raw `Vec<CapabilityGrant>`.
// The page normalizes this into its internal view via `normalizeUserCapabilities`.
const SAMPLE_USER_RESPONSE = [SAMPLE_GRANT];

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe('CapabilitiesAdminPage', () => {
  beforeEach(() => {
    vi.spyOn(globalThis, 'fetch');
    mockPromptMfa.mockResolvedValue(true);
  });

  afterEach(() => {
    sessionStorage.clear();
    vi.restoreAllMocks();
  });

  // ------------------------------------------------------------------
  // 3. Registry view (default) → renders a card for every capability
  // ------------------------------------------------------------------

  it('renders a registry card for every capability', async () => {
    // The registry mirrors the `admin_core::Capability` enum. Derive the
    // expected cards from CAPABILITIES itself so the test can't drift when a
    // capability is added on the backend (e.g. `oauth_client_write`).
    expect(CAPABILITIES.length).toBeGreaterThan(0);
    expect(new Set(CAPABILITIES).size).toBe(CAPABILITIES.length);

    // The registry view fetches GET /admin/capabilities/registry — return one
    // entry per capability so the card grid renders.
    vi.mocked(fetch).mockResolvedValueOnce({
      ok: true,
      status: 200,
      json: async () =>
        CAPABILITIES.map((capability) => ({
          capability,
          description: `desc for ${capability}`,
          risk_level: 'low' as const,
          holder_count: 0,
        })),
    } as Response);

    renderPage({ initialEntries: ['/identity/capabilities'] });

    await waitFor(() => expect(screen.getByText('Capabilities registry')).toBeDefined());

    // Every capability in the registry enum must render a card.
    await waitFor(() => expect(screen.getByText('agencies_read')).toBeDefined());
    for (const capability of CAPABILITIES) {
      expect(screen.getByText(capability)).toBeDefined();
    }
  });

  // ------------------------------------------------------------------
  // 1. Per-user view with userId → fetches /capabilities/users/{id}
  // ------------------------------------------------------------------

  it('fetches /capabilities/users/{id} when view=user&id=user-abc', async () => {
    vi.mocked(fetch).mockResolvedValueOnce({
      ok: true,
      status: 200,
      json: async () => SAMPLE_USER_RESPONSE,
    } as Response);

    renderPage({ initialEntries: ['/identity/capabilities?view=user&id=user-abc'] });

    await waitFor(() =>
      expect(fetch).toHaveBeenCalledWith(
        '/api/v1/admin/capabilities/users/user-abc',
        expect.objectContaining({
          headers: expect.objectContaining({ Authorization: 'Bearer test-token' }),
        })
      )
    );
  });

  // ------------------------------------------------------------------
  // 2. 404 from /users/{id} → silently falls back to /capabilities/me
  // ------------------------------------------------------------------

  it('falls back to /capabilities/me on 404 from per-user endpoint', async () => {
    // Backend `/me` returns `MyCapabilitiesResponse { user_id, principal_kind, capabilities }`.
    const ME_RESPONSE = {
      user_id: 'me',
      principal_kind: 'platform',
      capabilities: [],
    };

    // First fetch: /users/{id} → 404; second fetch: /me fallback → 200
    vi.mocked(fetch)
      .mockResolvedValueOnce({ ok: false, status: 404 } as Response)
      .mockResolvedValueOnce({
        ok: true,
        status: 200,
        json: async () => ME_RESPONSE,
      } as Response);

    renderPage({ initialEntries: ['/identity/capabilities?view=user&id=nonexistent-user'] });

    // The component should call /me as a fallback
    await waitFor(
      () => expect(fetch).toHaveBeenCalledWith('/api/v1/admin/capabilities/me', expect.any(Object)),
      { timeout: 3000 }
    );

    // After fallback, no grants are present → empty state row is rendered.
    await waitFor(() => expect(screen.getByText(/No capability grants found/i)).toBeDefined(), {
      timeout: 3000,
    });
  });

  // ------------------------------------------------------------------
  // 4. Grant button opens drawer; submitting without reason for tenant_purge blocks submit
  // ------------------------------------------------------------------

  it('opens grant drawer on "+ Grant capability" click', async () => {
    const user = userEvent.setup();

    vi.mocked(fetch).mockResolvedValueOnce({
      ok: true,
      status: 200,
      json: async () => SAMPLE_USER_RESPONSE,
    } as Response);

    renderPage({ initialEntries: ['/identity/capabilities?view=user&id=user-abc'] });

    await waitFor(() => screen.getByRole('button', { name: /\+ Grant capability/i }));
    await user.click(screen.getByRole('button', { name: /\+ Grant capability/i }));

    expect(screen.getByRole('dialog', { name: /Grant capability/i })).toBeDefined();
  });

  it('submit button is disabled when tenant_purge selected but no audit reason provided', async () => {
    const user = userEvent.setup();

    vi.mocked(fetch).mockResolvedValueOnce({
      ok: true,
      status: 200,
      json: async () => SAMPLE_USER_RESPONSE,
    } as Response);

    renderPage({ initialEntries: ['/identity/capabilities?view=user&id=user-abc'] });

    // Open the grant drawer using the "+" prefixed button in the page header
    await waitFor(() => screen.getByRole('button', { name: /\+ Grant capability/i }));
    await user.click(screen.getByRole('button', { name: /\+ Grant capability/i }));

    // Drawer is open — wait for the combobox
    await waitFor(() => screen.getByRole('combobox', { name: /Capability/i }));

    // Select tenant_purge
    const select = screen.getByRole('combobox', { name: /Capability/i });
    await user.selectOptions(select, 'tenant_purge');

    // Submit button is inside the drawer footer — it should be disabled (no reason yet)
    // The drawer has the submit button with text "Grant capability" (no + prefix)
    const drawer = screen.getByRole('dialog', { name: /Grant capability/i });
    const submitBtn = drawer.querySelector('.cap-btn--primary') as HTMLButtonElement;
    expect(submitBtn).toBeDefined();
    expect(submitBtn.disabled).toBe(true);
  });

  // ------------------------------------------------------------------
  // 5. Revoke button calls DELETE with MFA challenge
  // ------------------------------------------------------------------

  it('calls DELETE endpoint after successful MFA challenge on revoke', async () => {
    const user = userEvent.setup();
    mockPromptMfa.mockResolvedValue(true);

    vi.mocked(fetch)
      .mockResolvedValueOnce({
        ok: true,
        status: 200,
        json: async () => SAMPLE_USER_RESPONSE,
      } as Response)
      // Revoke DELETE call
      .mockResolvedValueOnce({ ok: true, status: 204 } as Response)
      // Refetch after invalidation
      .mockResolvedValueOnce({
        ok: true,
        status: 200,
        json: async () => ({ ...SAMPLE_USER_RESPONSE, grants: [] }),
      } as Response);

    // memberships_revoke capability is required for the Revoke button to be visible
    renderPage({
      initialEntries: ['/identity/capabilities?view=user&id=user-abc'],
      capabilities: ['memberships_grant', 'memberships_revoke'],
    });

    // Wait for the table to render — the Revoke button text is "Revoke"
    await waitFor(() => screen.getByText('Revoke'), { timeout: 3000 });
    await user.click(screen.getByText('Revoke'));

    await waitFor(() =>
      expect(fetch).toHaveBeenCalledWith(
        `/api/v1/admin/capabilities/users/user-abc/grant/${SAMPLE_GRANT.id}`,
        expect.objectContaining({ method: 'DELETE' })
      )
    );
  });

  // ------------------------------------------------------------------
  // 6. Revoke MFA cancel → no DELETE call sent
  // ------------------------------------------------------------------

  it('does not call DELETE when MFA challenge is cancelled', async () => {
    const user = userEvent.setup();
    mockPromptMfa.mockResolvedValue(false); // User cancels MFA

    vi.mocked(fetch).mockResolvedValueOnce({
      ok: true,
      status: 200,
      json: async () => SAMPLE_USER_RESPONSE,
    } as Response);

    // memberships_revoke capability is required for the Revoke button to be visible
    renderPage({
      initialEntries: ['/identity/capabilities?view=user&id=user-abc'],
      capabilities: ['memberships_grant', 'memberships_revoke'],
    });

    await waitFor(() => screen.getByText('Revoke'), { timeout: 3000 });
    await user.click(screen.getByText('Revoke'));

    // Wait a tick for async resolution
    await new Promise((r) => setTimeout(r, 100));

    // Only the initial GET should have been called, not DELETE
    const deleteCalls = vi
      .mocked(fetch)
      .mock.calls.filter(([, init]) => init && (init as RequestInit).method === 'DELETE');
    expect(deleteCalls).toHaveLength(0);
  });
});
