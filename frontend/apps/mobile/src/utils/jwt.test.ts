/**
 * Unit tests for the shared JWT decode util.
 *
 * `src/utils/jwt.ts` is the single source of truth for the base64url JWT
 * payload decode + `tenant_id` extraction that feeds the `X-Tenant-ID` header
 * on every tenant-scoped request. It used to be triplicated (issues #2327 /
 * #2329) with only the `useApi` copy tested; these cases pin the shared unit
 * directly so all three callers (`useApi`, `useOfflineSupport`,
 * `DocumentUploadScreen`) are covered by one suite.
 */

import { makeJwt } from '../test/jwt';
import { decodeJwtPayload, extractTenantId, isJwtExpired } from './jwt';

const nowSec = () => Math.floor(Date.now() / 1000);

describe('decodeJwtPayload', () => {
  it('decodes a well-formed payload into an object', () => {
    expect(decodeJwtPayload(makeJwt({ sub: 'u-1', tenant_id: 'org-42' }))).toEqual({
      sub: 'u-1',
      tenant_id: 'org-42',
    });
  });

  // Byte lengths chosen so the base64url length hits each `% 4` residue,
  // forcing the padding math down all three branches (0, 1, 2 `=`).
  it.each([
    ['no padding needed (len % 4 === 0)', 'tt'],
    ['two padding chars (len % 4 === 2)', 'ttt'],
    ['one padding char (len % 4 === 3)', 't'],
  ])('decodes a base64url payload with %s', (_label, tenant) => {
    expect(decodeJwtPayload(makeJwt({ tenant_id: tenant }))).toEqual({ tenant_id: tenant });
  });

  it('decodes a payload whose base64url segment still carries `=` padding', () => {
    const withPadding = makeJwt({ tenant_id: 'ttt' }, /* keepPadding */ true);
    // Guard the fixture: this variant must actually contain `=` so the test
    // exercises the padded path rather than silently matching the stripped one.
    expect(withPadding).toContain('=');
    expect(decodeJwtPayload(withPadding)).toEqual({ tenant_id: 'ttt' });
  });

  it('converts base64url `-` and `_` back to `+`/`/` before decoding', () => {
    const dashToken = makeJwt({ tenant_id: 'tn->??' });
    const underscoreToken = makeJwt({ tenant_id: 'tn-???' });
    expect(dashToken.split('.')[1]).toContain('-');
    expect(underscoreToken.split('.')[1]).toContain('_');

    expect(decodeJwtPayload(dashToken)).toEqual({ tenant_id: 'tn->??' });
    expect(decodeJwtPayload(underscoreToken)).toEqual({ tenant_id: 'tn-???' });
  });

  it.each([
    ['a token with no dots', 'not-a-jwt'],
    ['a single-segment token', 'onlyheader'],
    ['an empty string', ''],
    ['a token whose payload is not valid base64', 'header.!!!not-base64!!!.sig'],
    ['a token whose payload is not JSON', `header.${btoa('plain text').replace(/=+$/, '')}.sig`],
  ])('returns null (no throw) for %s', (_label, token) => {
    expect(decodeJwtPayload(token)).toBeNull();
  });

  it('mojibakes multi-byte (UTF-8) claim values — Latin1 `atob` limitation, pinned', () => {
    // A real server base64-encodes the UTF-8 *bytes* of the payload. `atob`
    // returns a Latin1 string, so a genuinely multi-byte char (outside the
    // Latin1 range that `btoa` can encode) comes back mojibake'd. `tenant_id`
    // is a UUID today so this never bites; this pins the limitation the source
    // only notes in a comment. Built by hand because `makeJwt`'s `btoa` can't
    // encode multi-byte chars directly.
    const json = JSON.stringify({ note: '你好' });
    const b64url = btoa(String.fromCharCode(...new TextEncoder().encode(json)))
      .replace(/\+/g, '-')
      .replace(/\//g, '_')
      .replace(/=+$/, '');
    const decoded = decodeJwtPayload(`header.${b64url}.sig`);
    expect(decoded).not.toBeNull();
    expect(decoded?.note).not.toBe('你好');
  });
});

describe('extractTenantId', () => {
  it('extracts the tenant_id claim from a well-formed token', () => {
    expect(extractTenantId(makeJwt({ sub: 'u-1', tenant_id: 'org-42' }))).toBe('org-42');
  });

  it('returns null when the payload has no tenant_id claim', () => {
    expect(extractTenantId(makeJwt({ sub: 'u-1', role: 'owner' }))).toBeNull();
  });

  it('returns null when tenant_id is present but not a string', () => {
    expect(extractTenantId(makeJwt({ tenant_id: 12345 }))).toBeNull();
  });

  it('returns null (no throw) for a malformed token', () => {
    expect(extractTenantId('not-a-jwt')).toBeNull();
  });
});

describe('isJwtExpired', () => {
  it('returns true for a token whose exp is in the past', () => {
    expect(isJwtExpired(makeJwt({ sub: 'u-1', exp: nowSec() - 60 }))).toBe(true);
  });

  it('returns false for a token whose exp is comfortably in the future', () => {
    expect(isJwtExpired(makeJwt({ sub: 'u-1', exp: nowSec() + 3600 }))).toBe(false);
  });

  it('treats a token within the skew window as already expired', () => {
    // exp is 10s out but the skew is 30s, so it should be considered stale.
    expect(isJwtExpired(makeJwt({ exp: nowSec() + 10 }), 30)).toBe(true);
    // Outside the skew window it is still valid.
    expect(isJwtExpired(makeJwt({ exp: nowSec() + 120 }), 30)).toBe(false);
  });

  it('returns false when exp is missing or non-numeric (defer to server 401)', () => {
    expect(isJwtExpired(makeJwt({ sub: 'u-1' }))).toBe(false);
    expect(isJwtExpired(makeJwt({ exp: 'soon' }))).toBe(false);
  });

  it('returns false (no throw) for a malformed token', () => {
    expect(isJwtExpired('not-a-jwt')).toBe(false);
    expect(isJwtExpired('')).toBe(false);
  });
});
