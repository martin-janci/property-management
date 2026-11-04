import { makeJwt } from '../test/jwt';
import { buildSessionUser, readTenants, readTokenPair, toAppRole } from './session';

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
});
