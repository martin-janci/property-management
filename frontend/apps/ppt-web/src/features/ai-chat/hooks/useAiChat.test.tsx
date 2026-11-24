/**
 * AI Chat Hook Tests
 * Epic 127: AI Chatbot Interface
 *
 * Regression (#2978): `useAiChat`'s hooks previously issued raw `fetch()` calls
 * with no Authorization header, so every AI-chat request (mounted behind
 * <ProtectedRoute>) went out unauthenticated and 401'd in production.
 * Additionally, `useDeleteSession` never checked `response.ok`, so a failed
 * delete resolved as success and silently hid the failure.
 *
 * The hooks now route through the shared axios client (`getApiClient()`), whose
 * request interceptor stamps `Authorization: Bearer <token>` and whose response
 * handling rejects on non-2xx. These tests seed a token provider on the
 * configured client and capture the outbound request with a recording adapter:
 * they fail on the pre-fix raw-fetch version (no bearer header; delete never
 * surfaces the error).
 */
/// <reference types="vitest/globals" />
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { renderHook, waitFor } from '@testing-library/react';
import type { AxiosAdapter, AxiosResponse } from 'axios';
import { AxiosHeaders } from 'axios';
import type { ReactNode } from 'react';
import { configureApiClient, resetApiClient } from '../../../lib/api';
import {
  aiChatKeys,
  useAiChatMessages,
  useAiChatSessions,
  useCreateSession,
  useDeleteSession,
} from './useAiChat';

const ACCESS_TOKEN = 'ai-chat-access-token-abc';

interface RecordedRequest {
  auth: string | undefined;
  url?: string;
  method?: string;
  params: unknown;
  data: unknown;
}

/** Adapter that records each request and returns a canned response. */
function recordingAdapter(
  responseData: unknown,
  status = 200
): {
  adapter: AxiosAdapter;
  requests: () => RecordedRequest[];
} {
  const seen: RecordedRequest[] = [];
  const adapter: AxiosAdapter = (config) => {
    const headers = AxiosHeaders.from(config.headers);
    seen.push({
      auth: headers.get('Authorization') as string | undefined,
      url: config.url,
      method: config.method,
      params: config.params,
      data: config.data,
    });
    const response: AxiosResponse = {
      data: responseData,
      status,
      statusText: status >= 400 ? 'Error' : 'OK',
      headers: {},
      config,
    };
    if (status >= 400) {
      return Promise.reject(
        Object.assign(new Error(`Request failed with status code ${status}`), {
          isAxiosError: true,
          response,
          config,
        })
      );
    }
    return Promise.resolve(response);
  };
  return { adapter, requests: () => seen };
}

function createWrapper() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false, gcTime: 0 },
      mutations: { retry: false },
    },
  });
  return ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );
}

describe('useAiChat — routes through the JWT-injecting axios client', () => {
  beforeEach(() => {
    resetApiClient();
  });

  afterEach(() => {
    resetApiClient();
  });

  it('sends the Authorization header when listing sessions', async () => {
    const instance = configureApiClient({ getToken: () => ACCESS_TOKEN });
    const { adapter, requests } = recordingAdapter({ sessions: [{ id: 's1' }] });
    instance.defaults.adapter = adapter;

    const { result } = renderHook(() => useAiChatSessions(), {
      wrapper: createWrapper(),
    });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    const [req] = requests();
    expect(req.auth).toBe(`Bearer ${ACCESS_TOKEN}`);
    expect(req.url).toBe('/ai/chat/sessions');
  });

  it('sends the Authorization header when creating a session', async () => {
    const instance = configureApiClient({ getToken: () => ACCESS_TOKEN });
    const { adapter, requests } = recordingAdapter({ id: 's-new' });
    instance.defaults.adapter = adapter;

    const { result } = renderHook(() => useCreateSession(), {
      wrapper: createWrapper(),
    });

    result.current.mutate({ title: 'Hello' } as never);

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    const [req] = requests();
    expect(req.auth).toBe(`Bearer ${ACCESS_TOKEN}`);
    expect(req.url).toBe('/ai/chat/sessions');
    expect(req.method).toBe('post');
  });

  describe('useDeleteSession', () => {
    it('sends the Authorization header and hits the delete endpoint', async () => {
      const instance = configureApiClient({ getToken: () => ACCESS_TOKEN });
      const { adapter, requests } = recordingAdapter({}, 204);
      instance.defaults.adapter = adapter;

      const { result } = renderHook(() => useDeleteSession(), {
        wrapper: createWrapper(),
      });

      result.current.mutate('session-1');

      await waitFor(() => expect(result.current.isSuccess).toBe(true));

      const [req] = requests();
      expect(req.auth).toBe(`Bearer ${ACCESS_TOKEN}`);
      expect(req.url).toBe('/ai/chat/sessions/session-1');
      expect(req.method).toBe('delete');
    });

    it('surfaces a failed delete instead of swallowing it', async () => {
      // Pre-fix, the raw `fetch()` result was never checked for `response.ok`,
      // so a failed delete resolved as success. Routing through the axios client
      // rejects on non-2xx. Use 403 (a non-retryable status — DELETE is
      // idempotent and 5xx would be auto-retried with backoff) so the error
      // surfaces immediately.
      const instance = configureApiClient({ getToken: () => ACCESS_TOKEN });
      const { adapter } = recordingAdapter({ message: 'Delete failed' }, 403);
      instance.defaults.adapter = adapter;

      const { result } = renderHook(() => useDeleteSession(), {
        wrapper: createWrapper(),
      });

      result.current.mutate('session-1');

      await waitFor(() => expect(result.current.isError).toBe(true));
      expect(result.current.error).toBeInstanceOf(Error);
    });
  });
});

