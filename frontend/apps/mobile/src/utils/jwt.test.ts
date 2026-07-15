/**
 * Tests for the shared client-side JWT helpers.
 *
 * This is the *single* suite for `decodeJwtPayload` / `extractTenantId` after
 * the duplicate copies in `hooks/useApi.ts` and
 * `screens/documents/DocumentUploadScreen.tsx` were consolidated here
 * (GitHub #2327). `useApi.test.ts` still exercises `decodeJwtPayload`
 * indirectly through `getTenantId`; the screen no longer owns its own copy.
 *
 * The decode relies on the `atob`/`btoa` globals shipped by the jest-expo
 * runtime.
 */

import { decodeJwtPayload, extractTenantId } from './jwt';

/** Build an unsigned JWT (`header.base64url(payload).sig`). */
function makeJwt(payload: Record<string, unknown>): string {
  const b64url = (obj: unknown) =>
    globalThis.btoa(JSON.stringify(obj)).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
  return `${b64url({ alg: 'none', typ: 'JWT' })}.${b64url(payload)}.sig`;
}

describe('decodeJwtPayload', () => {
  it('decodes the payload object from a base64url JWT', () => {
    expect(decodeJwtPayload(makeJwt({ sub: 'user-1', tenant_id: 'org-7f3c' }))).toEqual({
      sub: 'user-1',
      tenant_id: 'org-7f3c',
    });
  });

  it('returns null for a token with fewer than two segments', () => {
    expect(decodeJwtPayload('not-a-jwt')).toBeNull();
  });

  it('returns null when the payload segment is not valid base64/JSON', () => {
    expect(decodeJwtPayload('header.%%%not-base64%%%.sig')).toBeNull();
  });
});

describe('extractTenantId', () => {
  it('decodes the tenant_id claim from a base64url JWT payload', () => {
    expect(extractTenantId(makeJwt({ tenant_id: 'org-7f3c' }))).toBe('org-7f3c');
  });

  it('returns null when the tenant_id claim is absent', () => {
    expect(extractTenantId(makeJwt({ sub: 'user-1' }))).toBeNull();
  });

  it('returns null when tenant_id is not a string', () => {
    expect(extractTenantId(makeJwt({ tenant_id: 42 }))).toBeNull();
  });

  it('returns null for a token with fewer than two segments', () => {
    expect(extractTenantId('not-a-jwt')).toBeNull();
  });

  it('returns null when the payload segment is not valid base64/JSON', () => {
    expect(extractTenantId('header.%%%not-base64%%%.sig')).toBeNull();
  });
});
