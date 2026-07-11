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
 *   - the login page itself (bare and with a ?next= query) is never stored as
 *     a return URL — with ProtectedRoute actually mounted at /login so the
 *     storeReturnUrl guard is exercised rather than bypassed
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

/**
 * Mount ProtectedRoute *itself* at `entry` so its own `storeReturnUrl` guard is
 * actually exercised for the login path.
 *
 * The shared `renderAt` router wires `/login` to a sibling `<Route>`, so an
 * entry of `/login` matches that exact route and ProtectedRoute (on `path="*"`)
 * never mounts — the storeReturnUrl effect never runs and the "don't store
 * /login" assertion passes trivially. Here ProtectedRoute is the element for
 * `path="/login"`, so its effect runs with `location.pathname === '/login'`,
 * driving the `returnUrl !== LOGIN_PATH` guard. A distinct `redirectTo`
 * landing route absorbs the unauthenticated `Navigate` without redirect-looping
 * back onto the path under test.
 */
function renderProtectedAtLogin(entry: string) {
  return render(
    <MemoryRouter initialEntries={[entry]}>
      <Routes>
        <Route path="/landing" element={<div>Landing</div>} />
        <Route
          path="/login"
          element={
            <ProtectedRoute redirectTo="/landing">
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
    // ProtectedRoute is mounted *at* /login here (not on the catch-all), so its
    // storeReturnUrl effect genuinely runs with pathname === '/login'. The
    // `returnUrl !== LOGIN_PATH` guard must reject it — removing that guard
    // flips this test red instead of leaving it trivially green.
    mockAuth = unauthenticated();
    renderProtectedAtLogin('/login');
    expect(screen.getByText('Landing')).toBeInTheDocument();
    expect(getAndClearReturnUrl()).toBeNull();
  });

  it('does not store the login page with a query (?next=) as a return URL', () => {
    // Locks in the second half of the guard:
    // `!returnUrl.startsWith(`${LOGIN_PATH}?`)`. Without it, a login URL that
    // carries a next-hop query would be stored as its own return URL.
    mockAuth = unauthenticated();
    renderProtectedAtLogin('/login?next=/buildings/42');
    expect(screen.getByText('Landing')).toBeInTheDocument();
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
