/// <reference types="vitest/globals" />
/**
 * Outages list fetch-error regression (code-review finding
 * code-review-ppt-web-ui-outages-page-swallows-query-error).
 *
 * `OutagesPageRoute` destructured only `{ data, isLoading }` from `useOutages`
 * and dropped `isError`/`refetch`. On a failed query `data` is undefined, so
 * `outages` collapsed to `[]` and the `OutageList` rendered its "No outages
 * found" empty state — a genuine fetch failure (500, network drop, auth loss)
 * was indistinguishable from an organization that legitimately has no outages,
 * i.e. the error was swallowed.
 *
 * The fix surfaces the query error with the standard error-state UI used by the
 * sibling FacilitiesPage: a dedicated error block (title + description + retry)
 * instead of the empty placeholder. These tests pin that contract:
 *
 *  - on a query error the error title is shown and the empty-state copy is NOT
 *    (fails on `main`, where the empty state leaks through);
 *  - the retry button re-issues the query (`refetch`);
 *  - a successful empty result still shows the empty placeholder, not the error.
 */

import { fireEvent, render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import en from '../../../messages/en.json';
import '../../i18n';

// ─── network boundary: @ppt/api-client ──────────────────────────────────────
const useOutages = vi.fn();

vi.mock('@ppt/api-client', () => ({
  useOutages: (...args: unknown[]) => useOutages(...args),
}));

// ─── auth: user with a real organization so the page (not the gate) mounts ───
vi.mock('../../contexts', () => ({
  useAuth: () => ({ user: { id: 'u-1', organizationId: 'org-1', role: 'manager' } }),
}));

// ─── component stubs pulled in at module load by the route group ─────────────
vi.mock('../../components', () => ({
  AuthRequiredGate: () => <div data-testid="auth-gate">auth required</div>,
  ProtectedRoute: ({ children }: { children: React.ReactNode }) => <>{children}</>,
  useToast: () => ({ showToast: vi.fn() }),
}));

// ─── render the REAL OutagesPage (and OutageList) so the error UI is exercised ─
vi.mock('../lazyRoutes', async () => {
  const real = await vi.importActual<typeof import('../../features/outages')>(
    '../../features/outages'
  );
  return {
    OutagesPage: real.OutagesPage,
    CreateOutagePage: () => null,
    EditOutagePage: () => null,
    ViewOutagePage: () => null,
  };
});

import { OutagesPageRoute } from './outages';

const ERR = en.outages.errors;
const EMPTY = en.outages.noResults;

function renderRoute() {
  return render(
    <MemoryRouter initialEntries={['/outages']}>
      <OutagesPageRoute />
    </MemoryRouter>
  );
}

describe('OutagesPageRoute fetch error state', () => {
  beforeEach(() => {
    useOutages.mockReset();
  });

  it('shows a distinct error state (not the empty placeholder) when the query fails', () => {
    useOutages.mockReturnValue({
      data: undefined,
      isLoading: false,
      isError: true,
      refetch: vi.fn(),
    });

    renderRoute();

    expect(screen.getByText(ERR.fetchFailedTitle)).toBeInTheDocument();
    expect(screen.getByText(ERR.fetchFailedDescription)).toBeInTheDocument();
    // The empty-state copy must NOT be shown for a failed query.
    expect(screen.queryByText(EMPTY)).not.toBeInTheDocument();
  });

  it('retries the query when the retry button is clicked', () => {
    const refetch = vi.fn();
    useOutages.mockReturnValue({
      data: undefined,
      isLoading: false,
      isError: true,
      refetch,
    });

    renderRoute();

    fireEvent.click(screen.getByRole('button', { name: ERR.retry }));
    expect(refetch).toHaveBeenCalledTimes(1);
  });

  it('shows the empty placeholder (not the error state) on a successful empty result', () => {
    useOutages.mockReturnValue({
      data: { outages: [], total: 0 },
      isLoading: false,
      isError: false,
      refetch: vi.fn(),
    });

    renderRoute();

    expect(screen.getByText(EMPTY)).toBeInTheDocument();
    expect(screen.queryByText(ERR.fetchFailedTitle)).not.toBeInTheDocument();
  });
});
