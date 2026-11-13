/// <reference types="vitest/globals" />
/**
 * Privacy settings route auth-guard regression coverage.
 *
 * Code-review finding `code-review-ppt-web-core-privacy-route-unprotected`:
 * `settingsRoutes()` mounted `<PrivacySettingsPage />` bare at
 * `/settings/privacy` — no `<ProtectedRoute>` wrapper — while every peer
 * `/settings/*` route (oauth-grants, sessions, members, notifications,
 * integrations, …) is auth-gated. The page loads the signed-in user's own
 * GDPR privacy settings and data-export history from `/api/v1/gdpr/*`, so an
 * unauthenticated visitor to `/settings/privacy` rendered the (self-unguarded)
 * page instead of being sent to /login.
 *
 * These tests mount the real `settingsRoutes()` fragment and assert the
 * auth-only guard: an unauthenticated visitor is redirected to /login, and an
 * authenticated user of any role reaches the page (the peers that are
 * manager-only are role-gated separately — privacy is a per-user page and must
 * stay auth-only, not role-gated). They fail on the pre-fix route table (the
 * privacy page renders for an anonymous visitor) and pass once the route is
 * wrapped in `<ProtectedRoute>`.
 */
import { render, screen } from '@testing-library/react';
import { Suspense } from 'react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { describe, expect, it, vi } from 'vitest';
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

// Swap only the privacy page for a stub so the pre-fix render surfaces a known
// marker without doing any gdprFetch network work; every other lazy route
// wrapper stays real (none of them mount for the /settings/privacy entry).
vi.mock('../lazyRoutes', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../lazyRoutes')>();
  return {
    ...actual,
    PrivacySettingsPage: () => <div data-testid="privacy-page" />,
  };
});

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

// Import after the mocks are registered.
import { settingsRoutes } from './core';

function renderSettingsAt(entry: string) {
  return render(
    <MemoryRouter initialEntries={[entry]}>
      <Suspense fallback={<div>loading…</div>}>
        <Routes>
          <Route path="/login" element={<div>Login Page</div>} />
          {settingsRoutes()}
        </Routes>
      </Suspense>
    </MemoryRouter>
  );
}

describe('settingsRoutes /settings/privacy auth guard', () => {
  it('redirects an unauthenticated user away from /settings/privacy to /login', async () => {
    mockAuth = authFor(null);
    renderSettingsAt('/settings/privacy');
    expect(await screen.findByText('Login Page')).toBeInTheDocument();
    // The privacy page must NOT render for an unauthenticated user.
    expect(screen.queryByTestId('privacy-page')).not.toBeInTheDocument();
  });

  it('lets an authenticated non-manager user (resident) reach the privacy page', async () => {
    // Privacy is a per-user page: auth-only, NOT role-gated. A resident must be
    // able to manage their own GDPR settings.
    mockAuth = authFor('tenant');
    renderSettingsAt('/settings/privacy');
    expect(await screen.findByTestId('privacy-page')).toBeInTheDocument();
    expect(screen.queryByText('Login Page')).not.toBeInTheDocument();
  });
});
