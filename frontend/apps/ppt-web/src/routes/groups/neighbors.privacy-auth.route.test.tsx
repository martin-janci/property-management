/// <reference types="vitest/globals" />
/**
 * Neighbors privacy-settings route auth-guard regression coverage.
 *
 * Code-review finding `code-review-ppt-web-core-privacy-route-unguarded`:
 * auth gating in ppt-web is strictly per-route via `<ProtectedRoute>` — there
 * is NO global guard around `<AppRoutes />`. The per-user privacy surfaces must
 * therefore each carry their own wrapper. The `/settings/privacy` GDPR page was
 * gated in #3482 (`core.privacy-auth.route.test.tsx`) and the neighbor route
 * group was gated in #3486 (`mgmt-routes-auth-guard.route.test.tsx`), but that
 * management-group test only exercises the `/neighbors` directory entry — the
 * sibling `/neighbors/privacy` form (the per-user privacy settings page wired to
 * GET/PUT `/api/v1/users/me/privacy`) had no dedicated guard coverage, so its
 * `<ProtectedRoute>` wrapper could silently regress without any test failing and
 * expose a signed-out visitor's path to the privacy form.
 *
 * These tests mount the real `neighborRoutes()` fragment and assert the
 * auth-only guard for `/neighbors/privacy` specifically: an unauthenticated
 * visitor is redirected to /login and the privacy form does NOT render, while an
 * authenticated user of any role reaches it (privacy is a per-user, resident-
 * accessible page — auth-only, NOT role-gated). Removing the `<ProtectedRoute>`
 * wrapper around the `/neighbors/privacy` route makes the first case fail (the
 * form renders for an anonymous visitor).
 */
import { render, screen } from '@testing-library/react';
import { Suspense } from 'react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { describe, expect, it, vi } from 'vitest';
import type { AuthContextValue } from '../../contexts/AuthContext';

// Mutable auth mock injected per-test. ProtectedRoute reads useAuth from
// ../../contexts/AuthContext; the group file reads it from the ../../contexts
// barrel which re-exports the same module, so mock both to the same source.
let mockAuth: AuthContextValue;

vi.mock('../../contexts/AuthContext', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../../contexts/AuthContext')>();
  return { ...actual, useAuth: () => mockAuth };
});
vi.mock('../../contexts', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../../contexts')>();
  return { ...actual, useAuth: () => mockAuth };
});

// i18n pass-through so ProtectedRoute's spinner/labels don't need a provider.
vi.mock('react-i18next', async (importOriginal) => {
  const actual = await importOriginal<typeof import('react-i18next')>();
  return {
    ...actual,
    useTranslation: () => ({
      t: (key: string, opts?: { defaultValue?: string }) => opts?.defaultValue ?? key,
    }),
  };
});

// Stub the lazy privacy page with a marker so the (pre-fix) unguarded render
// surfaces a known testid without any real data wiring; the real ProtectedRoute
// wrapper is what we exercise. Keep every other export real.
vi.mock('../lazyRoutes', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../lazyRoutes')>();
  return {
    ...actual,
    NeighborsPrivacySettingsPage: () => <div data-testid="neighbors-privacy-page" />,
  };
});

// useToast needs a ToastProvider; override it so the route wrapper that calls it
// renders without one. Keep ProtectedRoute (same barrel) real.
vi.mock('../../components', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../../components')>();
  return { ...actual, useToast: () => ({ showToast: vi.fn() }) };
});

// Network-free stubs for the data hooks the route wrapper calls, so the pre-fix
// (unguarded) render completes without throwing.
vi.mock('../../features/neighbors', () => ({
  useNeighbors: () => ({ neighbors: [], isLoading: false, error: null }),
  usePrivacySettings: () => ({
    settings: undefined,
    isLoading: false,
    isSubmitting: false,
    error: null,
    updateSettings: vi.fn(),
  }),
}));

function authFor(role: string | null): AuthContextValue {
  return {
    user: role
      ? ({ id: 'u1', email: 'x@y.z', role, organizationId: 'org-1' } as AuthContextValue['user'])
      : null,
    isAuthenticated: role !== null,
    isLoading: false,
    login: vi.fn(),
    loginWithSsoCode: vi.fn(),
    logout: vi.fn(),
    refreshToken: vi.fn(),
    getAccessToken: () => null,
    setUser: vi.fn(),
  };
}

// Import after the mocks are registered.
import { neighborRoutes } from './neighbors';

function renderAt(entry: string) {
  return render(
    <MemoryRouter initialEntries={[entry]}>
      <Suspense fallback={<div>loading…</div>}>
        <Routes>
          <Route path="/login" element={<div>Login Page</div>} />
          {neighborRoutes()}
        </Routes>
      </Suspense>
    </MemoryRouter>
  );
}

describe('/neighbors/privacy auth guard (unauthenticated)', () => {
  it('redirects an anonymous visitor to /login and does not render the privacy form', async () => {
    mockAuth = authFor(null);
    renderAt('/neighbors/privacy');
    expect(await screen.findByText('Login Page')).toBeInTheDocument();
    // The privacy settings form must NOT render for an unauthenticated user.
    expect(screen.queryByTestId('neighbors-privacy-page')).not.toBeInTheDocument();
  });
});

describe('/neighbors/privacy auth guard (authenticated, any role)', () => {
  // Privacy settings are a per-user, resident-accessible page: auth-only, NOT
  // role-gated. A signed-in resident must still reach the form.
  it('lets an authenticated resident reach the privacy form', async () => {
    mockAuth = authFor('tenant');
    renderAt('/neighbors/privacy');
    expect(await screen.findByTestId('neighbors-privacy-page')).toBeInTheDocument();
    expect(screen.queryByText('Login Page')).not.toBeInTheDocument();
  });
});
