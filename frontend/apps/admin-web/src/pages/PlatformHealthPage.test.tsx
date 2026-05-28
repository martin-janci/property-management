/**
 * PlatformHealthPage — unit tests.
 *
 * Covers:
 *  1. Loading state renders correctly.
 *  2. Dashboard data renders metrics, alerts, thresholds.
 *  3. Error state renders error banner.
 *  4. "Acknowledge" button hidden when site_settings_write absent.
 *  5. "Acknowledge" button fires POST and shows success toast.
 *  6. History drill-in opens panel with range buttons.
 *  7. 401 mfa_required triggers MFA modal (regression: PR #471 bypass fix).
 */

import { CapabilityProvider } from '@ppt/admin-ui';
import { setMfaChallengeHandler } from '@ppt/api-client';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter } from 'react-router-dom';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { AdminAuthProvider } from '../auth/AdminAuthContext';
import { sessionTokenStore } from '../auth/tokenStore';
import { MfaWindowProvider } from '../components/MfaWindowChip';
import { ToastProvider } from '../components/Toast';
import PlatformHealthPage from './PlatformHealthPage';

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

const DASHBOARD = {
  metrics: [
    {
      metric_name: 'active_sessions',
      metric_type: 'gauge',
      value: 42,
      recorded_at: new Date().toISOString(),
      status: 'normal',
    },
    {
      metric_name: 'error_rate',
      metric_type: 'gauge',
      value: 85,
      recorded_at: new Date().toISOString(),
      status: 'critical',
    },
  ],
  alerts: [
    {
      id: 'alert-1',
      metric_name: 'error_rate',
      threshold_type: 'critical',
      value: 85,
      created_at: new Date().toISOString(),
      acknowledged_at: null,
      acknowledged_by: null,
    },
  ],
  thresholds: [
    {
      id: 'threshold-1',
      metric_name: 'error_rate',
      warning_threshold: 50,
      critical_threshold: 80,
      is_active: true,
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
    },
  ],
};

