/**
 * MFA challenge verification (N9).
 *
 * POSTs the user-supplied TOTP `code` to the api-server MFA verify endpoint
 * (`routes/mfa.rs`). On a 2xx the user is MFA-verified for the
 * RECENT_MFA_WINDOW (15 min) and the api-client retries the original mutation
 * transparently.
 *
 * Routes through the shared `authenticatedFetch` from `@ppt/api-client` — the
 * same authenticated client the generated SDK and the other core requests
 * (e.g. `gdprClient`, community) use — so the request resolves against the
 * configured API base (`client.setConfig({ baseUrl })` in `main.tsx`) and not
 * the web origin, and the bearer token plus `X-Tenant-ID` come from the
 * registered auth providers. Previously `App.tsx`'s `MfaWrapper` issued a raw
 * `fetch('/api/v1/auth/mfa/verify')` with a hand-built bearer header, which
 * resolved against the web origin and therefore hit the wrong host whenever
 * `VITE_API_URL` points at a different origin than the SPA.
 */

import { authenticatedFetch } from '@ppt/api-client';

/** api-server MFA verify endpoint (routes/mfa.rs). */
const MFA_VERIFY_PATH = '/api/v1/auth/mfa/verify';

/**
 * POST the TOTP `code` to the MFA verify endpoint. Resolves `true` when the
 * server accepts the code (2xx), `false` on any non-2xx response or a network
 * error.
 */
export async function verifyMfaCode(code: string): Promise<boolean> {
  try {
    const response = await authenticatedFetch(MFA_VERIFY_PATH, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ code }),
    });
    return response.ok;
  } catch {
    return false;
  }
}
