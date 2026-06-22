/// <reference types="vitest/globals" />
/**
 * UnitsSection tests (Story 3.2 / BIT-188).
 *
 * Exercises the manager-facing unit-management surface: empty state, list
 * rendering, create-form validation, and the archive confirmation flow.
 */

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import type { ReactNode } from 'react';
import { UnitsSection } from './UnitsSection';

const mockFetch = vi.fn();
global.fetch = mockFetch as unknown as typeof fetch;

function jsonResponse(body: unknown, ok = true, status = 200) {
  return Promise.resolve({
    ok,
    status,
    json: () => Promise.resolve(body),
  });
}

function createWrapper() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );
}

const sampleUnit = {
  id: 'unit-1',
  buildingId: 'b-1',
  designation: '3B',
  floor: 2,
  unitType: 'apartment',
  occupancyStatus: 'vacant',
  status: 'active',
};

beforeEach(() => {
  vi.clearAllMocks();
});

describe('UnitsSection', () => {
  it('shows the empty state when the building has no units', async () => {
    mockFetch.mockImplementation(() => jsonResponse({ units: [], total: 0, offset: 0, limit: 50 }));

    render(<UnitsSection buildingId="b-1" />, { wrapper: createWrapper() });

    expect(await screen.findByText('No units defined for this building yet.')).toBeInTheDocument();
  });

  it('renders the unit list when units exist', async () => {
    mockFetch.mockImplementation(() =>
      jsonResponse({ units: [sampleUnit], total: 1, offset: 0, limit: 50 })
    );

    render(<UnitsSection buildingId="b-1" />, { wrapper: createWrapper() });

    expect(await screen.findByText('3B')).toBeInTheDocument();
    expect(screen.getByText(/Apartment · Floor 2 · vacant/)).toBeInTheDocument();
  });

  it('surfaces a load error with a retry affordance', async () => {
    mockFetch.mockImplementation(() => jsonResponse({ message: 'boom' }, false, 500));

    render(<UnitsSection buildingId="b-1" />, { wrapper: createWrapper() });

    expect(await screen.findByText('boom')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Try again' })).toBeInTheDocument();
  });

  it('validates that designation is required before creating', async () => {
    mockFetch.mockImplementation(() => jsonResponse({ units: [], total: 0, offset: 0, limit: 50 }));

    render(<UnitsSection buildingId="b-1" />, { wrapper: createWrapper() });
    await screen.findByText('No units defined for this building yet.');

    fireEvent.click(screen.getByRole('button', { name: 'Add unit' }));

    const dialog = screen.getByRole('dialog', { name: 'Add unit' });
    // Submit with an empty designation.
    fireEvent.click(within(dialog).getByRole('button', { name: 'Add unit' }));

    expect(await within(dialog).findByText('Designation is required.')).toBeInTheDocument();
    // No POST should have been attempted (only the initial GET).
    const posts = mockFetch.mock.calls.filter((c) => c[1]?.method === 'POST');
    expect(posts).toHaveLength(0);
  });

  it('archives a unit only after the confirmation is accepted', async () => {
    mockFetch.mockImplementation((url: string, options?: { method?: string }) => {
      if (options?.method === 'DELETE') return jsonResponse(undefined, true, 204);
      return jsonResponse({ units: [sampleUnit], total: 1, offset: 0, limit: 50 });
    });

    render(<UnitsSection buildingId="b-1" />, { wrapper: createWrapper() });
    await screen.findByText('3B');

    fireEvent.click(screen.getByRole('button', { name: 'Archive' }));

    const dialog = await screen.findByRole('alertdialog');
    expect(within(dialog).getByText(/Archive unit "3B"/)).toBeInTheDocument();

    fireEvent.click(within(dialog).getByRole('button', { name: 'Archive' }));

    await waitFor(() => {
      const deletes = mockFetch.mock.calls.filter((c) => c[1]?.method === 'DELETE');
      expect(deletes).toHaveLength(1);
      expect(deletes[0][0]).toContain('/api/v1/buildings/b-1/units/unit-1');
    });
  });
});