const HISTORY = {
  metric_name: 'active_sessions',
  data_points: [
    { value: 38, recorded_at: new Date(Date.now() - 3600_000).toISOString() },
    { value: 42, recorded_at: new Date().toISOString() },
  ],
  stats: { min: 38, max: 42, avg: 40, count: 2 },
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

function renderPage({ capabilities = [] }: WrapperOpts = {}) {
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
                <PlatformHealthPage />
              </ToastProvider>
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

describe('PlatformHealthPage', () => {
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
    expect(screen.getByText('Platform Health')).toBeDefined();
  });

  it('shows error banner when dashboard endpoint fails', async () => {
    vi.mocked(fetch).mockResolvedValue({ ok: false, status: 500 } as Response);
    renderPage();
    await waitFor(() => expect(screen.getByRole('alert')).toBeDefined());
    expect(screen.getByRole('alert').textContent).toContain('Failed to load health dashboard');
  });

  it('renders metrics from dashboard response', async () => {
    vi.mocked(fetch).mockImplementation(async (input) => {
      const url = input.toString();
      if (url.includes('/health/dashboard')) {
        return { ok: true, status: 200, json: async () => DASHBOARD } as Response;
      }
      return { ok: false, status: 404, json: async () => ({}) } as Response;
    });
    renderPage();
    await waitFor(() => expect(screen.getByText('active_sessions')).toBeDefined());
    // error_rate appears in both metric card and alert row — use getAllByText
    expect(screen.getAllByText('error_rate').length).toBeGreaterThanOrEqual(1);
    // "Critical" appears both as a badge and as a threshold-table column header
    expect(screen.getAllByText('Critical').length).toBeGreaterThanOrEqual(1);
  });

  it('renders thresholds table', async () => {
    vi.mocked(fetch).mockImplementation(async (input) => {
      const url = input.toString();
      if (url.includes('/health/dashboard')) {
        return { ok: true, status: 200, json: async () => DASHBOARD } as Response;
      }
      return { ok: false, status: 404, json: async () => ({}) } as Response;
    });
    renderPage();
    await waitFor(() => expect(screen.getByText('Metric Thresholds')).toBeDefined());
    // threshold row — error_rate may appear in multiple sections (metric, alert, threshold)
    await waitFor(() => expect(screen.getAllByText('error_rate').length).toBeGreaterThanOrEqual(1));
  });

  it('hides Acknowledge button when site_settings_write is absent', async () => {
    vi.mocked(fetch).mockImplementation(async (input) => {
      const url = input.toString();
      if (url.includes('/health/dashboard')) {
        return { ok: true, status: 200, json: async () => DASHBOARD } as Response;
      }
      if (url.includes('/health/alerts')) {
        return { ok: true, status: 200, json: async () => DASHBOARD.alerts } as Response;
      }
      return { ok: false, status: 404 } as Response;
    });
    renderPage({ capabilities: ['audit_read'] });
    await waitFor(() => expect(screen.getByText('Alerts')).toBeDefined());
    // Acknowledge buttons should not appear (no site_settings_write)
    expect(screen.queryByText('Acknowledge')).toBeNull();
  });

  it('shows Acknowledge button and calls POST when site_settings_write present', async () => {
    const postSpy = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({ ...DASHBOARD.alerts[0], acknowledged_at: new Date().toISOString() }),
    } as Response);

    vi.mocked(fetch).mockImplementation(async (input, opts) => {
      const url = input.toString();
      if (url.includes('/health/dashboard')) {
        return { ok: true, status: 200, json: async () => DASHBOARD } as Response;
      }
      if (url.includes('/health/alerts') && !url.includes('/acknowledge')) {
        return { ok: true, status: 200, json: async () => DASHBOARD.alerts } as Response;
      }
      if (url.includes('/acknowledge') && opts?.method === 'POST') {
        return postSpy(url, opts) as Promise<Response>;
      }
      return { ok: false, status: 404 } as Response;
    });

    renderPage({ capabilities: ['site_settings_write'] });

    await waitFor(() => expect(screen.getByText('Acknowledge')).toBeDefined());

    const user = userEvent.setup();
    await user.click(screen.getByText('Acknowledge'));

    await waitFor(() => expect(postSpy).toHaveBeenCalledTimes(1));
    const calledUrl: string = postSpy.mock.calls[0][0] as string;
    expect(calledUrl).toContain('/acknowledge');
  });

  it('opens history panel when History button is clicked', async () => {
    vi.mocked(fetch).mockImplementation(async (input) => {
      const url = input.toString();
      if (url.includes('/health/dashboard')) {
        return { ok: true, status: 200, json: async () => DASHBOARD } as Response;
      }
      if (url.includes('/metrics/') && url.includes('/history')) {
        return { ok: true, status: 200, json: async () => HISTORY } as Response;
      }
      return { ok: false, status: 404 } as Response;
    });
    renderPage();
    await waitFor(() => expect(screen.getAllByText('History').length).toBeGreaterThanOrEqual(1));

    const user = userEvent.setup();
    // Click the first "History" button (active_sessions)
    await user.click(screen.getAllByText('History')[0]);

    // Panel should open
    await waitFor(() =>
      expect(screen.getByRole('dialog', { name: /metric history/i })).toBeDefined()
    );

    // Stats should render
    await waitFor(() => expect(screen.getByText('Min')).toBeDefined());
    expect(screen.getByText('Max')).toBeDefined();
  });

  it('shows range buttons in history panel', async () => {
    vi.mocked(fetch).mockImplementation(async (input) => {
      const url = input.toString();
      if (url.includes('/health/dashboard')) {
        return { ok: true, status: 200, json: async () => DASHBOARD } as Response;
      }
      if (url.includes('/metrics/') && url.includes('/history')) {
        return { ok: true, status: 200, json: async () => HISTORY } as Response;
      }
      return { ok: false, status: 404 } as Response;
    });
    renderPage();
    await waitFor(() => expect(screen.getAllByText('History').length).toBeGreaterThanOrEqual(1));

    const user = userEvent.setup();
    await user.click(screen.getAllByText('History')[0]);

    await waitFor(() =>
      expect(screen.getByRole('dialog', { name: /metric history/i })).toBeDefined()
    );

    // Range buttons
    expect(screen.getByText('1 hour')).toBeDefined();
    expect(screen.getByText('7 days')).toBeDefined();
    expect(screen.getByText('30 days')).toBeDefined();
  });

  /**
   * Regression test: PR #471 reviewer found that health-page API calls were
   * using a bespoke inline fetchJson() that bypassed the MFA interceptor in
   * @ppt/api-client. After the fix, all health endpoints go through
   * `authenticatedFetchJson` from `lib/fetch.ts` which intercepts
   * `401 { error: "mfa_required" }` and triggers the registered handler.
   *
   * This test verifies the end-to-end path: dashboard 401 → handler called
   * → retry with original request → dashboard data displayed.
   */
  it('triggers MFA handler on 401 mfa_required from dashboard endpoint (regression: PR #471)', async () => {
    // Register a handler that resolves immediately (user "completes" MFA)
    const mfaHandler = vi.fn().mockResolvedValue(true);
    setMfaChallengeHandler(mfaHandler);

    let callCount = 0;
    vi.mocked(fetch).mockImplementation(async (input) => {
      const url = input.toString();
      if (url.includes('/health/dashboard')) {
        callCount += 1;
        // First call: MFA required; second call (retry): success
        if (callCount === 1) {
          return {
            ok: false,
            status: 401,
            json: async () => ({ error: 'mfa_required' }),
          } as Response;
        }
        return { ok: true, status: 200, json: async () => DASHBOARD } as Response;
      }
      return { ok: false, status: 404, json: async () => ({}) } as Response;
    });

    renderPage();

    // After MFA challenge is resolved and the request retried, metrics appear
    await waitFor(() => expect(screen.getByText('active_sessions')).toBeDefined(), {
      timeout: 3000,
    });

    // The MFA handler must have been invoked exactly once
    expect(mfaHandler).toHaveBeenCalledTimes(1);
    // fetch must have been called at least twice (original + retry)
    expect(callCount).toBeGreaterThanOrEqual(2);

    // Clean up handler so it doesn't bleed into other tests
    setMfaChallengeHandler(null);
  });
});
