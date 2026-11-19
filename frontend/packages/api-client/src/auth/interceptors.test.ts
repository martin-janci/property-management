/**
 * Tests for the centralized auth request-interceptor (#1522).
 */

import { afterEach, describe, expect, it, vi } from 'vitest';
import { type AuthInterceptorClient, registerAuthInterceptors } from './interceptors';
import { clearOrgProvider, setOrgProvider } from './org-provider';
import { clearTokenProvider, setTokenProvider } from './token-provider';
import { clearUnauthorizedHandler, setUnauthorizedHandler } from './unauthorized-handler';

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

/**
 * Fake client that captures BOTH the request and response interceptors and can
 * drive a request through them, mimicking the generated client's phase order:
 * request interceptor runs (with the shared per-request `options`), then the
 * response interceptor sees the response alongside the same `options`.
 */
function fakeClientWithResponse(): {
  client: AuthInterceptorClient;
  runRequest: (request: Request, options: object) => Promise<Request>;
  runResponse: (response: Response, request: Request, options: object) => Promise<Response>;
} {
  let reqFn: ((request: Request, options?: unknown) => Request | Promise<Request>) | undefined;
  let resFn:
    | ((
        response: Response,
        request: Request,
        options: { fetch?: typeof fetch }
      ) => Response | Promise<Response>)
    | undefined;
  const client: AuthInterceptorClient = {
    interceptors: {
      request: {
        use: (fn) => {
          reqFn = fn;
          return 0;
        },
      },
      response: {
        use: (fn) => {
          resFn = fn as typeof resFn;
          return 0;
        },
      },
    },
  };
  return {
    client,
    runRequest: async (request, options) => {
      if (!reqFn) throw new Error('no request interceptor installed');
      return reqFn(request, options);
    },
    runResponse: async (response, request, options) => {
      if (!resFn) throw new Error('no response interceptor installed');
      return resFn(response, request, options as { fetch?: typeof fetch });
    },
  };
}

afterEach(() => {
  clearTokenProvider();
  clearOrgProvider();
  clearUnauthorizedHandler();
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

describe('registerAuthInterceptors — 401 refresh-and-replay', () => {
  it('passes a non-401 response straight through', async () => {
    const { client, runResponse } = fakeClientWithResponse();
    registerAuthInterceptors(client);

    const ok = new Response('ok', { status: 200 });
    const out = await runResponse(ok, new Request('https://api.test/x'), {});
    expect(out).toBe(ok);
  });

  it('refreshes on 401 and replays the original request with the rotated token', async () => {
    setTokenProvider(() => 'stale-token');
    const handler = vi.fn(async () => 'fresh-token');
    setUnauthorizedHandler(handler);

    const { client, runRequest, runResponse } = fakeClientWithResponse();
    registerAuthInterceptors(client);

    // A real request flows through the request interceptor first (shared opts).
    const options: { fetch: ReturnType<typeof vi.fn> } = {
      fetch: vi.fn(async (_req: Request) => new Response('replayed', { status: 200 })),
    };
    const original = new Request('https://api.test/widgets', { method: 'GET' });
    await runRequest(original, options);
    expect(original.headers.get('Authorization')).toBe('Bearer stale-token');

    // The server rejects with 401 — interceptor must refresh and replay.
    const replayed = await runResponse(new Response('nope', { status: 401 }), original, options);

    expect(handler).toHaveBeenCalledTimes(1);
    expect(options.fetch).toHaveBeenCalledTimes(1);
    const replayReq = options.fetch.mock.calls[0][0] as Request;
    expect(replayReq.headers.get('Authorization')).toBe('Bearer fresh-token');
    expect(replayReq.url).toBe('https://api.test/widgets');
    expect(replayed.status).toBe(200);
    expect(await replayed.text()).toBe('replayed');
  });

  it('replays a request that carried a body', async () => {
    setTokenProvider(() => 'stale');
    setUnauthorizedHandler(async () => 'fresh');

    const { client, runRequest, runResponse } = fakeClientWithResponse();
    registerAuthInterceptors(client);

    const seen: Request[] = [];
    const options = {
      fetch: vi.fn(async (req: Request) => {
        seen.push(req);
        return new Response(null, { status: 200 });
      }),
    };
    const original = new Request('https://api.test/widgets', {
      method: 'POST',
      body: JSON.stringify({ name: 'w' }),
      headers: { 'Content-Type': 'application/json' },
    });
    await runRequest(original, options);
    await runResponse(new Response(null, { status: 401 }), original, options);

    expect(options.fetch).toHaveBeenCalledTimes(1);
    expect(await seen[0].text()).toBe('{"name":"w"}');
    expect(seen[0].headers.get('Authorization')).toBe('Bearer fresh');
  });

  it('surfaces the 401 (no replay) when no handler is registered', async () => {
    const { client, runRequest, runResponse } = fakeClientWithResponse();
    registerAuthInterceptors(client);

    const options = { fetch: vi.fn() };
    const original = new Request('https://api.test/x');
    await runRequest(original, options);
    const resp = new Response(null, { status: 401 });
    const out = await runResponse(resp, original, options);

    expect(out).toBe(resp);
    expect(options.fetch).not.toHaveBeenCalled();
  });

  it('surfaces the 401 when the handler cannot recover the session (null)', async () => {
    const handler = vi.fn(async () => null);
    setUnauthorizedHandler(handler);

    const { client, runRequest, runResponse } = fakeClientWithResponse();
    registerAuthInterceptors(client);

    const options = { fetch: vi.fn() };
    const original = new Request('https://api.test/x');
    await runRequest(original, options);
    const resp = new Response(null, { status: 401 });
    const out = await runResponse(resp, original, options);

    expect(handler).toHaveBeenCalledTimes(1);
    expect(options.fetch).not.toHaveBeenCalled();
    expect(out).toBe(resp);
  });

  it('replays at most once — a replay that also 401s surfaces', async () => {
    const handler = vi.fn(async () => 'fresh');
    setUnauthorizedHandler(handler);

    const { client, runRequest, runResponse } = fakeClientWithResponse();
    registerAuthInterceptors(client);

    const options = {
      fetch: vi.fn(async () => new Response(null, { status: 401 })),
    };
    const original = new Request('https://api.test/x');
    await runRequest(original, options);

    // First 401 → refresh + replay (replay also 401s, returned as the result).
    const first = await runResponse(new Response(null, { status: 401 }), original, options);
    expect(first.status).toBe(401);

    // Feeding that replay's 401 back through must NOT refresh again (guarded).
    const second = await runResponse(first, original, options);
    expect(second).toBe(first);
    expect(handler).toHaveBeenCalledTimes(1);
    expect(options.fetch).toHaveBeenCalledTimes(1);
  });

  it('surfaces the 401 when the handler throws', async () => {
    const handler = vi.fn(async () => {
      throw new Error('refresh failed');
    });
    setUnauthorizedHandler(handler);

    const { client, runRequest, runResponse } = fakeClientWithResponse();
    registerAuthInterceptors(client);

    const options = { fetch: vi.fn() };
    const original = new Request('https://api.test/x');
    await runRequest(original, options);
    const resp = new Response(null, { status: 401 });
    const out = await runResponse(resp, original, options);

    expect(handler).toHaveBeenCalledTimes(1);
    expect(options.fetch).not.toHaveBeenCalled();
    expect(out).toBe(resp);
  });
});
