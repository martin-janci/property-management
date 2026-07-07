/**
 * OrganizationsPage — unit tests (Epic 10B, Story 10B.1).
 *
 * Covers:
 *  1. Page heading renders.
 *  2. Error banner when the list endpoint fails.
 *  3. Organization rows + platform stats cards render from responses.
 *  4. Empty state renders when the platform has no organizations.
 *  5. Suspend/Reactivate buttons hidden without `agencies_suspend`.
 *  6. Suspend flow: confirmation dialog requires a reason, POSTs it.
 *  7. Reactivate flow: confirmation dialog POSTs to /reactivate.
 *  8. Details dialog fetches and renders the org detail (incl. suspension).
 */

import { CapabilityProvider } from '@ppt/admin-ui';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter } from 'react-router-dom';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { AdminAuthProvider } from '../auth/AdminAuthContext';
import { sessionTokenStore } from '../auth/tokenStore';
import { MfaWindowProvider } from '../components/MfaWindowChip';
import { ToastProvider } from '../components/Toast';
import OrganizationsPage from './OrganizationsPage';

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

const ORG_ACTIVE = {
  id: '11111111-1111-1111-1111-111111111111',
  name: 'Acme Property',
  slug: 'acme-property',
  status: 'active',
  member_count: 12,
  building_count: 3,
  created_at: '2026-01-15T10:00:00Z',
};

const ORG_SUSPENDED = {
  id: '22222222-2222-2222-2222-222222222222',
  name: 'Shady Homes',
  slug: 'shady-homes',
  status: 'suspended',
  member_count: 5,
  building_count: 1,
  created_at: '2026-02-20T10:00:00Z',
};

const LIST = {
  organizations: [ORG_ACTIVE, ORG_SUSPENDED],
  total: 2,
  page: 1,
  page_size: 20,
};

const STATS = {
  stats: {
    active_orgs: 41,
    suspended_orgs: 2,
    active_users: 1234,
    total_buildings: 87,
    total_units: 950,
  },
};

const DETAIL = {
  id: ORG_SUSPENDED.id,
  name: ORG_SUSPENDED.name,
  slug: ORG_SUSPENDED.slug,
  contact_email: 'owner@shady-homes.example',
  logo_url: null,
  status: 'suspended',
  created_at: '2026-02-20T10:00:00Z',
  updated_at: '2026-03-01T10:00:00Z',
  suspended_at: '2026-03-01T10:00:00Z',
  suspended_by: '99999999-9999-9999-9999-999999999999',
  suspension_reason: 'Unpaid invoices',
  metrics: {
    member_count: 5,
    active_member_count: 4,
    building_count: 1,
    unit_count: 20,
  },
};

const ACTION_RESPONSE = {
  message: 'Organization suspended successfully. 5 user sessions revoked.',
  organization: DETAIL,
};

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

function makeQc() {
  return new QueryClient({ defaultOptions: { queries: { retry: false, retryDelay: 0 } } });
}

interface WrapperOpts {
  capabilities?: string[];
}

function renderPage({ capabilities = ['agencies_read', 'audit_read'] }: WrapperOpts = {}) {
  sessionStorage.clear();
  sessionTokenStore.set('test-token');
  const qc = makeQc();

  return render(
    <QueryClientProvider client={qc}>
      <MemoryRouter>
        <AdminAuthProvider>
          <CapabilityProvider
            value={{ isPlatformPrincipal: true, capabilities: capabilities as never[] }}
          >
            <MfaWindowProvider>
              <ToastProvider>
                <OrganizationsPage />
              </ToastProvider>
            </MfaWindowProvider>
          </CapabilityProvider>
        </AdminAuthProvider>
      </MemoryRouter>
    </QueryClientProvider>
  );
}

