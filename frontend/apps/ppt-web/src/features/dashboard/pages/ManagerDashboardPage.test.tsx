/// <reference types="vitest/globals" />
/**
 * ManagerDashboardPage defensive-layout tests (Task 3).
 *
 * Verifies that the page renders the DEFAULT_DASHBOARD_LAYOUT (i.e. the stats
 * section is visible) even when the layout endpoint is down — spec §4: never
 * gate the page on a layout fetch.
 */
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { describe, expect, it, vi } from 'vitest';
import { ManagerDashboardPage } from './ManagerDashboardPage';

// Mock the layout fetch so it fails (network down scenario).
vi.mock('@ppt/api-client', async (importOriginal) => ({
  ...(await importOriginal<object>()),
  fetchResolvedLayout: vi.fn().mockRejectedValue(new Error('network down')),
}));

// Stub i18n — all keys pass through as-is.
vi.mock('react-i18next', () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

// Stub useActionQueue so the ActionQueue section renders without a QueryClient
// hitting real network calls that would cascade errors.
vi.mock('../../hooks/useActionQueue', () => ({
  useActionQueue: vi.fn(() => ({
    items: [],
    total: 0,
    stats: { total: 0, urgent: 0, high: 0, medium: 0, low: 0 },
    isLoading: false,
    isError: false,
    error: null,
    refetch: vi.fn(),
    dismissItem: vi.fn(),
    handleAction: vi.fn(),
    isExecuting: false,
  })),
}));

function renderPage() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <MemoryRouter>
        <ManagerDashboardPage />
      </MemoryRouter>
    </QueryClientProvider>
  );
}

describe('ManagerDashboardPage (defensive layout)', () => {
  it('renders the default layout when the layout endpoint is down', async () => {
    renderPage();
    await waitFor(() => {
      // Both required default sections present despite fetch failure.
      expect(document.querySelector('.dashboard-page__stats')).toBeInTheDocument();
    });
  });
});
