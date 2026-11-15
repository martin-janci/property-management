/**
 * Regression coverage for the "default client is used while unconfigured" review
 * finding.
 *
 * `@hey-api/openapi-ts` compiles the OpenAPI `servers[0].url`
 * (`http://localhost:8080`) into `generated/client.gen.ts` as the client's
 * initial `baseUrl`. Only `ppt-web` overrides it (`main.tsx` →
 * `client.setConfig({ baseUrl: VITE_API_URL || '' })`). Any other consumer that
 * never calls `client.setConfig()` — e.g. `admin-web` — would otherwise send
 * EVERY request (the generated SDK and the raw `authenticatedFetch*` paths that
 * `lib/fetch.ts#resolveApiUrl` resolves against `client.getConfig().baseUrl`) to
 * an absolute `http://localhost:8080/...`, which targets the viewer's own
 * machine in staging/production and forces needless CORS in dev.
 *
 * `client-default.ts` relativizes that compiled-in absolute placeholder to a
 * same-origin default at package load. These tests pin both the pure rule and
 * the wired-up side effect on the real singleton.
 */

import { describe, expect, it } from 'vitest';
import { normalizeClientBaseUrl } from './client-default';
import { client } from './generated/client.gen';

const isAbsolute = (url: string): boolean => /^[a-z][a-z0-9+.-]*:\/\//i.test(url);

describe('normalizeClientBaseUrl', () => {
  it('relativizes a compiled-in absolute base URL (the generator default)', () => {
    expect(normalizeClientBaseUrl('http://localhost:8080')).toBe('');
    expect(normalizeClientBaseUrl('https://api.example.com')).toBe('');
    expect(normalizeClientBaseUrl('http://localhost:8080/')).toBe('');
  });

  it('leaves a relative (same-origin) base untouched', () => {
    expect(normalizeClientBaseUrl('')).toBeNull();
    expect(normalizeClientBaseUrl('/api/v1')).toBeNull();
    expect(normalizeClientBaseUrl(undefined)).toBeNull();
  });
});

describe('@ppt/api-client default client', () => {
  it('has no compiled-in absolute base URL once the package has loaded', () => {
    // Importing `./client-default` (above) runs its one-time normalization on
    // the shared singleton that `generated/client.gen.ts` exports — the same
    // object every SDK call and `lib/fetch.ts` use.
    const baseUrl = client.getConfig().baseUrl ?? '';
    expect(isAbsolute(baseUrl)).toBe(false);
  });
});
