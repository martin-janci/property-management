import { describe, expect, it } from 'vitest';
import { buildSessionUser } from './AuthContext';

function jwt(claims: Record<string, unknown>): string {
  // Encode UTF-8 like a real JWT so non-ASCII names (Manažér) round-trip.
  const b64 = (o: unknown) =>
    btoa(String.fromCharCode(...new TextEncoder().encode(JSON.stringify(o))))
      .replace(/\+/g, '-')
      .replace(/\//g, '_')
      .replace(/=+$/, '');
  return `${b64({ alg: 'HS256', typ: 'JWT' })}.${b64(claims)}.sig`;
}

const ORG = '0ff32ca6-f9b2-4abf-a6af-f86418892190';

describe('buildSessionUser', () => {
  it('derives the user from token claims when the login response has no `user` (real api-server shape)', () => {
    const user = buildSessionUser({
      accessToken: jwt({ sub: 'u-1', email: 'peter@example.test', name: 'Peter Manažér' }),
      tenants: [{ tenantId: ORG, tenantName: 'Demo správa', role: 'manager' }],
    });
    expect(user).toEqual({
      id: 'u-1',
      email: 'peter@example.test',
      firstName: 'Peter',
      lastName: 'Manažér',
      role: 'manager',
      organizationId: ORG,
      organizationName: 'Demo správa',
    });
  });

  it('picks the tenant named by the tenant_id claim', () => {
    const other = '11111111-1111-1111-1111-111111111111';
    const user = buildSessionUser({
      accessToken: jwt({ sub: 'u-1', email: 'a@b.c', name: 'A', tenant_id: other }),
      tenants: [
        { tenantId: ORG, tenantName: 'First', role: 'org_admin' },
        { tenantId: other, tenantName: 'Second', role: 'owner' },
      ],
    });
    expect(user.organizationId).toBe(other);
    expect(user.role).toBe('owner');
    expect(user.lastName).toBeUndefined();
  });

  it('keeps an embedded user object and fills in the organization', () => {
    const user = buildSessionUser({
      accessToken: jwt({ sub: 'u-2' }),
      tenants: [{ tenantId: ORG, tenantName: 'Demo', role: 'tenant' }],
      user: { id: 'u-2', email: 'x@y.z', firstName: 'X' },
    });
    expect(user).toMatchObject({ id: 'u-2', firstName: 'X', role: 'tenant', organizationId: ORG });
  });

  it('returns a user without role or organization when there are no memberships', () => {
    const user = buildSessionUser({
      accessToken: jwt({ sub: 'u-3', email: 'n@o.p' }),
      tenants: [],
    });
    expect(user.role).toBeUndefined();
    expect(user.organizationId).toBeUndefined();
  });
});
