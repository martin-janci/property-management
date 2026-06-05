/**
 * Acknowledgment hook-layer tests (gap-6-2 — Story 6.2 web viewing & acknowledgment).
 *
 * Covers the three data-layer hooks the ViewAnnouncementPage wires the
 * "Mark as Read" button, the "Acknowledge" button, and the manager stats panel
 * to. The presentational behaviour is already covered by
 * `ViewAnnouncementPage.test.tsx` / `AcknowledgmentStats.test.tsx`; the gap was
 * the hooks themselves — specifically:
 *
 *  - the request contract (endpoint + HTTP method) for read / acknowledge /
 *    acknowledgment-stats, and
 *  - the TanStack cache-invalidation contract on success. The Story 6.2 fix made
 *    `useMarkReadAnnouncement` invalidate the *list* cache (not just detail +
 *    unread-count) so unread badges update immediately; `useAcknowledgeAnnouncement`
 *    must refresh both the detail and the per-announcement acknowledgments cache.
 *    Those invalidation sets are exactly the kind of thing a refactor can silently
 *    drop, so they are pinned here.
 */

/// <reference types="vitest/globals" />
import {
  announcementKeys,
  useAcknowledgeAnnouncement,
  useAnnouncementAcknowledgmentStats,
  useMarkReadAnnouncement,
} from '@ppt/api-client';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { renderHook, waitFor } from '@testing-library/react';
import type { ReactNode } from 'react';

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

function ok204(): Response {
  return {
    ok: true,
    status: 204,
    json: async () => {
      throw new Error('no body');
    },
  } as unknown as Response;
}

beforeEach(() => {
  vi.clearAllMocks();
});

afterEach(() => {
  vi.resetAllMocks();
});

describe('useMarkReadAnnouncement', () => {
  it('POSTs to /api/v1/announcements/:id/read', async () => {
    mockFetch.mockResolvedValueOnce(ok204());
    const { wrapper } = setup();

    const { result } = renderHook(() => useMarkReadAnnouncement(), { wrapper });
    result.current.mutate('ann-1');

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(mockFetch).toHaveBeenCalledWith(
      '/api/v1/announcements/ann-1/read',
      expect.objectContaining({ method: 'POST' })
    );
  });

  it('invalidates detail, unread-count AND the list cache so unread badges refresh', async () => {
    mockFetch.mockResolvedValueOnce(ok204());
    const { invalidateSpy, wrapper } = setup();

    const { result } = renderHook(() => useMarkReadAnnouncement(), { wrapper });
    result.current.mutate('ann-1');

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    const keys = invalidatedKeys(invalidateSpy);
    expect(keys).toContain(JSON.stringify(announcementKeys.detail('ann-1')));
    expect(keys).toContain(JSON.stringify(announcementKeys.unreadCount()));
    // Story 6.2 fix: list cache must be invalidated so unread badges update.
    expect(keys).toContain(JSON.stringify(announcementKeys.lists()));
  });

  it('surfaces a server error message', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: false,
      status: 403,
      json: async () => ({ message: 'Not in audience' }),
    } as Response);
    const { wrapper } = setup();

    const { result } = renderHook(() => useMarkReadAnnouncement(), { wrapper });
    result.current.mutate('ann-1');

    await waitFor(() => expect(result.current.isError).toBe(true));
    expect(result.current.error?.message).toBe('Not in audience');
  });
});

describe('useAcknowledgeAnnouncement', () => {
  it('POSTs to /api/v1/announcements/:id/acknowledge', async () => {
    mockFetch.mockResolvedValueOnce(ok204());
    const { wrapper } = setup();

    const { result } = renderHook(() => useAcknowledgeAnnouncement(), { wrapper });
    result.current.mutate('ann-9');

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(mockFetch).toHaveBeenCalledWith(
      '/api/v1/announcements/ann-9/acknowledge',
      expect.objectContaining({ method: 'POST' })
    );
  });

  it('invalidates the detail and the per-announcement acknowledgments cache', async () => {
    mockFetch.mockResolvedValueOnce(ok204());
    const { invalidateSpy, wrapper } = setup();

    const { result } = renderHook(() => useAcknowledgeAnnouncement(), { wrapper });
    result.current.mutate('ann-9');

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    const keys = invalidatedKeys(invalidateSpy);
    expect(keys).toContain(JSON.stringify(announcementKeys.detail('ann-9')));
    expect(keys).toContain(JSON.stringify(announcementKeys.acknowledgments('ann-9')));
  });
});

describe('useAnnouncementAcknowledgmentStats', () => {
  it('GETs /api/v1/announcements/:id/acknowledgments and returns the stats payload', async () => {
    const payload = {
      stats: {
        announcementId: 'ann-2',
        totalTargeted: 10,
        readCount: 6,
        acknowledgedCount: 2,
        pendingCount: 4,
      },
    };
    mockFetch.mockResolvedValueOnce({
      ok: true,
      status: 200,
      json: async () => payload,
    } as Response);
    const { wrapper } = setup();

    const { result } = renderHook(() => useAnnouncementAcknowledgmentStats('ann-2'), { wrapper });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(result.current.data).toEqual(payload);
    const [url, init] = mockFetch.mock.calls[0];
    expect(url).toBe('/api/v1/announcements/ann-2/acknowledgments');
    // No method override -> defaults to GET.
    expect((init as RequestInit | undefined)?.method).toBeUndefined();
  });

  it('stays disabled (no request) until an id is supplied', () => {
    const { wrapper } = setup();

    const { result } = renderHook(() => useAnnouncementAcknowledgmentStats(''), { wrapper });

    expect(result.current.fetchStatus).toBe('idle');
    expect(mockFetch).not.toHaveBeenCalled();
  });

  it('respects an explicit enabled=false flag', () => {
    const { wrapper } = setup();

    const { result } = renderHook(() => useAnnouncementAcknowledgmentStats('ann-2', false), {
      wrapper,
    });

    expect(result.current.fetchStatus).toBe('idle');
    expect(mockFetch).not.toHaveBeenCalled();
  });
});
