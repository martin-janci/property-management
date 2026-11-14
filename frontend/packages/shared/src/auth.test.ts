import { describe, expect, it } from 'vitest';
import { createTokenStorage, isTokenExpired } from './auth';

/**
 * Build an unsigned JWT-shaped string (`header.payload.signature`) whose
 * payload is the given object, base64url-encoded. The signature segment is a
 * placeholder — these helpers never verify signatures, they only read claims.
 */
function makeToken(payload: Record<string, unknown>): string {
  // Symmetric to decodeJwt's atob/decodeURIComponent decoding: UTF-8 → base64
  // → URL-safe base64url (no padding).
  const encode = (obj: Record<string, unknown>): string =>
    btoa(
      encodeURIComponent(JSON.stringify(obj)).replace(/%([0-9A-F]{2})/g, (_, h) =>
        String.fromCharCode(Number.parseInt(h, 16))
      )
    )
      .replace(/\+/g, '-')
      .replace(/\//g, '_')
      .replace(/=+$/, '');
  const header = encode({ alg: 'HS256', typ: 'JWT' });
  return `${header}.${encode(payload)}.sig`;
}

const nowSec = () => Math.floor(Date.now() / 1000);

describe('isTokenExpired — exp bypass regression', () => {
  // These are the fail-closed cases. Before the fix, a missing / non-numeric /
  // NaN `exp` made `payload.exp <= now + buffer` evaluate to `false`, so the
  // token was accepted indefinitely. Each of these MUST now be treated as
  // expired (true).
  it('treats a token with NO exp claim as expired', () => {
    const token = makeToken({ sub: 'u1', iat: nowSec() });
    expect(isTokenExpired(token)).toBe(true);
  });

  it('treats a token with a string exp claim as expired', () => {
    const token = makeToken({ sub: 'u1', iat: nowSec(), exp: '9999999999' });
    expect(isTokenExpired(token)).toBe(true);
  });

  it('treats a token with a null exp claim as expired', () => {
    const token = makeToken({ sub: 'u1', iat: nowSec(), exp: null });
    expect(isTokenExpired(token)).toBe(true);
  });

  it('treats a token with a NaN exp claim as expired', () => {
    // JSON can't carry NaN directly, so smuggle it via a value that decodes to
    // a non-finite number path — use a boolean, which is also non-numeric.
    const token = makeToken({ sub: 'u1', iat: nowSec(), exp: true });
    expect(isTokenExpired(token)).toBe(true);
  });

  it('hasValidTokens() rejects a stored access token with no exp', () => {
    const storage = createTokenStorage('test_exp');
    storage.setTokens({
      accessToken: makeToken({ sub: 'u1', iat: nowSec() }),
      refreshToken: makeToken({ sub: 'u1', iat: nowSec(), exp: nowSec() + 3600 }),
    });
    expect(storage.hasValidTokens()).toBe(false);
    storage.clearTokens();
  });
});

describe('isTokenExpired — valid / expired numeric exp still behaves', () => {
  it('treats a token with a future exp as not expired', () => {
    const token = makeToken({ sub: 'u1', iat: nowSec(), exp: nowSec() + 3600 });
    expect(isTokenExpired(token)).toBe(false);
  });

  it('treats a token with a past exp as expired', () => {
    const token = makeToken({ sub: 'u1', iat: nowSec(), exp: nowSec() - 3600 });
    expect(isTokenExpired(token)).toBe(true);
  });

  it('treats a malformed (non-JWT) token as expired', () => {
    expect(isTokenExpired('not-a-jwt')).toBe(true);
  });
});
