/**
 * Regression (code-review-ppt-web-core-raw-fetch-bypasses-jwt-interceptor):
 * `useBuildingUnits` must route through the shared axios client so the JWT
 * request interceptor stamps the Authorization header.
 *
 * Before the fix the hook issued a raw `fetch('/api/v1/units?…')`, which never
 * touched the axios interceptor in lib/api.ts — so the request went out with NO
 * bearer token and 401'd for real signed-in users, silently leaving the
 * building-units dropdown (that feeds person-months) empty. This test seeds a
 * token provider on the configured client, captures the outbound request via a
 * recording adapter, and asserts it carried `Authorization: Bearer <token>` and
 * hit `/units` with the building filter. It fails on the raw-fetch version
 * (no header) and passes once the hook uses getApiClient().
 */
/// <reference types="vitest/globals" />

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { renderHook, waitFor } from '@testing-library/react';
import type { AxiosAdapter, AxiosResponse } from 'axios';
import { AxiosHeaders } from 'axios';
import type React from 'react';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { configureApiClient, resetApiClient } from '../../lib/api';
import { useBuildingUnits } from './person-months';

const ACCESS_TOKEN = 'access-token-abc123';

/** Adapter that records each request and returns a canned units payload. */
function recordingAdapter(): {
  adapter: AxiosAdapter;
  requests: () => Array<{ auth: string | undefined; url?: string; params: unknown }>;
} {
  const seen: Array<{ auth: string | undefined; url?: string; params: unknown }> = [];
  const adapter: AxiosAdapter = (config) => {
    const headers = AxiosHeaders.from(config.headers);
    seen.push({
      auth: headers.get('Authorization') as string | undefined,
      url: config.url,
      params: config.params,
    });
    const response: AxiosResponse = {
      data: { data: [{ id: 'unit-1', unitNumber: 'A/1' }] },
      status: 200,
      statusText: 'OK',
      headers: {},
      config,
    };
    return Promise.resolve(response);
  };
  return { adapter, requests: () => seen };
}

function wrapper(queryClient: QueryClient) {
  return function Wrapper({ children }: { children: React.ReactNode }) {
    return <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>;
  };
}

describe('useBuildingUnits — routes through the JWT-injecting axios client', () => {
  beforeEach(() => {
    resetApiClient();
  });

  afterEach(() => {
    resetApiClient();
  });

  it('sends the Authorization header and the buildingId filter on /units', async () => {
    // Configure the shared client exactly as AuthProvider does at runtime.
    const instance = configureApiClient({ getToken: () => ACCESS_TOKEN });
    const { adapter, requests } = recordingAdapter();
    instance.defaults.adapter = adapter;

    const queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false, gcTime: 0 } },
    });

    const { result } = renderHook(() => useBuildingUnits('building-1'), {
      wrapper: wrapper(queryClient),
    });

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    // The hook mapped the wire payload to the UI shape.
    expect(result.current.units).toEqual([{ id: 'unit-1', designation: 'A/1' }]);

    // Core assertions: the request carried the bearer token (proving it went
    // through the axios interceptor, not a raw fetch) and targeted /units for
    // the requested building.
    const reqs = requests();
    expect(reqs).toHaveLength(1);
    expect(reqs[0].auth).toBe(`Bearer ${ACCESS_TOKEN}`);
    expect(reqs[0].url).toBe('/units');
    expect(reqs[0].params).toMatchObject({ buildingId: 'building-1', limit: 500 });
  });
});
