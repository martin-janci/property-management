/// <reference types="vitest/globals" />
/**
 * Integration coverage for the ppt-web API client (Story 79.1) driven through
 * TanStack Query (Coverage 79-1).
 *
 * The axios instance in `lib/api.ts` owns three behaviours that the rest of the
 * app relies on but that were previously untested end-to-end:
 *
 *   1. Transient-failure retry with exponential backoff (network error + 5xx),
 *      capped at MAX_RETRIES, surfacing the transformed `ApiError` once the
 *      budget is exhausted.
 *   2. The 401 -> refresh -> retry path: a 401 fires `onUnauthorized` (where
 *      AuthContext refreshes the token); the *request* interceptor then injects
 *      the freshly-minted token on the retried call so the second attempt
 *      succeeds. This mirrors how `useQuery` re-runs after a refresh.
 *   3. 5xx surfacing: once retries are spent the error reaches the TanStack
 *      Query layer as a structured `ApiError` (status + code + isRetryable),
 *      not a bare axios error.
 *
 * Rather than hit a real socket, each test installs a deterministic custom
 * axios `adapter` on the configured instance. The adapter still runs *through*
 * the real request/response interceptors (token injection, error transform,
 * retry loop, 401 callback), so this is genuine integration coverage of
 * `api.ts` — only the wire is faked. The backoff `setTimeout` is stubbed to
 * fire on the next tick so the suite stays sub-second.
 *
 * Additive-only: no production code is touched.
 */

import { QueryClient, QueryClientProvider, useQuery } from '@tanstack/react-query';
import { renderHook } from '@testing-library/react';
import type { AxiosAdapter, AxiosInstance, AxiosResponse } from 'axios';
import { AxiosError, AxiosHeaders } from 'axios';
import type { ReactNode } from 'react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { type ApiError, configureApiClient, getApiClient, isApiError, resetApiClient } from './api';

// ---------------------------------------------------------------------------
// Adapter harness
// ---------------------------------------------------------------------------

/** One scripted outcome the fake adapter will play back for a single attempt. */
type ScriptedStep =
  | { kind: 'ok'; status?: number; data?: unknown }
  | { kind: 'network' }
  | { kind: 'status'; status: number; data?: unknown };

/**
 * Build a deterministic adapter that plays back `script` step-by-step (one step
 * per attempt) and records the Authorization header it saw on each attempt.
 * After the script is exhausted it repeats the final step.
 */
function scriptedAdapter(script: ScriptedStep[]): {
  adapter: AxiosAdapter;
  attempts: () => number;
  authHeaders: () => Array<string | undefined>;
} {
  let attempt = 0;
  const seenAuth: Array<string | undefined> = [];

  const adapter: AxiosAdapter = (config) => {
    const step = script[Math.min(attempt, script.length - 1)];
    attempt += 1;

    const headers = AxiosHeaders.from(config.headers);
    seenAuth.push(headers.get('Authorization') as string | undefined);

    if (step.kind === 'ok') {
      const response: AxiosResponse = {
        data: step.data ?? { ok: true },
        status: step.status ?? 200,
        statusText: 'OK',
        headers: {},
        config,
      };
      return Promise.resolve(response);
    }

    if (step.kind === 'network') {
      // Axios represents a network error as a rejected error with no response.
      const err = new AxiosError('Network Error', AxiosError.ERR_NETWORK, config);
      return Promise.reject(err);
    }

    // step.kind === 'status'
    const response: AxiosResponse = {
      data: step.data ?? { error: 'ERR', message: `status ${step.status}` },
      status: step.status,
      statusText: 'ERR',
      headers: {},
      config,
    };
    const err = new AxiosError(
      `Request failed with status code ${step.status}`,
      step.status >= 500 ? AxiosError.ERR_BAD_RESPONSE : AxiosError.ERR_BAD_REQUEST,
      config,
      {},
      response
    );
    return Promise.reject(err);
  };

  return {
    adapter,
    attempts: () => attempt,
    authHeaders: () => seenAuth,
  };
}

/** Force the configured instance to use our scripted adapter. */
function installAdapter(instance: AxiosInstance, adapter: AxiosAdapter): void {
  instance.defaults.adapter = adapter;
}

function createWrapper() {
  const queryClient = new QueryClient({
    // TanStack Query retry is OFF — the retry logic under test lives in the
    // axios interceptor, so we must not double-retry at the query layer.
    defaultOptions: {
      queries: { retry: false, gcTime: 0 },
    },
  });
  const Wrapper = ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );
  return Wrapper;
}

