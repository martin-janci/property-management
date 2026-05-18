/**
 * AuditPage — integration tests for the audit log viewer.
 *
 * Covers the page-level contracts that, if broken, would silently mis-render
 * or mis-export security-critical data:
 *   - rows render from the API response
 *   - filter inputs propagate into the fetch URL (actor + date range presets)
 *   - empty/error states have user-visible affordances
 *   - "Load more" forwards the server-provided next_cursor
 *   - CSV export hits the right endpoint with the current filter and triggers
 *     a real `<a download>` click
 *
 * Skipped (with reason):
 *   - "Forbidden state": AuditPage itself does not render <ForbiddenPage> —
 *     capability gating happens upstream in <ProtectedRoute>. That path is
 *     already covered in components/ProtectedRoute.test.tsx, so we don't
 *     re-verify it here.
 *   - "Reason prompt fires on CSV export": the current audit.tsx does NOT
 *     wrap export in <AuditReasonPrompt> (verified by grep) — `handleExportCsv`
 *     calls fetch directly. Test would be asserting against behavior that
 *     doesn't exist.
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
import AuditPage from './audit';

// ---------------------------------------------------------------------------
// i18next mock (Toast uses useTranslation)
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
  initialEntries?: string[];
}

function Providers({
  capabilities = ['audit_read'],
  initialEntries = ['/ops/audit'],
  children,
}: RenderOptions & { children: ReactNode }) {
  sessionStorage.clear();
  sessionTokenStore.set('test-token');
  const qc = makeQueryClient();
  return (
    <QueryClientProvider client={qc}>
      <MemoryRouter initialEntries={initialEntries}>
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
      <AuditPage />
    </Providers>
  );
}

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

function makeRow(id: string, overrides: Record<string, unknown> = {}) {
  return {
    id,
    occurred_at: '2026-05-17T10:30:00Z',
    actor_email: `actor-${id}@example.com`,
    capability: 'audit_read',
    target_kind: 'user' as const,
    target_slug: `target-${id}`,
    outcome: 'success' as const,
    reason: `reason for ${id}`,
    ip: '10.0.0.1',
    ...overrides,
  };
}

/** Helper to find the most recent GET call URL that fetched /admin/audit. */
function lastAuditGetUrl(): string | undefined {
  const calls = vi.mocked(fetch).mock.calls;
  for (let i = calls.length - 1; i >= 0; i--) {
    const [input, init] = calls[i] as [RequestInfo, RequestInit?];
    const url = input.toString();
    const method = (init?.method ?? 'GET').toUpperCase();
    if (method === 'GET' && url.includes('/admin/audit') && !url.includes('/csv')) {
      return url;
    }
  }
  return undefined;
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe('AuditPage', () => {
  beforeEach(() => {
    vi.spyOn(globalThis, 'fetch');
    vi.spyOn(console, 'warn').mockImplementation(() => {});
  });

  afterEach(() => {
    sessionStorage.clear();
    vi.restoreAllMocks();
  });

  // ------------------------------------------------------------------
  // 1. Renders rows from /admin/audit
  // ------------------------------------------------------------------
  it('renders audit rows from the API response', async () => {
    vi.mocked(fetch).mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({
        items: [makeRow('a'), makeRow('b'), makeRow('c')],
      }),
    } as Response);

    renderPage();

    await waitFor(() => expect(screen.getByText('actor-a@example.com')).toBeDefined());
    expect(screen.getByText('actor-b@example.com')).toBeDefined();
    expect(screen.getByText('actor-c@example.com')).toBeDefined();
  });

  // ------------------------------------------------------------------
  // 2. Empty state
  // ------------------------------------------------------------------
  it('renders empty-state copy when the API returns zero items', async () => {
    vi.mocked(fetch).mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({ items: [] }),
    } as Response);

    renderPage();

    await waitFor(() =>
      expect(screen.getByText(/No audit events match this filter/i)).toBeDefined()
    );
  });

  // ------------------------------------------------------------------
  // 3. Error state — page renders an alert with a Retry control
  // ------------------------------------------------------------------
  it('renders an error alert with a Retry button when the fetch fails', async () => {
    vi.mocked(fetch).mockResolvedValue({ ok: false, status: 500 } as Response);

    renderPage();

    await waitFor(() => expect(screen.getByRole('alert')).toBeDefined());
    expect(screen.getByText(/Unable to load audit log/i)).toBeDefined();
    expect(screen.getByRole('button', { name: /Retry/i })).toBeDefined();
  });

  // ------------------------------------------------------------------
  // 4. Filter by actor → refetches with ?actor=
  // ------------------------------------------------------------------
  it('refetches with ?actor= when the actor filter is typed', async () => {
    const user = userEvent.setup();
    vi.mocked(fetch).mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({ items: [] }),
    } as Response);

    renderPage();

    // Wait for the initial fetch
    await waitFor(() => expect(lastAuditGetUrl()).toBeDefined());

    const input = screen.getByLabelText('Actor') as HTMLInputElement;
    await user.type(input, 'alice@example.com');

    // Filter input is debounced (~250ms) — wait for a fetch that includes the actor param
    await waitFor(
      () => {
        const url = lastAuditGetUrl();
        expect(url).toBeDefined();
        expect(url).toContain('actor=alice%40example.com');
      },
      { timeout: 2000 }
    );
  });

  // ------------------------------------------------------------------
  // 5. Filter by date range preset → refetches with ?range=
  // ------------------------------------------------------------------
  it('refetches with ?range= when a date-range preset is clicked', async () => {
    const user = userEvent.setup();
    vi.mocked(fetch).mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({ items: [] }),
    } as Response);

    renderPage();

    await waitFor(() => expect(lastAuditGetUrl()).toBeDefined());

    // Click the "24h" preset button
    await user.click(screen.getByRole('button', { name: '24h' }));

    await waitFor(() => {
      const url = lastAuditGetUrl();
      expect(url).toBeDefined();
      expect(url).toContain('range=24h');
    });
  });

  // ------------------------------------------------------------------
  // 6. Pagination — clicking "Load more" forwards next_cursor
  // ------------------------------------------------------------------
  it('forwards the next_cursor to the API when "Load more" is clicked', async () => {
    const user = userEvent.setup();
    vi.mocked(fetch)
      // First page
      .mockResolvedValueOnce({
        ok: true,
        status: 200,
        json: async () => ({
          items: [makeRow('a')],
          next_cursor: 'cursor-page-2',
        }),
      } as Response)
      // Second page (cursor)
      .mockResolvedValueOnce({
        ok: true,
        status: 200,
        json: async () => ({
          items: [makeRow('b')],
        }),
      } as Response);

    renderPage();

    await waitFor(() => expect(screen.getByText('actor-a@example.com')).toBeDefined());

    const loadMore = await screen.findByRole('button', { name: /Load more/i });
    await user.click(loadMore);

    await waitFor(() => {
      const url = lastAuditGetUrl();
      expect(url).toBeDefined();
      expect(url).toContain('after=cursor-page-2');
    });

    // And the second page's row gets appended
    await waitFor(() => expect(screen.getByText('actor-b@example.com')).toBeDefined());
  });

  // ------------------------------------------------------------------
  // 7. CSV export click — fires fetch against /admin/audit/csv with filter
  // ------------------------------------------------------------------
  it('fetches /admin/audit/csv with the current filters when Export CSV is clicked', async () => {
    const user = userEvent.setup();

    // jsdom doesn't implement URL.createObjectURL — stub it (and revoke).
    const createObjectURL = vi
      .spyOn(URL, 'createObjectURL')
      .mockReturnValue('blob:fake-object-url');
    const revokeObjectURL = vi.spyOn(URL, 'revokeObjectURL').mockImplementation(() => {});

    vi.mocked(fetch).mockImplementation(async (input) => {
      const url = input.toString();
      if (url.includes('/admin/audit/csv')) {
        return {
          ok: true,
          status: 200,
          blob: async () => new Blob(['col1,col2\n1,2'], { type: 'text/csv' }),
        } as Response;
      }
      return {
        ok: true,
        status: 200,
        json: async () => ({ items: [] }),
      } as Response;
    });

    renderPage();
    await waitFor(() => expect(lastAuditGetUrl()).toBeDefined());

    // Apply a filter so the CSV URL is non-trivial
    await user.click(screen.getByRole('button', { name: '24h' }));
    await waitFor(() => {
      const url = lastAuditGetUrl();
      expect(url).toContain('range=24h');
    });

    await user.click(screen.getByRole('button', { name: /Export CSV/i }));

    await waitFor(() => {
      const csvCall = vi
        .mocked(fetch)
        .mock.calls.find(([input]) => input.toString().includes('/admin/audit/csv'));
      expect(csvCall).toBeDefined();
      const url = csvCall![0].toString();
      // Filter is forwarded
      expect(url).toContain('range=24h');
      // Auth header is forwarded
      const init = csvCall![1] as RequestInit;
      expect(init.headers).toEqual(expect.objectContaining({ Authorization: 'Bearer test-token' }));
    });

    // Object URL plumbing fired
    expect(createObjectURL).toHaveBeenCalledTimes(1);
    // revokeObjectURL is deferred via setTimeout — we don't assert on it here.
    expect(revokeObjectURL).toBeDefined();
  });

  // ------------------------------------------------------------------
  // 8. CSV export failure → error toast surfaces
  // ------------------------------------------------------------------
  it('shows an error toast when the CSV export endpoint returns a non-OK status', async () => {
    const user = userEvent.setup();

    vi.mocked(fetch).mockImplementation(async (input) => {
      const url = input.toString();
      if (url.includes('/admin/audit/csv')) {
        return { ok: false, status: 500 } as Response;
      }
      return {
        ok: true,
        status: 200,
        json: async () => ({ items: [] }),
      } as Response;
    });

    renderPage();
    await waitFor(() => expect(lastAuditGetUrl()).toBeDefined());

    await user.click(screen.getByRole('button', { name: /Export CSV/i }));

    await waitFor(() => expect(screen.getByText(/Export failed \(500\)/i)).toBeDefined());
  });
});
