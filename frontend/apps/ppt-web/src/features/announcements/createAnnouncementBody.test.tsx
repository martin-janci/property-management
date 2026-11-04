/// <reference types="vitest/globals" />
/**
 * The /announcements/new route posts through the standalone
 * `useCreateAnnouncement` hook. The body must be snake_case — the api-server
 * rejected the camelCase `targetType` with 422 `missing field target_type`
 * (round-3 audit #8).
 */
import { useCreateAnnouncement, useUpdateAnnouncement } from '@ppt/api-client';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { act, renderHook } from '@testing-library/react';
import type { ReactNode } from 'react';

function wrapper({ children }: { children: ReactNode }) {
  const client = new QueryClient({ defaultOptions: { mutations: { retry: false } } });
  return <QueryClientProvider client={client}>{children}</QueryClientProvider>;
}

function lastBody(): Record<string, unknown> {
  const calls = vi.mocked(fetch).mock.calls;
  const init = calls[calls.length - 1]?.[1] as RequestInit | undefined;
  return JSON.parse(String(init?.body ?? '{}'));
}

beforeEach(() => {
  vi.stubGlobal(
    'fetch',
    vi.fn(async () => new Response(JSON.stringify({ id: 'a1', message: 'ok' }), { status: 201 }))
  );
});

afterEach(() => {
  vi.unstubAllGlobals();
});

it('useCreateAnnouncement sends snake_case fields', async () => {
  const { result } = renderHook(() => useCreateAnnouncement(), { wrapper });
  await act(async () => {
    await result.current.mutateAsync({
      title: 'Water shutoff',
      content: 'Tomorrow',
      targetType: 'all',
      commentsEnabled: true,
      acknowledgmentRequired: false,
    });
  });
  expect(lastBody()).toEqual({
    title: 'Water shutoff',
    content: 'Tomorrow',
    target_type: 'all',
    comments_enabled: true,
    acknowledgment_required: false,
  });
});

it('useUpdateAnnouncement sends snake_case fields', async () => {
  const { result } = renderHook(() => useUpdateAnnouncement('a1'), { wrapper });
  await act(async () => {
    await result.current.mutateAsync({ targetType: 'building', targetIds: ['b1'] });
  });
  expect(lastBody()).toEqual({ target_type: 'building', target_ids: ['b1'] });
});
