/**
 * ProtectedRoute routing / redirect coverage (#482).
 *
 * The sibling `ProtectedRoute.test.tsx` covers the role-gate decision and
 * `deriveActiveRole` multi-tenant selection. This file covers the component's
 * own routing behaviour, which was previously untested:
 *   - loading state renders the spinner and withholds children
 *   - unauthenticated users are redirected to /login (Navigate)
 *   - the `redirectTo` override targets a custom path
 *   - the current location (path + query) is persisted as the return URL
 *   - the login page itself is never stored as a return URL
 *   - authenticated users pass through when no roles are required
 *   - an empty `requiredRoles` array skips the role gate
 *   - a user holding any one of several accepted roles is allowed
 */
/// <reference types="vitest/globals" />

import type { AuthUser } from '@ppt/api-client';
import { getAndClearReturnUrl } from '@ppt/shared';
import { render, screen } from '@testing-library/react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { AuthContextValue } from '../contexts/AuthContext';

// Mock useAuth so ProtectedRoute can be driven under different auth states
// without standing up the real provider.
vi.mock('../contexts/AuthContext', async () => {
  const actual =
    await vi.importActual<typeof import('../contexts/AuthContext')>('../contexts/AuthContext');
  return {
    ...actual,
    useAuth: () => mockAuth,
  };
});

import { ProtectedRoute } from './ProtectedRoute';

const LOADING_STATE: AuthContextValue = {
  user: null,
  isAuthenticated: false,
  isLoading: true,
  login: vi.fn(),
  loginWithSsoCode: vi.fn(),
  logout: vi.fn(),
  refreshToken: vi.fn(),
  getAccessToken: () => null,
  setUser: vi.fn(),
};

// Mutable mock value injected per-test.
let mockAuth: AuthContextValue = LOADING_STATE;

function unauthenticated(): AuthContextValue {
  return {
    user: null,
    isAuthenticated: false,
    isLoading: false,
    login: vi.fn(),
    loginWithSsoCode: vi.fn(),
    logout: vi.fn(),
    refreshToken: vi.fn(),
    getAccessToken: () => null,
    setUser: vi.fn(),
  };
}

function authedAs(user: AuthUser): AuthContextValue {
  return {
    user,
    isAuthenticated: true,
    isLoading: false,
    login: vi.fn(),
    loginWithSsoCode: vi.fn(),
    logout: vi.fn(),
    refreshToken: vi.fn(),
    getAccessToken: () => null,
    setUser: vi.fn(),
  };
}

/**
 * Render ProtectedRoute at `entry` inside a router that also exposes a
 * recognisable /login (and optional custom) landing route, so a redirect can
 * be asserted by the marker text that ends up on screen.
 */
function renderAt(entry: string, props: { requiredRoles?: string[]; redirectTo?: string } = {}) {
  return render(
    <MemoryRouter initialEntries={[entry]}>
      <Routes>
        <Route path="/login" element={<div>Login Page</div>} />
        <Route path="/denied" element={<div>Custom Redirect Target</div>} />
        <Route
          path="*"
          element={
            <ProtectedRoute requiredRoles={props.requiredRoles} redirectTo={props.redirectTo}>
              <div>Secret content</div>
            </ProtectedRoute>
          }
        />
      </Routes>
    </MemoryRouter>
  );
}

beforeEach(() => {
  // Start each test from a clean return-URL slot.
  if (typeof sessionStorage !== 'undefined') sessionStorage.clear();
});

afterEach(() => {
  if (typeof sessionStorage !== 'undefined') sessionStorage.clear();
});

describe('ProtectedRoute loading state', () => {
  it('renders the auth spinner and withholds children while loading', () => {
    mockAuth = LOADING_STATE;
    renderAt('/dashboard');
    expect(screen.getByLabelText('Checking authentication')).toBeInTheDocument();
    expect(screen.getByText('Loading...')).toBeInTheDocument();
    expect(screen.queryByText('Secret content')).not.toBeInTheDocument();
    expect(screen.queryByText('Login Page')).not.toBeInTheDocument();
  });
});

describe('ProtectedRoute redirect behaviour', () => {
  it('redirects an unauthenticated user to /login', () => {
    mockAuth = unauthenticated();
    renderAt('/dashboard');
    expect(screen.getByText('Login Page')).toBeInTheDocument();
    expect(screen.queryByText('Secret content')).not.toBeInTheDocument();
  });

  it('honours the redirectTo override', () => {
    mockAuth = unauthenticated();
    renderAt('/dashboard', { redirectTo: '/denied' });
    expect(screen.getByText('Custom Redirect Target')).toBeInTheDocument();
    expect(screen.queryByText('Login Page')).not.toBeInTheDocument();
  });

  it('stores the current path and query as the return URL', () => {
    mockAuth = unauthenticated();
    renderAt('/buildings/42?tab=faults');
    expect(getAndClearReturnUrl()).toBe('/buildings/42?tab=faults');
  });

  it('does not store the login page as a return URL', () => {
    mockAuth = unauthenticated();
    renderAt('/login');
    expect(getAndClearReturnUrl()).toBeNull();
  });
});

describe('ProtectedRoute authenticated pass-through', () => {
  it('renders children when authenticated and no roles are required', () => {
    mockAuth = authedAs({ id: 'u1', email: 'x@y.z' });
    renderAt('/dashboard');
    expect(screen.getByText('Secret content')).toBeInTheDocument();
  });

  it('skips the role gate when requiredRoles is an empty array', () => {
    // Role is absent but an empty requiredRoles array must not deny access.
    mockAuth = authedAs({ id: 'u1', email: 'x@y.z' });
    renderAt('/dashboard', { requiredRoles: [] });
    expect(screen.getByText('Secret content')).toBeInTheDocument();
    expect(screen.queryByText('Access Denied')).not.toBeInTheDocument();
  });

  it('allows a user holding any one of several accepted roles', () => {
    mockAuth = authedAs({ id: 'u1', email: 'x@y.z', role: 'technical_manager' });
    renderAt('/dashboard', { requiredRoles: ['manager', 'technical_manager'] });
    expect(screen.getByText('Secret content')).toBeInTheDocument();
  });

  it('denies a user whose role is outside a multi-role requirement', () => {
    mockAuth = authedAs({ id: 'u1', email: 'x@y.z', role: 'tenant' });
    renderAt('/dashboard', { requiredRoles: ['manager', 'technical_manager'] });
    expect(screen.getByText('Access Denied')).toBeInTheDocument();
    expect(screen.queryByText('Secret content')).not.toBeInTheDocument();
  });
});
