/**
 * Tests for the centralized auth request-interceptor (#1522).
 */

import { afterEach, describe, expect, it } from 'vitest';
import { type AuthInterceptorClient, registerAuthInterceptors } from './interceptors';
import { clearOrgProvider, setOrgProvider } from './org-provider';
import { clearTokenProvider, setTokenProvider } from './token-provider';

/** Capture the interceptor fn that `registerAuthInterceptors` installs. */
function fakeClient(): {
  client: AuthInterceptorClient;
  run: (request: Request) => Promise<Request>;
} {
  let installed: ((request: Request) => Request | Promise<Request>) | undefined;
  const client: AuthInterceptorClient = {
    interceptors: {
      request: {
        use: (fn) => {
          installed = fn;
          return 0;
        },
      },
    },
  };
  return {
    client,
    run: async (request) => {
      if (!installed) throw new Error('no interceptor installed');
      return installed(request);
    },
  };
}

afterEach(() => {
  clearTokenProvider();
  clearOrgProvider();
});

describe('registerAuthInterceptors', () => {
  it('injects Authorization and X-Tenant-ID from the providers', async () => {
    setTokenProvider(() => 'tok-123');
    setOrgProvider(() => 'org-abc');
    const { client, run } = fakeClient();
    registerAuthInterceptors(client);

    const out = await run(new Request('https://api.test/x'));
    expect(out.headers.get('Authorization')).toBe('Bearer tok-123');
    expect(out.headers.get('X-Tenant-ID')).toBe('org-abc');
  });

  it('adds no auth headers when there is no token or org', async () => {
    setTokenProvider(() => null);
    setOrgProvider(() => null);
    const { client, run } = fakeClient();
    registerAuthInterceptors(client);

    const out = await run(new Request('https://api.test/x'));
    expect(out.headers.has('Authorization')).toBe(false);
    expect(out.headers.has('X-Tenant-ID')).toBe(false);
  });

  it('does NOT overwrite an explicit per-request header', async () => {
    setTokenProvider(() => 'tok-from-provider');
    setOrgProvider(() => 'org-from-provider');
    const { client, run } = fakeClient();
    registerAuthInterceptors(client);

    const req = new Request('https://api.test/x', {
      headers: { Authorization: 'Bearer explicit', 'X-Tenant-ID': 'org-explicit' },
    });
    const out = await run(req);
    expect(out.headers.get('Authorization')).toBe('Bearer explicit');
    expect(out.headers.get('X-Tenant-ID')).toBe('org-explicit');
  });
});
