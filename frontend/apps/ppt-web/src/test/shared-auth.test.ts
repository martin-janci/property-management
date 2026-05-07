/// <reference types="vitest/globals" />
/**
 * Tests for @ppt/shared auth utilities.
 *
 * Pure logic only: JWT decoding, expiry math, role hierarchy, auth headers,
 * and session storage helpers. Token storage creation is exercised against
 * the jsdom localStorage.
 */

import {
  ROLES,
  createAuthHeader,
  createTokenStorage,
  decodeJwt,
  extractTokenFromHeader,
  getAndClearReturnUrl,
  getTokenExpiration,
  getTokenRemainingTime,
  getUserFromToken,
  hasAnyRole,
  hasRolePermission,
  isTokenExpired,
  setReturnUrl,
} from '@ppt/shared';

/** Build a signed-looking (but not verified) JWT with the given payload. */
function makeJwt(payload: Record<string, unknown>): string {
  const header = btoa(JSON.stringify({ alg: 'HS256', typ: 'JWT' }))
    .replace(/=/g, '')
    .replace(/\+/g, '-')
    .replace(/\//g, '_');
  const body = btoa(JSON.stringify(payload))
    .replace(/=/g, '')
    .replace(/\+/g, '-')
    .replace(/\//g, '_');
  return `${header}.${body}.signature`;
}

describe('@ppt/shared - auth', () => {
  describe('decodeJwt', () => {
    it('decodes a valid token', () => {
      const exp = Math.floor(Date.now() / 1000) + 3600;
      const iat = Math.floor(Date.now() / 1000);
      const token = makeJwt({
        sub: 'user-1',
        email: 'u@example.com',
        name: 'User',
        tenant_id: 't-1',
        role: 'manager',
        iat,
        exp,
      });
      const decoded = decodeJwt(token);
      expect(decoded).not.toBeNull();
      expect(decoded?.sub).toBe('user-1');
      expect(decoded?.email).toBe('u@example.com');
      expect(decoded?.tenant_id).toBe('t-1');
      expect(decoded?.exp).toBe(exp);
    });

    it('returns null for malformed tokens', () => {
      expect(decodeJwt('')).toBeNull();
      expect(decodeJwt('not.a.jwt.extra')).toBeNull();
      expect(decodeJwt('only-one-segment')).toBeNull();
      expect(decodeJwt('a.b')).toBeNull(); // 2 segments, not 3
    });

    it('returns null when payload is not valid JSON', () => {
      // Second segment is base64 of non-JSON garbage
      const badPayload = btoa('{not json')
        .replace(/=/g, '')
        .replace(/\+/g, '-')
        .replace(/\//g, '_');
      expect(decodeJwt(`header.${badPayload}.sig`)).toBeNull();
    });
  });

  describe('isTokenExpired', () => {
    it('treats tokens within buffer as expired', () => {
      const exp = Math.floor(Date.now() / 1000) + 10; // 10s left, default buffer 30
      const token = makeJwt({ sub: 'u', iat: 0, exp });
      expect(isTokenExpired(token)).toBe(true);
    });

    it('accepts tokens comfortably in the future', () => {
      const exp = Math.floor(Date.now() / 1000) + 3600;
      const token = makeJwt({ sub: 'u', iat: 0, exp });
      expect(isTokenExpired(token)).toBe(false);
    });

    it('honors custom buffer', () => {
      const exp = Math.floor(Date.now() / 1000) + 10;
      const token = makeJwt({ sub: 'u', iat: 0, exp });
      expect(isTokenExpired(token, 0)).toBe(false);
      expect(isTokenExpired(token, 60)).toBe(true);
    });

    it('invalid tokens count as expired', () => {
      expect(isTokenExpired('garbage')).toBe(true);
    });
  });

  describe('getTokenExpiration', () => {
    it('returns a Date at exp seconds', () => {
      const exp = Math.floor(Date.now() / 1000) + 100;
      const token = makeJwt({ sub: 'u', iat: 0, exp });
      const d = getTokenExpiration(token);
      expect(d).toBeInstanceOf(Date);
      expect(d?.getTime()).toBe(exp * 1000);
    });

    it('returns null for invalid tokens', () => {
      expect(getTokenExpiration('bad')).toBeNull();
    });
  });

  describe('getTokenRemainingTime', () => {
    it('returns seconds until expiry', () => {
      vi.useFakeTimers();
      vi.setSystemTime(new Date('2026-01-01T00:00:00Z'));
      try {
        const exp = Math.floor(Date.now() / 1000) + 100;
        const token = makeJwt({ sub: 'u', iat: 0, exp });
        expect(getTokenRemainingTime(token)).toBe(100);
      } finally {
        vi.useRealTimers();
      }
    });

    it('returns 0 when already expired', () => {
      const exp = Math.floor(Date.now() / 1000) - 100;
      const token = makeJwt({ sub: 'u', iat: 0, exp });
      expect(getTokenRemainingTime(token)).toBe(0);
    });

    it('returns -1 for invalid tokens', () => {
      expect(getTokenRemainingTime('bad')).toBe(-1);
    });
  });

  describe('getUserFromToken', () => {
    it('extracts known claims', () => {
      const token = makeJwt({
        sub: 'u-1',
        email: 'a@b.co',
        name: 'A',
        tenant_id: 't-9',
        role: 'admin',
        iat: 0,
        exp: Math.floor(Date.now() / 1000) + 3600,
      });
      expect(getUserFromToken(token)).toEqual({
        id: 'u-1',
        email: 'a@b.co',
        name: 'A',
        tenantId: 't-9',
        role: 'admin',
      });
    });

    it('returns null for invalid tokens', () => {
      expect(getUserFromToken('not-a-jwt')).toBeNull();
    });
  });

  describe('createAuthHeader / extractTokenFromHeader', () => {
    it('creates a Bearer header', () => {
      expect(createAuthHeader('abc')).toBe('Bearer abc');
    });

    it('extracts token from Bearer header', () => {
      expect(extractTokenFromHeader('Bearer xyz')).toBe('xyz');
    });

    it('returns null for non-Bearer headers', () => {
      expect(extractTokenFromHeader('Basic abc')).toBeNull();
      expect(extractTokenFromHeader('xyz')).toBeNull();
    });

    it('round-trips', () => {
      const token = 'abc.def.ghi';
      expect(extractTokenFromHeader(createAuthHeader(token))).toBe(token);
    });
  });

  describe('role helpers', () => {
    it('has correct hierarchy: admin > manager > owner > resident > viewer', () => {
      expect(hasRolePermission('admin', ROLES.MANAGER)).toBe(true);
      expect(hasRolePermission('manager', ROLES.OWNER)).toBe(true);
      expect(hasRolePermission('owner', ROLES.RESIDENT)).toBe(true);
      expect(hasRolePermission('resident', ROLES.VIEWER)).toBe(true);

      expect(hasRolePermission('viewer', ROLES.RESIDENT)).toBe(false);
      expect(hasRolePermission('resident', ROLES.OWNER)).toBe(false);
      expect(hasRolePermission('owner', ROLES.MANAGER)).toBe(false);
      expect(hasRolePermission('manager', ROLES.ADMIN)).toBe(false);
    });

    it('allows the same role', () => {
      expect(hasRolePermission('manager', ROLES.MANAGER)).toBe(true);
    });

    it('rejects unknown roles', () => {
      expect(hasRolePermission('stranger', ROLES.VIEWER)).toBe(false);
      expect(hasRolePermission('admin', 'stranger' as never)).toBe(false);
    });

    it('hasAnyRole checks membership', () => {
      expect(hasAnyRole('manager', [ROLES.MANAGER, ROLES.ADMIN])).toBe(true);
      expect(hasAnyRole('viewer', [ROLES.MANAGER, ROLES.ADMIN])).toBe(false);
      expect(hasAnyRole('viewer', [])).toBe(false);
    });
  });

  describe('createTokenStorage', () => {
    const storage = createTokenStorage('ppt_test');

    beforeEach(() => {
      storage.clearTokens();
    });

    it('round-trips both tokens', () => {
      const future = Math.floor(Date.now() / 1000) + 3600;
      const access = makeJwt({ sub: 'u', iat: 0, exp: future });
      const refresh = makeJwt({ sub: 'u', iat: 0, exp: future });

      storage.setTokens({ accessToken: access, refreshToken: refresh });
      expect(storage.getAccessToken()).toBe(access);
      expect(storage.getRefreshToken()).toBe(refresh);
      expect(storage.getTokens()).toEqual({
        accessToken: access,
        refreshToken: refresh,
      });
    });

    it('returns null pair when tokens missing', () => {
      expect(storage.getTokens()).toBeNull();
    });

    it('hasValidTokens reflects access expiry', () => {
      const access = makeJwt({ sub: 'u', iat: 0, exp: Math.floor(Date.now() / 1000) + 3600 });
      storage.setAccessToken(access);
      expect(storage.hasValidTokens()).toBe(true);

      const expired = makeJwt({ sub: 'u', iat: 0, exp: Math.floor(Date.now() / 1000) - 100 });
      storage.setAccessToken(expired);
      expect(storage.hasValidTokens()).toBe(false);
    });

    it('canRefresh reflects refresh token validity', () => {
      expect(storage.canRefresh()).toBe(false);

      const refresh = makeJwt({
        sub: 'u',
        iat: 0,
        exp: Math.floor(Date.now() / 1000) + 86_400,
      });
      storage.setRefreshToken(refresh);
      expect(storage.canRefresh()).toBe(true);
    });

    it('clearTokens removes both', () => {
      storage.setTokens({ accessToken: 'a', refreshToken: 'r' });
      storage.clearTokens();
      expect(storage.getAccessToken()).toBeNull();
      expect(storage.getRefreshToken()).toBeNull();
    });

    it('namespaces keys by prefix', () => {
      const a = createTokenStorage('ns_a');
      const b = createTokenStorage('ns_b');
      a.setAccessToken('token-a');
      b.setAccessToken('token-b');
      expect(a.getAccessToken()).toBe('token-a');
      expect(b.getAccessToken()).toBe('token-b');
      a.clearTokens();
      b.clearTokens();
    });
  });

  describe('return URL helpers', () => {
    afterEach(() => {
      sessionStorage.clear();
    });

    it('stores and clears the return URL in one shot', () => {
      setReturnUrl('/dashboard');
      expect(getAndClearReturnUrl()).toBe('/dashboard');
      expect(getAndClearReturnUrl()).toBeNull();
    });

    it('returns null when nothing was stored', () => {
      expect(getAndClearReturnUrl()).toBeNull();
    });
  });
});
