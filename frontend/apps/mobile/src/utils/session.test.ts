import { makeJwt } from '../test/jwt';
import {
  buildSessionUser,
  parseStoredUser,
  readTenants,
  readTokenPair,
  toAppRole,
} from './session';

describe('readTokenPair', () => {
  it('reads the camelCase api-server shape', () => {
    expect(readTokenPair({ accessToken: 'a', refreshToken: 'r' })).toEqual({
      accessToken: 'a',
      refreshToken: 'r',
    });
  });

  it('still accepts the legacy snake_case shape', () => {
    expect(readTokenPair({ access_token: 'a', refresh_token: 'r' })).toEqual({
      accessToken: 'a',
      refreshToken: 'r',
    });
  });

  it('returns null when a token is missing', () => {
    expect(readTokenPair({ accessToken: 'a' })).toBeNull();
    expect(readTokenPair(null)).toBeNull();
  });
});

describe('readTenants', () => {
  it('normalises tenants and drops malformed entries', () => {
    expect(
      readTenants({
        tenants: [{ tenantId: 't1', tenantName: 'Org', role: 'manager' }, { foo: 1 }, null],
      })
    ).toEqual([{ tenantId: 't1', tenantName: 'Org', role: 'manager' }]);
  });
});

describe('toAppRole', () => {
  it.each([
    ['org_admin', 'admin'],
    ['manager', 'manager'],
    ['property_manager', 'manager'],
    ['owner_delegate', 'owner'],
    ['tenant', 'tenant'],
    ['guest', 'resident'],
    [undefined, 'resident'],
  ])('%s -> %s', (input, expected) => {
    expect(toAppRole(input)).toBe(expected);
  });
});

describe('buildSessionUser', () => {
  it('derives the user from claims and the first tenant', () => {
    const token = makeJwt({ sub: 'u1', email: 'j@x.sk', name: 'Ján Novák Starší' });
    const { user, tenantId } = buildSessionUser(
      { tenants: [{ tenantId: 'o1', tenantName: 'SVB', role: 'tenant' }] },
      token
    );
    expect(tenantId).toBe('o1');
    expect(user).toEqual({
      id: 'u1',
      email: 'j@x.sk',
      firstName: 'Ján',
      lastName: 'Novák Starší',
      role: 'tenant',
      organizationId: 'o1',
      organizationName: 'SVB',
    });
  });

  it('prefers the tenant named by the tenant_id claim', () => {
    const token = makeJwt({ sub: 'u1', tenant_id: 'o2' });
    const { user, tenantId } = buildSessionUser(
      {
        tenants: [
          { tenantId: 'o1', role: 'tenant' },
          { tenantId: 'o2', tenantName: 'Second', role: 'manager' },
        ],
      },
      token
    );
    expect(tenantId).toBe('o2');
    expect(user.role).toBe('manager');
    expect(user.organizationName).toBe('Second');
  });

  it('keeps an embedded user object', () => {
    const embedded = { id: 'u5', email: 'a@b.c', firstName: 'A', lastName: 'B', role: 'owner' };
    const { user } = buildSessionUser({ user: embedded }, makeJwt({ sub: 'u5' }));
    expect(user).toEqual(embedded);
  });

  it('narrows a malformed embedded user instead of trusting it verbatim', () => {
    // The login response body is attacker-influenceable / can drift from the
    // contract: guard against non-string, missing, and junk fields rather than
    // casting the embedded object straight into a SessionUser (code-review:
    // unvalidated token response body).
    const token = makeJwt({ sub: 'u9', email: 'claim@x.sk', name: 'Claim Name' });
    const { user } = buildSessionUser(
      {
        user: {
          id: 'u9',
          email: 42, // malformed — not a string
          firstName: null, // missing
          role: 'wizard', // unknown role must not leak past toAppRole()
          isAdmin: true, // junk field must be dropped, not spread through
        },
      },
      token
    );
    expect(user.id).toBe('u9');
    // Non-string email is rejected and falls back to the token claim.
    expect(user.email).toBe('claim@x.sk');
    expect(user.firstName).toBe('');
    expect(user.lastName).toBe('');
    // Unknown role is narrowed to the safe default, never trusted verbatim.
    expect(user.role).toBe('resident');
    // Junk fields from the response are not carried into the session user.
    expect(user).not.toHaveProperty('isAdmin');
  });
});

describe('parseStoredUser', () => {
  it('parses a well-formed stored user', () => {
    const json = JSON.stringify({
      id: 'u1',
      email: 'j@x.sk',
      firstName: 'Ján',
      lastName: 'Novák',
      role: 'manager',
      buildingId: 'b1',
      unitId: 'un1',
      avatarUrl: 'https://x/a.png',
      organizationId: 'o1',
      organizationName: 'SVB',
    });
    expect(parseStoredUser(json)).toEqual({
      id: 'u1',
      email: 'j@x.sk',
      firstName: 'Ján',
      lastName: 'Novák',
      role: 'manager',
      buildingId: 'b1',
      unitId: 'un1',
      avatarUrl: 'https://x/a.png',
      organizationId: 'o1',
      organizationName: 'SVB',
    });
  });

  it('runs the persisted role through toAppRole instead of trusting it verbatim', () => {
    // SecureStore `ppt_user` is on-device, mutable persisted state: a tampered
    // or drifted blob must never grant an arbitrary role. An unknown role is
    // narrowed to the safe default, and a privileged server role collapses onto
    // the app's role set exactly like the login path does (code-review:
    // session role cast bypass).
    expect(parseStoredUser(JSON.stringify({ id: 'u1', role: 'root' }))?.role).toBe('resident');
    expect(parseStoredUser(JSON.stringify({ id: 'u1', role: 'super_admin' }))?.role).toBe('admin');
    // A non-string role must not be trusted either.
    expect(parseStoredUser(JSON.stringify({ id: 'u1', role: 42 }))?.role).toBe('resident');
  });

  it('narrows non-string fields and drops junk keys', () => {
    const user = parseStoredUser(
      JSON.stringify({ id: 'u1', email: 7, firstName: null, isAdmin: true, role: 'owner' })
    );
    expect(user?.email).toBe('');
    expect(user?.firstName).toBe('');
    expect(user?.role).toBe('owner');
    expect(user).not.toHaveProperty('isAdmin');
  });

  it('returns null for unusable input', () => {
    expect(parseStoredUser(null)).toBeNull();
    expect(parseStoredUser('not json')).toBeNull();
    expect(parseStoredUser('42')).toBeNull();
    // A stored user with no id is unusable — restoring it would create a
    // half-formed session.
    expect(parseStoredUser(JSON.stringify({ role: 'admin' }))).toBeNull();
  });
});
