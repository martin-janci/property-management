/**
 * Dashboard — fallback chain + capability gating tests.
 */

import { CapabilityProvider } from '@ppt/admin-ui';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { AdminAuthProvider } from '../auth/AdminAuthContext';
import { sessionTokenStore } from '../auth/tokenStore';
import { MfaWindowProvider } from '../components/MfaWindowChip';
import { Dashboard } from './Dashboard';

// ---------------------------------------------------------------------------
// Providers helper
// ---------------------------------------------------------------------------

function makeQueryClient() {
  return new QueryClient({
    defaultOptions: { queries: { retry: false, retryDelay: 0 } },
  });
}

interface WrapperOptions {
  token?: string;
  capabilities?: string[];
  isPlatformPrincipal?: boolean;
}

function renderDashboard({
  token = 'test-token',
  capabilities = [],
  isPlatformPrincipal = true,
}: WrapperOptions = {}) {
  sessionStorage.clear();
  sessionTokenStore.set(token);
  const qc = makeQueryClient();

  return render(
    <QueryClientProvider client={qc}>
      <MemoryRouter>
        <AdminAuthProvider>
          <CapabilityProvider
            value={{ isPlatformPrincipal, capabilities: capabilities as never[] }}
          >
            <MfaWindowProvider>
              <Dashboard />
            </MfaWindowProvider>
          </CapabilityProvider>
        </AdminAuthProvider>
      </MemoryRouter>
    </QueryClientProvider>
  );
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe('Dashboard', () => {
  beforeEach(() => {
    vi.spyOn(console, 'warn').mockImplementation(() => {});
    vi.spyOn(globalThis, 'fetch');
  });

  afterEach(() => {
    sessionStorage.clear();
    vi.restoreAllMocks();
  });

  // 1. Metrics endpoint error → query errors, tiles fall back to zero via the
  //    consumer's `?? 0` defaults (the dead "stub fallback" path was removed).
  it('renders tiles with zero values when the metrics endpoint errors', async () => {
    vi.mocked(fetch).mockImplementation(async (input) => {
      const url = input.toString();
      if (url.includes('metrics/summary')) {
        return { ok: false, status: 500 } as Response;
      }
      return { ok: false, status: 404 } as Response;
    });

    renderDashboard();

    // Wait for tiles to render (not skeleton anymore)
    await waitFor(() => expect(screen.getByText('Tenants active')).toBeDefined());

    // No metrics data → tiles fall back to 0 via `metrics?.field ?? 0`.
    const values = screen.getAllByText('0');
    expect(values.length).toBeGreaterThanOrEqual(3);
  });

  // 2. Metrics endpoint 200 → tiles render with values
  it('renders tiles with API values on 200 response', async () => {
    vi.mocked(fetch).mockImplementation(async (input) => {
      const url = input.toString();
      if (url.includes('metrics/summary')) {
        return {
          ok: true,
          status: 200,
          // Backend serializes camelCase (`MetricsSummary` has
          // `#[serde(rename_all = "camelCase")]`) — mirror the real wire
          // shape so this test would catch a regression in the Dashboard
          // adapter.
          json: async () => ({
            tenantsActive: 42,
            operatorsOnline: 7,
            pendingMerges: 3,
            highRisk24h: 5,
          }),
        } as Response;
      }
      return { ok: false, status: 404 } as Response;
    });

    renderDashboard({ capabilities: [] });

    await waitFor(() => expect(screen.getByText('42')).toBeDefined());
    expect(screen.getByText('7')).toBeDefined();
    expect(screen.getByText('3')).toBeDefined();
    expect(screen.getByText('5')).toBeDefined();
  });

  // 3. Capability audit_read absent → "Recent high-risk events" card not rendered
  it('hides high-risk events card when audit_read capability is absent', async () => {
    vi.mocked(fetch).mockResolvedValue({ ok: false, status: 404 } as Response);

    renderDashboard({ capabilities: [] });

    await waitFor(() => expect(screen.getByText('Tenants active')).toBeDefined());

    expect(screen.queryByText(/Recent high-risk events/i)).toBeNull();
  });

  // 4. audit_read present + audit endpoint 404 → card renders empty state
  it('renders empty state when audit_read present but audit endpoint returns 404', async () => {
    vi.mocked(fetch).mockImplementation(async (input) => {
      const url = input.toString();
      if (url.includes('metrics/summary')) {
        return { ok: false, status: 404 } as Response;
      }
      if (url.includes('audit')) {
        return { ok: false, status: 404 } as Response;
      }
      return { ok: false, status: 404 } as Response;
    });

    renderDashboard({ capabilities: ['audit_read'] });

    await waitFor(() => expect(screen.getByText(/Recent high-risk events/i)).toBeDefined());

    await waitFor(() =>
      expect(screen.getByText(/No high-risk events in the last 24 hours/i)).toBeDefined()
    );
  });

  // 5. audit_read present + audit endpoint 200 → events list renders
  it('renders events list when audit_read present and audit endpoint returns data', async () => {
    vi.mocked(fetch).mockImplementation(async (input) => {
      const url = input.toString();
      if (url.includes('metrics/summary')) {
        return { ok: false, status: 404 } as Response;
      }
      if (url.includes('audit')) {
        return {
          ok: true,
          status: 200,
          json: async () => ({
            items: [
              {
                id: 'e1',
                actor_email: 'admin@example.com',
                capability: 'tenant_purge',
                target_slug: 'acme-corp',
                severity: 'high',
                occurred_at: new Date().toISOString(),
                outcome: 'success',
              },
            ],
          }),
        } as Response;
      }
      return { ok: false, status: 404 } as Response;
    });

    renderDashboard({ capabilities: ['audit_read'] });

    await waitFor(() => expect(screen.getByText('admin@example.com')).toBeDefined());
    expect(screen.getByText('tenant_purge')).toBeDefined();
  });

  // 6. All quick-action capabilities absent → "Quick actions" card hidden
  it('hides Quick actions card when all gating capabilities are absent', async () => {
    vi.mocked(fetch).mockResolvedValue({ ok: false, status: 404 } as Response);

    renderDashboard({ capabilities: [] });

    await waitFor(() => expect(screen.getByText('Tenants active')).toBeDefined());

    expect(screen.queryByText('Quick actions')).toBeNull();
  });

  // Quick actions visible with audit_read
  it('renders Quick actions card when audit_read is present', async () => {
    vi.mocked(fetch).mockResolvedValue({ ok: false, status: 404 } as Response);

    renderDashboard({ capabilities: ['audit_read'] });

    await waitFor(() => expect(screen.getByText('Quick actions')).toBeDefined());
  });
});
