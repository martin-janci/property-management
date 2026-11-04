/// <reference types="vitest/globals" />
/**
 * The new-conversation recipient picker is fed by the building's neighbors
 * (round-3 audit #18). The api-server sends them in snake_case; before the
 * client normalized them every neighbor was filtered out as hidden and the
 * picker never offered anyone.
 */
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { renderHook, waitFor } from '@testing-library/react';
import type { ReactNode } from 'react';
import { useMessageRecipients } from './useMessaging';

function wrapper({ children }: { children: ReactNode }) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return <QueryClientProvider client={client}>{children}</QueryClientProvider>;
}

beforeEach(() => {
  vi.stubGlobal(
    'fetch',
    vi.fn(
      async () =>
        new Response(
          JSON.stringify({
            neighbors: [
              {
                user_id: 'u1',
                display_name: 'Jana Nováková',
                unit_label: 'A-12',
                is_visible: true,
                email: 'jana@example.com',
                phone: null,
                resident_type: 'owner',
              },
              {
                user_id: 'u2',
                display_name: 'Resident',
                unit_label: 'A-13',
                is_visible: false,
                email: null,
                phone: null,
                resident_type: 'tenant',
              },
            ],
            count: 2,
            total: 2,
          }),
          { status: 200 }
        )
    )
  );
});

afterEach(() => {
  vi.unstubAllGlobals();
});

it('offers the visible neighbors of the building as recipients', async () => {
  const { result } = renderHook(() => useMessageRecipients('b1'), { wrapper });
  await waitFor(() => expect(result.current.recipients).toHaveLength(1));
  expect(result.current.recipients[0]).toEqual({
    id: 'u1',
    name: 'Jana Nováková',
    email: 'jana@example.com',
  });
  const url = String(vi.mocked(fetch).mock.calls[0]?.[0]);
  expect(url).toContain('/api/v1/buildings/b1/neighbors');
});

it('makes no request without a building', () => {
  renderHook(() => useMessageRecipients(undefined), { wrapper });
  expect(fetch).not.toHaveBeenCalled();
});