/**
 * Drive a TanStack Query to a terminal state under Vitest fake timers.
 *
 * The interceptor's exponential backoff (`setTimeout(resolve, delay)`, up to
 * ~7s + jitter across the budget) is far too slow for a unit test, so we run
 * with fake timers and fast-forward time. testing-library's `waitFor` polls on
 * a real-timer interval and deadlocks against fake timers, so instead we hand-
 * roll the wait: repeatedly flush pending microtasks + advance the fake clock
 * until `predicate()` holds (or we give up after a bounded number of turns).
 */
async function advanceUntil(predicate: () => boolean): Promise<void> {
  for (let i = 0; i < 50; i++) {
    if (predicate()) return;
    // Let queued promise jobs (axios adapter, interceptors) run, then push the
    // fake clock past the backoff sleep so the next retry fires.
    await vi.advanceTimersByTimeAsync(10_000);
  }
  if (!predicate()) {
    throw new Error('advanceUntil: predicate never became true within budget');
  }
}

beforeEach(() => {
  resetApiClient();
  vi.useFakeTimers();
});

afterEach(() => {
  vi.useRealTimers();
  resetApiClient();
});

// ---------------------------------------------------------------------------
// 1. Network-error retry
// ---------------------------------------------------------------------------

describe('api client — network error retry (through useQuery)', () => {
  it('retries a network error and succeeds on a later attempt', async () => {
    const instance = configureApiClient({});
    const { adapter, attempts } = scriptedAdapter([
      { kind: 'network' },
      { kind: 'network' },
      { kind: 'ok', data: { value: 42 } },
    ]);
    installAdapter(instance, adapter);

    const { result } = renderHook(
      () =>
        useQuery({
          queryKey: ['net-retry'],
          queryFn: async () => {
            const res = await getApiClient().get('/widgets');
            return res.data as { value: number };
          },
        }),
      { wrapper: createWrapper() }
    );

    await advanceUntil(() => result.current.isSuccess);

    expect(result.current.data).toEqual({ value: 42 });
    // 2 network failures + 1 success.
    expect(attempts()).toBe(3);
  });

  it('gives up after MAX_RETRIES and surfaces a transformed ApiError', async () => {
    const instance = configureApiClient({});
    // Always a network error — exhaust the retry budget (1 initial + 3 retries).
    const { adapter, attempts } = scriptedAdapter([{ kind: 'network' }]);
    installAdapter(instance, adapter);

    const { result } = renderHook(
      () =>
        useQuery({
          queryKey: ['net-exhaust'],
          queryFn: async () => {
            const res = await getApiClient().get('/widgets');
            return res.data;
          },
        }),
      { wrapper: createWrapper() }
    );

    await advanceUntil(() => result.current.isError);

    const error = result.current.error;
    expect(isApiError(error)).toBe(true);
    const apiError = error as ApiError;
    // Network errors have no HTTP status.
    expect(apiError.status).toBe(0);
    expect(apiError.isRetryable).toBe(true);
    // 1 initial attempt + MAX_RETRIES (3) = 4 total.
    expect(attempts()).toBe(4);
  });
});

// ---------------------------------------------------------------------------
// 2. 401 -> refresh -> retry
// ---------------------------------------------------------------------------

