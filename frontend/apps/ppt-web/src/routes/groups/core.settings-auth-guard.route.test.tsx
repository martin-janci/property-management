/// <reference types="vitest/globals" />
/**
 * Core `/settings/*` authed-root auth-guard regression coverage.
 *
 * Code-review finding `code-review-ppt-web-core-authed-roots-adhoc-guard-gap`:
 * several authenticated root routes in `core.tsx` bypassed the shared/central
 * `<ProtectedRoute>` guard and relied on an ad-hoc inline guard instead:
 *
 *   - `/settings/password`   (ChangePasswordPage)
 *   - `/settings/two-factor` (TwoFactorAuthPage)
 *   - `/settings/profile`    (ProfileEditPage)
 *       → each had an inline `if (!isAuthenticated) <Navigate to="/login" />`
 *         that, unlike `<ProtectedRoute>`, never captured the return URL via
 *         `@ppt/shared` `setReturnUrl` — so a login triggered from one of these
 *         pages dropped the user on the default dashboard instead of back here.
 *   - `/settings/accessibility` (AccessibilitySettingsPage)
 *       → had NO guard at all: an anonymous visitor rendered the page.
 *
 * Every peer `/settings/*` route (privacy, oauth-grants, sessions, members,
 * notifications, …) is wrapped in `<ProtectedRoute>`. These tests assert the
 * four stragglers now go through the same shared guard: an unauthenticated
 * visitor is redirected to /login AND the attempted path is captured via
 * `setReturnUrl`, and an authenticated user reaches the page.
 *
 * The pages are stubbed (like `core.privacy-auth.route.test.tsx`) so the unit
 * under test is purely the route-table guard wiring. Pre-fix the stub renders
 * for an anonymous visitor and `setReturnUrl` is never called — both tests
 * fail. Post-fix the shared `<ProtectedRoute>` redirects and captures the
 * return URL — both pass.
 */
import { render, screen } from '@testing-library/react';
import { Suspense } from 'react';
import { MemoryRouter, Routes } from 'react-router-dom';
import { describe, expect, it, type Mock, vi } from 'vitest';
import type { AuthContextValue } from '../../contexts/AuthContext';

// Mutable auth mock injected per-test. ProtectedRoute reads useAuth from
// ../../contexts/AuthContext; core.tsx reads it from the ../../contexts barrel
// which re-exports the same module, so mocking the concrete module covers both.
let mockAuth: AuthContextValue;

vi.mock('../../contexts/AuthContext', async () => {
  const actual = await vi.importActual<typeof import('../../contexts/AuthContext')>(
    '../../contexts/AuthContext'
  );
  return { ...actual, useAuth: () => mockAuth };
});

// Spy on the return-URL capture helper ProtectedRoute calls on redirect. The
// ad-hoc inline guards these routes used to rely on never called it.
vi.mock('@ppt/shared', async () => {
  const actual = await vi.importActual<typeof import('@ppt/shared')>('@ppt/shared');
  return { ...actual, setReturnUrl: vi.fn() };
});

// i18n pass-through so ProtectedRoute's spinner/labels don't need a provider.
vi.mock('react-i18next', async () => {
  const actual = await vi.importActual<typeof import('react-i18next')>('react-i18next');
  return {
    ...actual,
    useTranslation: () => ({
      t: (key: string, opts?: { defaultValue?: string }) => opts?.defaultValue ?? key,
    }),
  };
});

// Stub the pages under test (plus the /login target) so the pre-fix render
// surfaces known markers without any page-level network/hook work; every other
// lazy route wrapper stays real (none of them mount for these entries).
vi.mock('../lazyRoutes', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../lazyRoutes')>();
  return {
    ...actual,
    LoginPage: () => <div>Login Page</div>,
    ChangePasswordPage: () => <div data-testid="settings-page" />,
    TwoFactorAuthPage: () => <div data-testid="settings-page" />,
    ProfileEditPage: () => <div data-testid="settings-page" />,
    AccessibilitySettingsPage: () => <div data-testid="settings-page" />,
  };
});

import { setReturnUrl } from '@ppt/shared';
// Import after the mocks are registered.
import { authRoutes, settingsRoutes } from './core';

function authFor(role: string | null): AuthContextValue {
  return {
    user: role ? ({ id: 'u1', email: 'x@y.z', role } as AuthContextValue['user']) : null,
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

function renderCoreAt(entry: string) {
  return render(
    <MemoryRouter initialEntries={[entry]}>
      <Suspense fallback={<div>loading…</div>}>
        <Routes>
          {authRoutes()}
          {settingsRoutes()}
        </Routes>
      </Suspense>
    </MemoryRouter>
  );
}

// The authed `/settings/*` roots that must route through the shared guard.
const AUTHED_SETTINGS_ROUTES = [
  '/settings/password',
  '/settings/two-factor',
  '/settings/profile',
  '/settings/accessibility',
] as const;

describe('core authed /settings/* routes go through the shared ProtectedRoute guard', () => {
  it.each(AUTHED_SETTINGS_ROUTES)(
    'redirects an unauthenticated visitor to /login and captures the return URL for %s',
    async (path) => {
      vi.clearAllMocks();
      mockAuth = authFor(null);
      renderCoreAt(path);

      // Redirected to the login surface; the settings page must NOT render.
      expect(await screen.findByText('Login Page')).toBeInTheDocument();
      expect(screen.queryByTestId('settings-page')).not.toBeInTheDocument();

      // Shared-guard contract the ad-hoc inline guards skipped: the attempted
      // path is stored so login returns the user here, not the dashboard.
      expect(setReturnUrl as unknown as Mock).toHaveBeenCalledWith(path);
    }
  );

  it.each(AUTHED_SETTINGS_ROUTES)('lets an authenticated user reach %s', async (path) => {
    vi.clearAllMocks();
    // Per-user account pages: auth-only, not role-gated. A resident reaches them.
    mockAuth = authFor('tenant');
    renderCoreAt(path);

    expect(await screen.findByTestId('settings-page')).toBeInTheDocument();
    expect(screen.queryByText('Login Page')).not.toBeInTheDocument();
    expect(setReturnUrl as unknown as Mock).not.toHaveBeenCalled();
  });
});