/** fetch mock serving list + stats; returns 404 for the rest. */
function mockListAndStats() {
  vi.mocked(fetch).mockImplementation(async (input) => {
    const url = input.toString();
    if (url.includes('/platform-admin/stats')) {
      return { ok: true, status: 200, json: async () => STATS } as Response;
    }
    if (url.includes('/platform-admin/organizations?')) {
      return { ok: true, status: 200, json: async () => LIST } as Response;
    }
    return { ok: false, status: 404, json: async () => ({}) } as Response;
  });
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe('OrganizationsPage', () => {
  beforeEach(() => {
    vi.spyOn(console, 'warn').mockImplementation(() => {});
    vi.spyOn(console, 'error').mockImplementation(() => {});
    vi.spyOn(globalThis, 'fetch');
  });

  afterEach(() => {
    sessionStorage.clear();
    vi.restoreAllMocks();
  });

  it('renders page heading', async () => {
    vi.mocked(fetch).mockResolvedValue({ ok: false, status: 404 } as Response);
    renderPage();
    expect(screen.getByText('Organizations')).toBeDefined();
  });

  it('shows error banner when the list endpoint fails', async () => {
    vi.mocked(fetch).mockResolvedValue({ ok: false, status: 500 } as Response);
    renderPage();
    await waitFor(() => expect(screen.getByRole('alert')).toBeDefined());
    expect(screen.getByRole('alert').textContent).toContain('Failed to load organizations');
  });

  it('renders organization rows and platform stats cards', async () => {
    mockListAndStats();
    renderPage();
    await waitFor(() => expect(screen.getByText('Acme Property')).toBeDefined());
    expect(screen.getByText('Shady Homes')).toBeDefined();
    expect(screen.getByText('acme-property')).toBeDefined();
    // status badges
    expect(screen.getByText('active')).toBeDefined();
    // "suspended" appears as badge (row) — at least once
    expect(screen.getAllByText('suspended').length).toBeGreaterThanOrEqual(1);
    // stats cards
    await waitFor(() => expect(screen.getByText('Active orgs')).toBeDefined());
    expect(screen.getByText('41')).toBeDefined();
    expect(screen.getByText('1,234')).toBeDefined();
  });

  it('renders empty state when there are no organizations', async () => {
    vi.mocked(fetch).mockImplementation(async (input) => {
      const url = input.toString();
      if (url.includes('/platform-admin/organizations?')) {
        return {
          ok: true,
          status: 200,
          json: async () => ({ organizations: [], total: 0, page: 1, page_size: 20 }),
        } as Response;
      }
      return { ok: false, status: 404, json: async () => ({}) } as Response;
    });
    renderPage();
    await waitFor(() => expect(screen.getByText('No organizations.')).toBeDefined());
  });

  it('hides Suspend/Reactivate buttons without agencies_suspend', async () => {
    mockListAndStats();
    renderPage({ capabilities: ['agencies_read', 'audit_read'] });
    await waitFor(() => expect(screen.getByText('Acme Property')).toBeDefined());
    expect(screen.queryByText('Suspend')).toBeNull();
    expect(screen.queryByText('Reactivate')).toBeNull();
    // Details drill-in is still available
    expect(screen.getAllByText('Details').length).toBe(2);
  });

  it('suspend flow requires a reason and POSTs it to /suspend', async () => {
    const postSpy = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ACTION_RESPONSE,
    } as Response);

    vi.mocked(fetch).mockImplementation(async (input, opts) => {
      const url = input.toString();
      if (url.includes('/suspend') && opts?.method === 'POST') {
        return postSpy(url, opts) as Promise<Response>;
      }
      if (url.includes('/platform-admin/organizations?')) {
        return { ok: true, status: 200, json: async () => LIST } as Response;
      }
      if (url.includes('/platform-admin/stats')) {
        return { ok: true, status: 200, json: async () => STATS } as Response;
      }
      return { ok: false, status: 404, json: async () => ({}) } as Response;
    });

    renderPage({ capabilities: ['agencies_read', 'agencies_suspend', 'audit_read'] });
    await waitFor(() => expect(screen.getByText('Suspend')).toBeDefined());

    const user = userEvent.setup();
    await user.click(screen.getByText('Suspend'));

    // Confirmation dialog opens
    const dialog = await screen.findByRole('dialog', { name: /suspend organization/i });
    expect(dialog).toBeDefined();

    // Confirm button disabled until a reason is typed
    const confirmBtn = screen.getByRole('button', { name: 'Suspend organization' });
    expect((confirmBtn as HTMLButtonElement).disabled).toBe(true);

    await user.type(
      screen.getByPlaceholderText('e.g. Unpaid invoices'),
      'Terms of service violation'
    );
    expect((confirmBtn as HTMLButtonElement).disabled).toBe(false);
    await user.click(confirmBtn);

    await waitFor(() => expect(postSpy).toHaveBeenCalledTimes(1));
    const [calledUrl, calledOpts] = postSpy.mock.calls[0] as [string, RequestInit];
    expect(calledUrl).toContain(`/platform-admin/organizations/${ORG_ACTIVE.id}/suspend`);
    expect(JSON.parse(calledOpts.body as string)).toEqual({
      reason: 'Terms of service violation',
    });
  });

  it('reactivate flow POSTs to /reactivate for a suspended org', async () => {
    const postSpy = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({ ...ACTION_RESPONSE, message: 'Organization reactivated successfully' }),
    } as Response);

    vi.mocked(fetch).mockImplementation(async (input, opts) => {
      const url = input.toString();
      if (url.includes('/reactivate') && opts?.method === 'POST') {
        return postSpy(url, opts) as Promise<Response>;
      }
      if (url.includes('/platform-admin/organizations?')) {
        return { ok: true, status: 200, json: async () => LIST } as Response;
      }
      if (url.includes('/platform-admin/stats')) {
        return { ok: true, status: 200, json: async () => STATS } as Response;
      }
      return { ok: false, status: 404, json: async () => ({}) } as Response;
    });

    renderPage({ capabilities: ['agencies_read', 'agencies_suspend', 'audit_read'] });
    await waitFor(() => expect(screen.getByText('Reactivate')).toBeDefined());

    const user = userEvent.setup();
    await user.click(screen.getByText('Reactivate'));

    await screen.findByRole('dialog', { name: /reactivate organization/i });
    await user.click(screen.getByRole('button', { name: 'Reactivate organization' }));

    await waitFor(() => expect(postSpy).toHaveBeenCalledTimes(1));
    const calledUrl: string = postSpy.mock.calls[0][0] as string;
    expect(calledUrl).toContain(`/platform-admin/organizations/${ORG_SUSPENDED.id}/reactivate`);
  });

  it('details dialog fetches and renders the org detail with suspension info', async () => {
    vi.mocked(fetch).mockImplementation(async (input) => {
      const url = input.toString();
      if (url.includes(`/platform-admin/organizations/${ORG_SUSPENDED.id}`)) {
        return { ok: true, status: 200, json: async () => DETAIL } as Response;
      }
      if (url.includes('/platform-admin/organizations?')) {
        return { ok: true, status: 200, json: async () => LIST } as Response;
      }
      if (url.includes('/platform-admin/stats')) {
        return { ok: true, status: 200, json: async () => STATS } as Response;
      }
      return { ok: false, status: 404, json: async () => ({}) } as Response;
    });

    renderPage();
    await waitFor(() => expect(screen.getByText('Shady Homes')).toBeDefined());

    const user = userEvent.setup();
    // Second row (Shady Homes) Details button
    await user.click(screen.getAllByText('Details')[1]);

    await screen.findByRole('dialog', { name: /organization details/i });
    await waitFor(() => expect(screen.getByText('owner@shady-homes.example')).toBeDefined());
    expect(screen.getByText(/Unpaid invoices/)).toBeDefined();
    // metrics: members "5 (4 active)"
    expect(screen.getByText('5 (4 active)')).toBeDefined();
  });
});