/**
 * Regression: the paged reads (`useAiChatMessages`, `useAiChatSessions`,
 * `useEscalatedMessages`) accept `limit`/`offset` but previously built their
 * TanStack `queryKey` WITHOUT those inputs (`aiChatKeys.messages(sessionId)` /
 * `aiChatKeys.sessions()`). Two different pages therefore collided on one cache
 * entry: once page 1 was cached (and still fresh under its `staleTime`),
 * requesting page 2 returned page 1's rows and issued no second request.
 *
 * The keys now fold `{ limit, offset }` in (`aiChatKeys.messagesList(...)` /
 * `sessionsList(...)`), giving each page its own cache slot. These tests share
 * ONE QueryClient across both page requests and assert that a second,
 * distinct-offset request is actually issued and returns its own page — they
 * fail on the pre-fix key that omitted the pagination inputs.
 */
describe('useAiChat — paginated query keys include limit/offset', () => {
  beforeEach(() => {
    resetApiClient();
  });

  afterEach(() => {
    resetApiClient();
  });

  /** Adapter whose `{ messages }` page depends on the request's `offset`. */
  function pagedMessagesAdapter(): {
    adapter: AxiosAdapter;
    requests: () => RecordedRequest[];
  } {
    const seen: RecordedRequest[] = [];
    const adapter: AxiosAdapter = (config) => {
      const headers = AxiosHeaders.from(config.headers);
      const params = (config.params ?? {}) as { limit?: number; offset?: number };
      seen.push({
        auth: headers.get('Authorization') as string | undefined,
        url: config.url,
        method: config.method,
        params: config.params,
        data: config.data,
      });
      const offset = Number(params.offset ?? 0);
      const messages = offset === 0 ? [{ id: 'm-page0' }] : [{ id: 'm-page1' }];
      return Promise.resolve({
        data: { messages },
        status: 200,
        statusText: 'OK',
        headers: {},
        config,
      } as AxiosResponse);
    };
    return { adapter, requests: () => seen };
  }

  function sharedClientWrapper() {
    const queryClient = new QueryClient({
      defaultOptions: {
        // No retry, but keep the default (non-zero) staleTime behaviour of the
        // hooks: a fresh cache hit must NOT refetch. gcTime high so the first
        // page stays cached while the second page mounts.
        queries: { retry: false },
        mutations: { retry: false },
      },
    });
    const wrapper = ({ children }: { children: ReactNode }) => (
      <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
    );
    return { wrapper };
  }

  it('serves a different message page instead of the first page cache', async () => {
    const instance = configureApiClient({ getToken: () => ACCESS_TOKEN });
    const { adapter, requests } = pagedMessagesAdapter();
    instance.defaults.adapter = adapter;

    const { wrapper } = sharedClientWrapper();

    const first = renderHook(() => useAiChatMessages('s1', 1, 0), { wrapper });
    await waitFor(() => expect(first.result.current.isSuccess).toBe(true));
    expect(first.result.current.data).toEqual([{ id: 'm-page0' }]);

    const second = renderHook(() => useAiChatMessages('s1', 1, 1), { wrapper });
    await waitFor(() => expect(second.result.current.isSuccess).toBe(true));
    // Pre-fix this returned page 0's cached rows; now each page has its own key.
    expect(second.result.current.data).toEqual([{ id: 'm-page1' }]);

    // A second, distinct request was actually issued (pre-fix: only one).
    expect(requests().map((r) => r.params)).toEqual([
      { limit: 1, offset: 0 },
      { limit: 1, offset: 1 },
    ]);
  });

  it('keys message pages under the invalidation scope prefix', () => {
    // The scope key stays a strict prefix of each paged key, so invalidating
    // `messages(sessionId)` still prefix-matches every page.
    const scope = aiChatKeys.messages('s1');
    const page = aiChatKeys.messagesList('s1', 25, 50);
    expect(page.slice(0, scope.length)).toEqual([...scope]);
    expect(page[page.length - 1]).toEqual({ limit: 25, offset: 50 });

    const sessionsScope = aiChatKeys.sessions();
    const sessionsPage = aiChatKeys.sessionsList(10, 20);
    expect(sessionsPage.slice(0, sessionsScope.length)).toEqual([...sessionsScope]);
    expect(sessionsPage[sessionsPage.length - 1]).toEqual({ limit: 10, offset: 20 });
  });
});
