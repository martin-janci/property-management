/// <reference types="vitest/globals" />
/**
 * Realtime read-state propagation tests for direct messaging
 * (Coverage 6-5 — Epic 6, Story 6.5).
 *
 * The realtime UX for direct messaging hinges on the TanStack cache being
 * invalidated at exactly the right query keys when the user reads, sends, or
 * starts a thread — otherwise the thread list, the open thread, and the global
 * unread badge drift apart and the "live" feel breaks. Those invalidation sets
 * live in the shared messaging hooks factory but are surfaced through the
 * ppt-web `useMessaging` wrappers, so we exercise the wrappers here:
 *
 *  - `useMarkThreadRead` — the read-state mutation. MUST invalidate the thread
 *    detail, the thread list, AND the global unread count so every surface that
 *    shows "unread" updates the moment a thread is opened/read. A refactor that
 *    drops the unread-count invalidation leaves a stale badge; that regression
 *    is pinned here.
 *  - `useSendMessage` — sending a message refreshes the open thread and the
 *    list preview/order.
 *  - `useStartThread` — a new thread must appear in the list immediately.
 *
 * The presentational behaviour is covered elsewhere; the gap was the
 * cache-sync contract that makes the realtime sync observable in the UI.
 */

import { messagingKeys } from '@ppt/api-client';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { renderHook, waitFor } from '@testing-library/react';
import type { ReactNode } from 'react';
import { useMarkThreadRead, useSendMessage, useStartThread } from './useMessaging';

const mockFetch = vi.fn();
global.fetch = mockFetch;

function setup() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });
  const invalidateSpy = vi.spyOn(queryClient, 'invalidateQueries');
  const wrapper = ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );
  return { queryClient, invalidateSpy, wrapper };
}

/** Collect every queryKey passed to invalidateQueries as a JSON string for set comparison. */
function invalidatedKeys(invalidateSpy: ReturnType<typeof vi.spyOn>): string[] {
  return invalidateSpy.mock.calls.map((call) =>
    JSON.stringify((call[0] as { queryKey: unknown })?.queryKey)
  );
}

/** A generic 2xx JSON response — the mutations only need `response.ok`. */
function okJson(body: unknown = {}): Response {
  return {
    ok: true,
    status: 200,
    json: async () => body,
  } as unknown as Response;
}

beforeEach(() => {
  vi.clearAllMocks();
});

afterEach(() => {
  vi.resetAllMocks();
});

describe('useMarkThreadRead (read-state propagation)', () => {
  it('POSTs to /api/v1/messages/threads/:id/read', async () => {
    mockFetch.mockResolvedValueOnce(okJson({ success: true }));
    const { wrapper } = setup();

    const { result } = renderHook(() => useMarkThreadRead(), { wrapper });
    result.current.mutate('thread-1');

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(mockFetch).toHaveBeenCalledWith(
      '/api/v1/messages/threads/thread-1/read',
      expect.objectContaining({ method: 'POST' })
    );
  });

  it('invalidates thread detail, the thread list AND the unread count', async () => {
    mockFetch.mockResolvedValueOnce(okJson({ success: true }));
    const { invalidateSpy, wrapper } = setup();

    const { result } = renderHook(() => useMarkThreadRead(), { wrapper });
    result.current.mutate('thread-1');

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    const keys = invalidatedKeys(invalidateSpy);
    // detail of the read thread refreshes (messages flip to read)
    expect(keys).toContain(JSON.stringify(messagingKeys.threadDetail('thread-1')));
    // the list refreshes so the per-thread unread badge clears
    expect(keys).toContain(JSON.stringify(messagingKeys.threads()));
    // the GLOBAL unread badge must refresh too — dropping this leaves a stale count
    expect(keys).toContain(JSON.stringify(messagingKeys.unreadCount()));
  });
});

describe('useSendMessage (delivery → cache sync)', () => {
  it('POSTs to the thread messages endpoint', async () => {
    mockFetch.mockResolvedValueOnce(okJson({ id: 'm-1' }));
    const { wrapper } = setup();

    const { result } = renderHook(() => useSendMessage(), { wrapper });
    result.current.mutate({ threadId: 'thread-7', data: { content: 'hello' } });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(mockFetch).toHaveBeenCalledWith(
      '/api/v1/messages/threads/thread-7/messages',
      expect.objectContaining({ method: 'POST' })
    );
  });

  it('invalidates the open thread and the thread list so the new message shows', async () => {
    mockFetch.mockResolvedValueOnce(okJson({ id: 'm-1' }));
    const { invalidateSpy, wrapper } = setup();

    const { result } = renderHook(() => useSendMessage(), { wrapper });
    result.current.mutate({ threadId: 'thread-7', data: { content: 'hello' } });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    const keys = invalidatedKeys(invalidateSpy);
    expect(keys).toContain(JSON.stringify(messagingKeys.threadDetail('thread-7')));
    expect(keys).toContain(JSON.stringify(messagingKeys.threads()));
  });
});

describe('useStartThread (new thread → cache sync)', () => {
  it('invalidates the thread list so a freshly started thread appears', async () => {
    mockFetch.mockResolvedValueOnce(okJson({ thread: { id: 'thread-new' } }));
    const { invalidateSpy, wrapper } = setup();

    const { result } = renderHook(() => useStartThread(), { wrapper });
    result.current.mutate({ recipient_ids: ['user-2'], initial_message: 'hi there' });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    const keys = invalidatedKeys(invalidateSpy);
    expect(keys).toContain(JSON.stringify(messagingKeys.threads()));
  });
});
