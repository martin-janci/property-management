/**
 * Unit tests for the mobile app's core fetch/auth module.
 *
 * `useApi.ts` owns tenant-scoping for *every* api-server request: it decodes
 * the JWT client-side to pull the `tenant_id` claim and injects it as the
 * `X-Tenant-ID` header that the server's RLS extractor requires. Until now it
 * had no direct coverage — every consumer just mocked it — so these tests pin
 * the three pure-ish units the module is built on:
 *
 * - `decodeJwtPayload` (exercised through `getTenantId`, which is the only
 *   caller): base64url with and without `=` padding, the `-`/`_` → `+`/`/`
 *   conversion, and malformed tokens returning `null` without throwing.
 * - `getTenantId`: extracts the `tenant_id` claim, and yields `null` (so no
 *   header is injected) when the token/claim is missing or non-string.
 * - `apiRequest`: composes the `X-Tenant-ID` header, surfaces server error
 *   messages, and handles a 204 No-Content response.
 *
 * `expo-secure-store` and `expo-constants` are mocked globally in
 * `src/test/setup.ts`; `fetch` is stubbed per suite.
 */

import * as SecureStore from 'expo-secure-store';
import { apiRequest, getTenantId } from './useApi';

const mockedGetItem = SecureStore.getItemAsync as jest.Mock;

const ACCESS_TOKEN_KEY = 'ppt_access_token';

/**
 * Build a JWT-shaped string (`header.payload.signature`) whose payload is the
 * given object, encoded exactly the way a real base64url JWT payload is.
 *
 * `keepPadding` retains the standard-base64 `=` padding instead of stripping
 * it (real JWTs strip it, but the decoder must tolerate both — see the issue).
 */
