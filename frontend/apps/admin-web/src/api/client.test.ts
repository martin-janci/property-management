import { describe, expect, it, vi } from 'vitest';

import { createApiClient } from './client';

describe('admin api client', () => {
  it('clears token and triggers onUnauthenticated when server returns 401', async () => {
    const onUnauthenticated = vi.fn();
    const tokenStore = {
      get: () => 'expired',
      set: vi.fn(),
      clear: vi.fn(),
    };
    const client = createApiClient({ baseURL: '/api', tokenStore, onUnauthenticated });

    // Simulate the interceptor's effect directly by calling its rejection
    // handler with an axios-like error.
    const error = { response: { status: 401, data: { error: 'unauthenticated' } } };
    await client.handle401(error).catch(() => {});

    expect(tokenStore.clear).toHaveBeenCalled();
    expect(onUnauthenticated).toHaveBeenCalled();
  });
});
