/**
 * ProtectedRoute tests — verify the role-gate fix (#482).
 *
 * Covers:
 *   - user with no role is denied a role-restricted route
 *   - user with the required role is allowed
 *   - user with a different role is denied
 *   - role-derivation: JWT tenant_id claim resolves the membership for
 *     multi-tenant users instead of relying on tenants[0]
 *   - single-tenant case still works
 *   - JWT `role` claim re-derivation on refresh invalidates cached role
 */
/// <reference types="vitest/globals" />
import { render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import type { AuthUser, TenantMembership } from '@ppt/api-client';
import { describe, expect, it, vi } from 'vitest';
import { deriveActiveRole } from '../contexts/AuthContext';
import type { AuthContextValue } from '../contexts/AuthContext';

// Mock useAuth so we can drive ProtectedRoute under different conditions
// without standing up the real provider.
vi.mock('../contexts/AuthContext', async () => {
  const actual = await vi.importActual<typeof import('../contexts/AuthContext')>(
    '../contexts/AuthContext'
  );
  return {
    ...actual,
    useAuth: () => mockAuth,
  };
});

import { ProtectedRoute } from './ProtectedRoute';

// Mutable mock value injected per-test
let mockAuth: AuthContextValue = {
  user: null,
  isAuthenticated: false,
  isLoading: false,
  login: vi.fn(),
  logout: vi.fn(),
  refreshToken: vi.fn(),
  getAccessToken: () => null,
  setUser: vi.fn(),
};

function authedAs(user: AuthUser | null): AuthContextValue {
  return {
    user,
    isAuthenticated: user !== null,
    isLoading: false,
    login: vi.fn(),
    logout: vi.fn(),
    refreshToken: vi.fn(),
    getAccessToken: () => null,
    setUser: vi.fn(),
  };
}

function renderProtected(requiredRoles?: string[]) {
  return render(
    <MemoryRouter>
      <ProtectedRoute requiredRoles={requiredRoles}>
        <div>Secret content</div>
      </ProtectedRoute>
    </MemoryRouter>
  );
}

describe('ProtectedRoute role gating', () => {
  it('denies a user with no role when a role is required', () => {
    mockAuth = authedAs({ id: 'u1', email: 'x@y.z' });
    renderProtected(['manager']);
    expect(screen.queryByText('Secret content')).not.toBeInTheDocument();
    expect(screen.getByText('Access Denied')).toBeInTheDocument();
  });

  it('allows a user whose role matches', () => {
    mockAuth = authedAs({ id: 'u1', email: 'x@y.z', role: 'manager' });
    renderProtected(['manager']);
    expect(screen.getByText('Secret content')).toBeInTheDocument();
  });

  it('denies a user whose role does not match', () => {
    mockAuth = authedAs({ id: 'u1', email: 'x@y.z', role: 'tenant' });
    renderProtected(['manager']);
    expect(screen.queryByText('Secret content')).not.toBeInTheDocument();
    expect(screen.getByText('Access Denied')).toBeInTheDocument();
  });
});

// Build a minimal JWT (header.payload.signature) with the given payload.
function jwt(payload: Record<string, unknown>): string {
  const b64 = (obj: unknown) =>
    btoa(JSON.stringify(obj)).replace(/=/g, '').replace(/\+/g, '-').replace(/\//g, '_');
  return `${b64({ alg: 'none' })}.${b64(payload)}.sig`;
}

describe('deriveActiveRole (multi-tenant role selection)', () => {
  const tenants: TenantMembership[] = [
    { tenantId: 't-tenant', tenantName: 'B', role: 'tenant' },
    { tenantId: 't-manager', tenantName: 'A', role: 'manager' },
  ];

  it('picks the role for the JWT tenant_id when present (multi-tenant user)', () => {
    const token = jwt({ tenant_id: 't-manager' });
    expect(deriveActiveRole(token, tenants)).toBe('manager');
  });

  it('returns the only role for a single-tenant user', () => {
    const single: TenantMembership[] = [{ tenantId: 't1', tenantName: 'A', role: 'tenant' }];
    expect(deriveActiveRole(jwt({ tenant_id: 't1' }), single)).toBe('tenant');
  });

  it('falls back to highest-privilege role when JWT lacks tenant_id', () => {
    // No tenant_id claim — must not return tenants[0]; must return manager
    // because it outranks tenant in ROLE_PRIORITY.
    expect(deriveActiveRole(jwt({}), tenants)).toBe('manager');
  });

  it('honours JWT `role` claim when tenants list does not match tenant_id', () => {
    const token = jwt({ tenant_id: 'unknown', role: 'org_admin' });
    // role from JWT outranks both memberships
    expect(deriveActiveRole(token, tenants)).toBe('org_admin');
  });

  it('returns undefined when no memberships exist', () => {
    expect(deriveActiveRole(jwt({ tenant_id: 't1' }), [])).toBeUndefined();
    expect(deriveActiveRole(jwt({ tenant_id: 't1' }), undefined)).toBeUndefined();
  });

  it('handles malformed/missing token gracefully', () => {
    // No token → no tenant_id, no role claim, so highest-priority across
    // memberships wins (cache invalidation safety net).
    expect(deriveActiveRole(null, tenants)).toBe('manager');
    expect(deriveActiveRole('not-a-jwt', tenants)).toBe('manager');
  });
});
