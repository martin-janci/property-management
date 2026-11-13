/**
 * MFA challenge verification (N9).
 *
 * POSTs the user-supplied TOTP `code` to the api-server MFA verify endpoint
 * (`routes/mfa.rs`). On a 2xx the user is MFA-verified for the
 * RECENT_MFA_WINDOW (15 min) and the api-client retries the original mutation
 * transparently.
 *
 * NOTE: this helper is the extracted seam for the former inline
 * `fetch('/api/v1/auth/mfa/verify')` in `App.tsx`'s `MfaWrapper`. It is kept
 * standalone so the base-URL handling can be unit-tested in isolation.
 */

/** api-server MFA verify endpoint (routes/mfa.rs). */
const MFA_VERIFY_PATH = '/api/v1/auth/mfa/verify';

/**
 * POST the TOTP `code` to the MFA verify endpoint. Resolves `true` when the
 * server accepts the code (2xx), `false` on any non-2xx response or a network
 * error.
 */
export async function verifyMfaCode(code: string): Promise<boolean> {
  try {
    const response = await fetch(MFA_VERIFY_PATH, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ code }),
    });
    return response.ok;
  } catch {
    return false;
  }
}