describe('api client — 401 triggers onUnauthorized (refresh hook)', () => {
  it('invokes the onUnauthorized callback exactly once per 401', async () => {
    const onUnauthorized = vi.fn();
    const instance = configureApiClient({ onUnauthorized });
    // 401 is NOT in the retryable list, so it surfaces immediately after the
    // callback fires — no retry loop.
    const { adapter, attempts } = scriptedAdapter([
      { kind: 'status', status: 401, data: { error: 'UNAUTHORIZED', message: 'expired' } },
    ]);
    installAdapter(instance, adapter);

    const { result } = renderHook(
      () =>
        useQuery({
          queryKey: ['unauth'],
          queryFn: async () => {
            const res = await getApiClient().get('/me');
            return res.data;
          },
        }),
      { wrapper: createWrapper() }
    );

    await advanceUntil(() => result.current.isError);

    expect(onUnauthorized).toHaveBeenCalledTimes(1);
    expect(attempts()).toBe(1);
    const apiError = result.current.error as ApiError;
    expect(apiError.status).toBe(401);
    expect(apiError.code).toBe('UNAUTHORIZED');
  });

  it('injects the refreshed token on a manual re-run after refresh (401 -> refresh -> retry)', async () => {
    // Simulate AuthContext: the first token is stale; onUnauthorized refreshes
    // it. The request interceptor reads the current token on every attempt.
    let currentToken = 'stale-token';
    const onUnauthorized = vi.fn(() => {
      currentToken = 'fresh-token';
    });
    const instance = configureApiClient({
      getToken: () => currentToken,
      onUnauthorized,
    });

    // Attempt 1: 401 (token stale). Attempt 2 (query re-run): 200.
    const { adapter, attempts, authHeaders } = scriptedAdapter([
      { kind: 'status', status: 401, data: { error: 'UNAUTHORIZED', message: 'expired' } },
      { kind: 'ok', data: { user: 'me' } },
    ]);
    installAdapter(instance, adapter);

    const wrapper = createWrapper();

    // First call fails with 401 and triggers refresh.
    await expect(getApiClient().get('/me')).rejects.toMatchObject({ status: 401 });
    expect(onUnauthorized).toHaveBeenCalledTimes(1);
    expect(currentToken).toBe('fresh-token');

    // The query layer re-runs after the refresh — this is the retried call.
    const { result } = renderHook(
      () =>
        useQuery({
          queryKey: ['after-refresh'],
          queryFn: async () => {
            const res = await getApiClient().get('/me');
            return res.data as { user: string };
          },
        }),
      { wrapper }
    );

    await advanceUntil(() => result.current.isSuccess);

    expect(result.current.data).toEqual({ user: 'me' });
    expect(attempts()).toBe(2);
    // Attempt 1 carried the stale token; attempt 2 carried the refreshed token.
    expect(authHeaders()[0]).toBe('Bearer stale-token');
    expect(authHeaders()[1]).toBe('Bearer fresh-token');
  });
});

// ---------------------------------------------------------------------------
// 3. 5xx surfacing
// ---------------------------------------------------------------------------

describe('api client — 5xx surfacing through useQuery', () => {
  it('retries a 503 and surfaces a structured ApiError after the budget is spent', async () => {
    const instance = configureApiClient({});
    const { adapter, attempts } = scriptedAdapter([
      { kind: 'status', status: 503, data: { error: 'SERVICE_UNAVAILABLE', message: 'down' } },
    ]);
    installAdapter(instance, adapter);

    const { result } = renderHook(
      () =>
        useQuery({
          queryKey: ['five-xx'],
          queryFn: async () => {
            const res = await getApiClient().get('/widgets');
            return res.data;
          },
        }),
      { wrapper: createWrapper() }
    );

    await advanceUntil(() => result.current.isError);

    const apiError = result.current.error as ApiError;
    expect(isApiError(apiError)).toBe(true);
    expect(apiError.status).toBe(503);
    expect(apiError.code).toBe('SERVICE_UNAVAILABLE');
    expect(apiError.message).toBe('down');
    expect(apiError.isRetryable).toBe(true);
    // 1 initial + 3 retries.
    expect(attempts()).toBe(4);
  });

  it('recovers when a 500 clears before the retry budget is spent', async () => {
    const instance = configureApiClient({});
    const { adapter, attempts } = scriptedAdapter([
      { kind: 'status', status: 500, data: { error: 'INTERNAL', message: 'boom' } },
      { kind: 'ok', data: { recovered: true } },
    ]);
    installAdapter(instance, adapter);

    const { result } = renderHook(
      () =>
        useQuery({
          queryKey: ['five-xx-recover'],
          queryFn: async () => {
            const res = await getApiClient().get('/widgets');
            return res.data as { recovered: boolean };
          },
        }),
      { wrapper: createWrapper() }
    );

    await advanceUntil(() => result.current.isSuccess);

    expect(result.current.data).toEqual({ recovered: true });
    expect(attempts()).toBe(2);
  });

  it('does NOT retry a non-retryable 4xx and surfaces it immediately', async () => {
    const instance = configureApiClient({});
    const { adapter, attempts } = scriptedAdapter([
      { kind: 'status', status: 400, data: { error: 'BAD_REQUEST', message: 'invalid' } },
    ]);
    installAdapter(instance, adapter);

    const { result } = renderHook(
      () =>
        useQuery({
          queryKey: ['four-xx'],
          queryFn: async () => {
            const res = await getApiClient().get('/widgets');
            return res.data;
          },
        }),
      { wrapper: createWrapper() }
    );

    await advanceUntil(() => result.current.isError);

    const apiError = result.current.error as ApiError;
    expect(apiError.status).toBe(400);
    expect(apiError.isRetryable).toBe(false);
    // No retries for a 400.
    expect(attempts()).toBe(1);
  });
});
