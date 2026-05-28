/**
 * SupportDataPage — unit tests (gap-10b-5-support-data-ui).
 *
 * Covers:
 *  1. Page heading renders.
 *  2. Loading state shows skeleton text.
 *  3. Error banner shown when endpoint fails.
 *  4. Stat cards render user counts and active sessions.
 *  5. Fault-by-status table renders rows and percentages.
 *  6. "No faults recorded" shown when fault list is empty.
 *  7. Refresh button calls refetch.
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
import SupportDataPage from './SupportDataPage';

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

const SUPPORT_DATA = {
  data: {
    total_users: 1234,
    active_users: 1100,
    pending_users: 80,
    suspended_users: 54,
    active_sessions: 312,
    total_faults: 77,
    fault_by_status: [
      { status: 'open', count: 45 },
      { status: 'resolved', count: 22 },
      { status: 'closed', count: 10 },
    ],
  },
};

const EMPTY_FAULTS_DATA = {
  data: {
    ...SUPPORT_DATA.data,
    total_faults: 0,
    fault_by_status: [],
  },
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function makeQc() {
  return new QueryClient({ defaultOptions: { queries: { retry: false, retryDelay: 0 } } });
}

function renderPage() {
  sessionStorage.clear();
  sessionTokenStore.set('test-token');
  const qc = makeQc();

  return render(
    <QueryClientProvider client={qc}>
      <MemoryRouter>
        <AdminAuthProvider>
          <CapabilityProvider
            value={{ isPlatformPrincipal: true, capabilities: ['audit_read' as never] }}
          >
            <MfaWindowProvider>
              <ToastProvider>
                <SupportDataPage />
              </ToastProvider>
            </MfaWindowProvider>
          </CapabilityProvider>
        </AdminAuthProvider>
      </MemoryRouter>
    </QueryClientProvider>
  );
}

function mockFetch(handler: (url: string) => Response | null) {
  vi.mocked(fetch).mockImplementation(async (input) => {
    const url = input.toString();
    const result = handler(url);
    if (result) return result;
    return { ok: false, status: 404, json: async () => ({}) } as Response;
  });
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe('SupportDataPage', () => {
  beforeEach(() => {
    vi.spyOn(console, 'warn').mockImplementation(() => {});
    vi.spyOn(console, 'error').mockImplementation(() => {});
    vi.spyOn(globalThis, 'fetch');
  });

  afterEach(() => {
    sessionStorage.clear();
    vi.restoreAllMocks();
  });

  it('renders page heading', () => {
    vi.mocked(fetch).mockResolvedValue({ ok: false, status: 404 } as Response);
    renderPage();
    expect(screen.getByText('Support Data')).toBeDefined();
  });

  it('shows loading text while fetching', () => {
    // Never resolve — stays in loading state
    vi.mocked(fetch).mockImplementation(() => new Promise(() => {}));
    renderPage();
    const loadingEls = screen.getAllByText('Loading…');
    expect(loadingEls.length).toBeGreaterThanOrEqual(1);
  });

  it('shows error banner when endpoint fails', async () => {
    mockFetch(() => ({ ok: false, status: 500, json: async () => ({}) }) as Response);
    renderPage();
    await waitFor(() => expect(screen.getByRole('alert')).toBeDefined());
    expect(screen.getByRole('alert').textContent).toContain('Failed to load support data');
  });

  it('renders user count stat cards', async () => {
    mockFetch((url) => {
      if (url.includes('/support-data')) {
        return { ok: true, status: 200, json: async () => SUPPORT_DATA } as Response;
      }
      return null;
    });
    renderPage();

    await waitFor(() => expect(screen.getByText('1,234')).toBeDefined());
    expect(screen.getByText('1,100')).toBeDefined();
    expect(screen.getByText('80')).toBeDefined();
    expect(screen.getByText('54')).toBeDefined();
  });

  it('renders active sessions stat card', async () => {
    mockFetch((url) => {
      if (url.includes('/support-data')) {
        return { ok: true, status: 200, json: async () => SUPPORT_DATA } as Response;
      }
      return null;
    });
    renderPage();

    await waitFor(() => expect(screen.getByText('312')).toBeDefined());
  });

  it('renders fault-by-status table rows', async () => {
    mockFetch((url) => {
      if (url.includes('/support-data')) {
        return { ok: true, status: 200, json: async () => SUPPORT_DATA } as Response;
      }
      return null;
    });
    renderPage();

    await waitFor(() => expect(screen.getByText('open')).toBeDefined());
    expect(screen.getByText('resolved')).toBeDefined();
    expect(screen.getByText('closed')).toBeDefined();
  });

  it('renders fault percentage column', async () => {
    mockFetch((url) => {
      if (url.includes('/support-data')) {
        return { ok: true, status: 200, json: async () => SUPPORT_DATA } as Response;
      }
      return null;
    });
    renderPage();

    // 45 / 77 ≈ 58.4 %
    await waitFor(() => {
      const pctCells = screen.getAllByText(/\d+(\.\d+)? %/);
      expect(pctCells.length).toBeGreaterThanOrEqual(1);
    });
  });

  it('shows "No faults recorded" when fault list is empty', async () => {
    mockFetch((url) => {
      if (url.includes('/support-data')) {
        return { ok: true, status: 200, json: async () => EMPTY_FAULTS_DATA } as Response;
      }
      return null;
    });
    renderPage();

    await waitFor(() => expect(screen.getByText('No faults recorded.')).toBeDefined());
  });

  it('calls refetch when Refresh button is clicked', async () => {
    let callCount = 0;
    mockFetch((url) => {
      if (url.includes('/support-data')) {
        callCount++;
        return { ok: true, status: 200, json: async () => SUPPORT_DATA } as Response;
      }
      return null;
    });
    renderPage();

    await waitFor(() => expect(screen.getByText('1,234')).toBeDefined());
    const initialCount = callCount;

    const user = userEvent.setup();
    await user.click(screen.getByText('Refresh'));

    await waitFor(() => expect(callCount).toBeGreaterThan(initialCount));
  });
});
