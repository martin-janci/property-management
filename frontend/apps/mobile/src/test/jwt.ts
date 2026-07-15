/**
 * Shared JWT fixture builder for tests.
 *
 * Previously this helper was copy-pasted into `useApi.test.ts` and
 * `DocumentUploadScreen.test.tsx` (issue #2329). It lives here now so both
 * suites (and any future one) build tokens the same way.
 *
 * Produces `header.<base64url(payload)>.signature`. The header and signature
 * segments are literal placeholders — the decoder only reads the payload
 * segment — which keeps `Authorization: Bearer header.…`-style assertions
 * stable.
 */

/**
 * Build a JWT-shaped string whose payload is the given object, encoded exactly
 * the way a real base64url JWT payload is.
 *
 * `keepPadding` retains the standard-base64 `=` padding instead of stripping it
 * (real JWTs strip it, but the decoder must tolerate both — see issue #2283).
 */
export function makeJwt(payload: Record<string, unknown>, keepPadding = false): string {
  // `btoa` is a global in the RN / jest-expo runtime (Node ≥ 16 too), the same
  // one the source decoder pairs with `atob`, so the test stays off `Buffer`.
  // Payloads are ASCII, so Latin1 `btoa` is safe.
  const base64 = btoa(JSON.stringify(payload)).replace(/\+/g, '-').replace(/\//g, '_');
  const segment = keepPadding ? base64 : base64.replace(/=+$/, '');
  return `header.${segment}.signature`;
}
