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
import { useAiChatSessions, useCreateSession, useDeleteSession } from './useAiChat';

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