function makeJwt(payload: Record<string, unknown>, keepPadding = false): string {
  // `btoa` (a global in the RN runtime and Node ≥16, same one the source's
  // decoder pairs with `atob`) keeps the test off `Buffer`, which isn't in the
  // mobile app's TS `types`. Payloads are ASCII, so Latin1 `btoa` is safe.
  const base64 = btoa(JSON.stringify(payload)).replace(/\+/g, '-').replace(/\//g, '_');
  const segment = keepPadding ? base64 : base64.replace(/=+$/, '');
  return `header.${segment}.signature`;
}

/** Prime the SecureStore mock so `getAccessToken()` returns `token`. */
function primeToken(token: string | null): void {
  mockedGetItem.mockImplementation(async (key: string) =>
    key === ACCESS_TOKEN_KEY ? token : null
  );
}

beforeEach(() => {
  jest.clearAllMocks();
});

describe('getTenantId (and decodeJwtPayload via it)', () => {
  it('extracts the tenant_id claim from a well-formed token', async () => {
    primeToken(makeJwt({ sub: 'u-1', tenant_id: 'org-42' }));
    await expect(getTenantId()).resolves.toBe('org-42');
  });

  // Byte lengths chosen so the base64url length hits each `% 4` residue,
  // forcing the decoder's padding math down all three branches (0, 1, 2 `=`).
  it.each([
    ['no padding needed (len % 4 === 0)', 'tt'],
    ['two padding chars (len % 4 === 2)', 'ttt'],
    ['one padding char (len % 4 === 3)', 't'],
  ])('decodes a base64url payload with %s', async (_label, tenant) => {
    primeToken(makeJwt({ tenant_id: tenant }));
    await expect(getTenantId()).resolves.toBe(tenant);
  });

  it('decodes a payload whose base64url segment still carries `=` padding', async () => {
    const withPadding = makeJwt({ tenant_id: 'ttt' }, /* keepPadding */ true);
    // Guard the fixture: this variant must actually contain `=` so the test
    // is exercising the padded path rather than silently matching the stripped one.
    expect(withPadding).toContain('=');
    primeToken(withPadding);
    await expect(getTenantId()).resolves.toBe('ttt');
  });

  it('converts base64url `-` and `_` back to `+`/`/` before decoding', async () => {
    // 'tn->??' encodes to a segment containing '-'; 'tn-???' contains '_'.
    // Both must round-trip, proving the char-class replacement runs.
    const dashToken = makeJwt({ tenant_id: 'tn->??' });
    const underscoreToken = makeJwt({ tenant_id: 'tn-???' });
    expect(dashToken.split('.')[1]).toContain('-');
    expect(underscoreToken.split('.')[1]).toContain('_');

    primeToken(dashToken);
    await expect(getTenantId()).resolves.toBe('tn->??');

    primeToken(underscoreToken);
    await expect(getTenantId()).resolves.toBe('tn-???');
  });

  it.each([
    ['a token with no dots', 'not-a-jwt'],
    ['a single-segment token', 'onlyheader'],
    ['an empty string', ''],
    ['a token whose payload is not valid base64', 'header.!!!not-base64!!!.sig'],
    ['a token whose payload is not JSON', `header.${btoa('plain text').replace(/=+$/, '')}.sig`],
  ])('returns null (no throw) for %s', async (_label, token) => {
    primeToken(token);
    await expect(getTenantId()).resolves.toBeNull();
  });

  it('returns null when the payload has no tenant_id claim', async () => {
    primeToken(makeJwt({ sub: 'u-1', role: 'owner' }));
    await expect(getTenantId()).resolves.toBeNull();
  });

  it('returns null when tenant_id is present but not a string', async () => {
    primeToken(makeJwt({ tenant_id: 12345 }));
    await expect(getTenantId()).resolves.toBeNull();
  });

  it('returns null when no token is stored', async () => {
    primeToken(null);
    await expect(getTenantId()).resolves.toBeNull();
  });
});

describe('apiRequest', () => {
  let fetchMock: jest.Mock;

  /** Build a minimal `Response`-like object for the fetch mock. */
  function fakeResponse(opts: { status?: number; ok?: boolean; json?: () => Promise<unknown> }) {
    const status = opts.status ?? 200;
    return {
      ok: opts.ok ?? (status >= 200 && status < 300),
      status,
      json: opts.json ?? (async () => ({})),
    };
  }

  beforeEach(() => {
    fetchMock = jest.fn();
    globalThis.fetch = fetchMock as unknown as typeof fetch;
  });

  /** Return the headers object fetch was called with on its first call. */
  function firstCallHeaders(): Record<string, string> {
    const [, init] = fetchMock.mock.calls[0];
    return init.headers as Record<string, string>;
  }

  it('injects Authorization and X-Tenant-ID headers from the token', async () => {
    primeToken(makeJwt({ sub: 'u-1', tenant_id: 'org-42' }));
    fetchMock.mockResolvedValueOnce(fakeResponse({ json: async () => ({ ok: true }) }));

    const result = await apiRequest<{ ok: boolean }>('https://api.test/v1/outages');

    expect(result).toEqual({ ok: true });
    const headers = firstCallHeaders();
    expect(headers.Authorization).toMatch(/^Bearer header\./);
    expect(headers['X-Tenant-ID']).toBe('org-42');
    expect(headers.Accept).toBe('application/json');
  });

  it('omits X-Tenant-ID when the token has no tenant_id claim', async () => {
    primeToken(makeJwt({ sub: 'u-1' }));
    fetchMock.mockResolvedValueOnce(fakeResponse({ json: async () => ({}) }));

    await apiRequest('https://api.test/v1/me');

    const headers = firstCallHeaders();
    expect(headers.Authorization).toBeDefined();
    expect(headers).not.toHaveProperty('X-Tenant-ID');
  });

  it('sets Content-Type and serializes the body for a mutation', async () => {
    primeToken(makeJwt({ tenant_id: 'org-42' }));
    fetchMock.mockResolvedValueOnce(fakeResponse({ json: async () => ({ id: 1 }) }));

    await apiRequest('https://api.test/v1/faults', {
      method: 'POST',
      body: { title: 'Leak' },
    });

    const [, init] = fetchMock.mock.calls[0];
    expect((init.headers as Record<string, string>)['Content-Type']).toBe('application/json');
    expect(init.body).toBe(JSON.stringify({ title: 'Leak' }));
  });

  it('surfaces the server error message from a JSON error body', async () => {
    primeToken(makeJwt({ tenant_id: 'org-42' }));
    fetchMock.mockResolvedValueOnce(
      fakeResponse({
        status: 403,
        ok: false,
        json: async () => ({ message: 'Forbidden for tenant' }),
      })
    );

    await expect(apiRequest('https://api.test/v1/buildings')).rejects.toThrow(
      'Forbidden for tenant'
    );
  });

  it('falls back to the HTTP status when the error body is not JSON', async () => {
    primeToken(makeJwt({ tenant_id: 'org-42' }));
    fetchMock.mockResolvedValueOnce(
      fakeResponse({
        status: 500,
        ok: false,
        json: async () => {
          throw new Error('Unexpected token < in JSON');
        },
      })
    );

    await expect(apiRequest('https://api.test/v1/buildings')).rejects.toThrow('HTTP 500');
  });

  it('returns undefined for a 204 No-Content response without parsing a body', async () => {
    primeToken(makeJwt({ tenant_id: 'org-42' }));
    const json = jest.fn();
    fetchMock.mockResolvedValueOnce(fakeResponse({ status: 204, json }));

    const result = await apiRequest('https://api.test/v1/faults/1', { method: 'DELETE' });

    expect(result).toBeUndefined();
    expect(json).not.toHaveBeenCalled();
  });
});
