/**
 * Safe base-URL default for the generated `@hey-api` client.
 *
 * `@hey-api/openapi-ts` compiles the OpenAPI `servers[0].url` (currently
 * `http://localhost:8080`) into `generated/client.gen.ts` as the client's
 * initial `baseUrl`. That absolute dev URL is correct for exactly one local
 * setup. Only `ppt-web` reconfigures it (`main.tsx` →
 * `client.setConfig({ baseUrl: VITE_API_URL || '' })`). Any other consumer that
 * never calls `client.setConfig()` — e.g. `admin-web` — would otherwise send
 * EVERY request to that absolute host: both the generated SDK and the raw
 * `authenticatedFetch` / `authenticatedFetchJson` paths that
 * `lib/fetch.ts#resolveApiUrl` resolves against `client.getConfig().baseUrl`.
 * Off-localhost that points at the viewer's own machine (broken in
 * staging/production) and forces needless CORS in dev.
 *
 * A relative base (`''`) is the safe SPA default: requests resolve same-origin
 * and flow through each app's dev proxy. Apps that must target another host
 * still override it explicitly at bootstrap.
 *
 * This lives in a NON-generated module (re-exported by the package index) so
 * `pnpm generate` can never silently reintroduce the footgun.
 */

import { client } from './generated/client.gen';

const ABSOLUTE_URL = /^[a-z][a-z0-9+.-]*:\/\//i;

/**
 * Decide the base URL to apply given the generated client's compiled-in value.
 *
 * Returns `''` when `baseUrl` is an absolute (`scheme://host`) URL — the
 * generator's compiled-in default is never the right runtime base for a
 * deployed SPA — and `null` when the current value is already a safe relative
 * base (or unset) and should be left untouched.
 */
export function normalizeClientBaseUrl(baseUrl: string | undefined): string | null {
  if (baseUrl && ABSOLUTE_URL.test(baseUrl)) {
    return '';
  }
  return null;
}

// Apply the safe default once, at package load — before any app bootstrap or
// request. Mutates the shared singleton that `generated/client.gen.ts` exports,
// which is the same object every SDK call and `lib/fetch.ts` resolve against.
const normalized = normalizeClientBaseUrl(client.getConfig().baseUrl);
if (normalized !== null) {
  client.setConfig({ baseUrl: normalized });
}

export { client };
