/**
 * useNotificationAnalytics — JWT interceptor regression (#2982).
 *
 * Before the fix the hook hand-rolled a raw `fetch()` with a bearer token read
 * directly from localStorage, bypassing the shared axios interceptor stack in
 * lib/api.ts — so it missed single-flight 401 refresh/replay, ErrorResponse →
 * ApiError transformation, and transient-failure retry, and its auth depended
 * on a duplicated localStorage read rather than the single source of truth
 * (getApiClient()'s configured token getter).
 *
 * The hook now routes through `getApiClient()`, whose request interceptor
 * stamps `Authorization: Bearer <token>`. This test seeds a token provider on
 * the configured client and captures the outbound request with a recording
 * adapter: it asserts the request carried the bearer token and hit
 * `/admin/notifications/analytics` with the window/channel query params. It
 * fails on the pre-fix raw-fetch version (which read the token from
 * localStorage, not the configured getter) and passes after the reroute.
 */
/// <reference types="vitest/globals" />
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { renderHook, waitFor } from '@testing-library/react';
import type { AxiosAdapter, AxiosResponse } from 'axios';
import { AxiosHeaders } from 'axios';
import type { ReactNode } from 'react';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { configureApiClient, resetApiClient } from '../../../lib/api';
import { useNotificationAnalytics } from './useNotificationAnalytics';

const ACCESS_TOKEN = 'analytics-access-token-xyz';

interface RecordedRequest {
  auth: string | undefined;
  url?: string;
  method?: string;
  params: unknown;
}

function recordingAdapter(responseData: unknown, status = 200) {
  const seen: RecordedRequest[] = [];
  const adapter: AxiosAdapter = (config) => {
    const headers = AxiosHeaders.from(config.headers);
    seen.push({
      auth: headers.get('Authorization') as string | undefined,
      url: config.url,
      method: config.method,
      params: config.params,
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
    defaultOptions: { queries: { retry: false, gcTime: 0 } },
  });
  return ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );
}

describe('useNotificationAnalytics — routes through the JWT-injecting axios client (#2982)', () => {
  beforeEach(() => {
    resetApiClient();
  });

  afterEach(() => {
    resetApiClient();
  });

  it('sends the Authorization header and window/channel filters', async () => {
    const instance = configureApiClient({ getToken: () => ACCESS_TOKEN });
    const { adapter, requests } = recordingAdapter({ channels: [], totals: {} });
    instance.defaults.adapter = adapter;

    const { result } = renderHook(
      () => useNotificationAnalytics({ window: '24h', channel: 'email' }),
      { wrapper: createWrapper() }
    );

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    const [req] = requests();
    expect(req.auth).toBe(`Bearer ${ACCESS_TOKEN}`);
    expect(req.url).toBe('/admin/notifications/analytics');
    expect(req.method).toBe('get');
    expect(req.params).toMatchObject({ after: '24h', channel: 'email' });
  });
});
