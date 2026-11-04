/// <reference types="vitest/globals" />
/**
 * The resident dashboard reads announcements from the resident-visible
 * `GET /announcements/published` endpoint, not the manager-only list that
 * returned 403 for owners and tenants (round-3 audit #17).
 */
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { renderHook, waitFor } from '@testing-library/react';
import type { ReactNode } from 'react';
import { useResidentAnnouncements } from './useResidentAnnouncements';

function wrapper({ children }: { children: ReactNode }) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return <QueryClientProvider client={client}>{children}</QueryClientProvider>;
}

afterEach(() => {
  vi.unstubAllGlobals();
});

it('calls /announcements/published and maps the snake_case list', async () => {
  vi.stubGlobal(
    'fetch',
    vi.fn(
      async () =>
        new Response(
          JSON.stringify({
            announcements: [
              {
                id: 'an-1',
                title: 'Water outage',
                status: 'published',
                published_at: '2026-01-02T10:00:00Z',
              },
            ],
            count: 1,
            total: 1,
          }),
          { status: 200 }
        )
    )
  );

  const { result } = renderHook(() => useResidentAnnouncements(5), { wrapper });
  await waitFor(() => expect(result.current.isSuccess).toBe(true));

  const url = new URL(String(vi.mocked(fetch).mock.calls[0][0]), 'http://origin.invalid');
  expect(url.pathname).toBe('/api/v1/announcements/published');
  expect(url.searchParams.has('status')).toBe(false);
  expect(result.current.data?.items[0]).toMatchObject({
    id: 'an-1',
    title: 'Water outage',
    publishedAt: '2026-01-02T10:00:00Z',
  });
});

it('surfaces a failed request as an error', async () => {
  vi.stubGlobal(
    'fetch',
    vi.fn(async () => new Response('{}', { status: 500 }))
  );
  const { result } = renderHook(() => useResidentAnnouncements(5), { wrapper });
  await waitFor(() => expect(result.current.isError).toBe(true));
});
