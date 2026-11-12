/**
 * Leases route group — list error-state regression test.
 *
 * Regression for the code-review finding "ppt-web leases list swallows its
 * query error": `LeasesListPageRoute` destructured only `{ data, isLoading }`
 * from `useLeases`, so a failed leases query fell through to the empty state
 * ("No leases found") with no error surfaced and no retry. This pins the
 * sibling-list error pattern (role="alert" + retry → refetch) for the leases
 * list, matching faults / announcements (`isError={!!error}` + `onRetry`).
 */

/// <reference types="vitest/globals" />
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { Suspense } from 'react';
import { MemoryRouter, Routes } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';

// ── i18n: return the key (or defaultValue) so assertions are locale-agnostic ──
vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, opts?: { defaultValue?: string }) => opts?.defaultValue ?? key,
    i18n: { language: 'en' },
  }),
}));

// ── toast + route guard ──
vi.mock('../../components', () => ({
  useToast: () => ({ showToast: vi.fn() }),
  ProtectedRoute: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));

// ── auth role ──
vi.mock('../../contexts', () => ({
  useAuth: () => ({ user: { role: 'manager' } }),
}));

// ── organization ──
vi.mock('../../hooks/useOrganization', () => ({
  useOrganization: () => ({ organizationId: 'org-1', organizationName: 'Acme s.r.o.' }),
}));

// ── api-client: configurable useLeases result + refetch spy ──
const { leasesState, refetchSpy } = vi.hoisted(() => ({
  leasesState: { current: {} as Record<string, unknown> },
  refetchSpy: vi.fn(),
}));

vi.mock('@ppt/api-client', () => ({
  useLeases: () => ({ refetch: refetchSpy, ...leasesState.current }),
  // Unused by the list route but imported by the module under test.
  useBuildings: () => ({ data: { items: [] }, isLoading: false }),
}));

import { leaseRoutes } from './leases';

function renderList() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <MemoryRouter initialEntries={['/leases/list']}>
        <Suspense fallback={<div>loading…</div>}>
          <Routes>{leaseRoutes()}</Routes>
        </Suspense>
      </MemoryRouter>
    </QueryClientProvider>
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  leasesState.current = {};
});

describe('LeasesListPageRoute — query error', () => {
  it('surfaces the error state with a retry when the leases query fails', async () => {
    leasesState.current = { data: undefined, isLoading: false, isError: true };
    renderList();

    const alert = await screen.findByRole('alert');
    expect(alert).toHaveTextContent('Failed to load leases');
    // The empty-state ("No leases") must NOT show when the query errored.
    expect(screen.queryByText('leases.list.noLeases')).not.toBeInTheDocument();

    await userEvent.click(screen.getByRole('button', { name: 'Retry' }));
    await waitFor(() => expect(refetchSpy).toHaveBeenCalledTimes(1));
  });

  it('renders the empty state (not the error state) on a successful empty result', async () => {
    leasesState.current = { data: { items: [], total: 0 }, isLoading: false, isError: false };
    renderList();

    expect(await screen.findByText('leases.list.noLeases')).toBeInTheDocument();
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });
});
