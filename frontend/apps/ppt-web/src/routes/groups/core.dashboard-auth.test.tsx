/// <reference types="vitest/globals" />
/**
 * Dashboard route auth-guard regression coverage.
 *
 * Before the fix, `dashboardRoutes()` mounted `<ManagerDashboardPage />` and
 * `<ResidentDashboardPage />` bare — no `<ProtectedRoute>` wrapper — and the
 * pages did not self-guard, so an unauthenticated visitor to `/dashboard/manager`
 * or `/dashboard/resident` rendered the dashboard shell instead of being sent
 * to /login (the surrounding comment even claimed the pages handled auth).
 *
 * These tests mount the real `dashboardRoutes()` fragment with an
 * unauthenticated `useAuth` and assert the redirect to /login. They fail on the
 * pre-fix route table (dashboard content renders) and pass once each route is
 * wrapped in `<ProtectedRoute>`.
 */
import { render, screen } from '@testing-library/react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { describe, expect, it, vi } from 'vitest';
import type { AuthContextValue } from '../../contexts/AuthContext';

// Mutable auth mock injected per-test. ProtectedRoute reads useAuth from
// ../../contexts/AuthContext; core.tsx reads it from the ../../contexts barrel
// which re-exports the same module, so mocking the concrete module covers both.
let mockAuth: AuthContextValue;

vi.mock('../../contexts/AuthContext', async () => {
  const actual =
    await vi.importActual<typeof import('../../contexts/AuthContext')>('../../contexts/AuthContext');
  return { ...actual, useAuth: () => mockAuth };
});

// i18n pass-through so ProtectedRoute's spinner/labels don't need a provider.
// Preserve the real module (i18n/index.ts needs initReactI18next) and only
// override useTranslation.
vi.mock('react-i18next', async () => {
  const actual = await vi.importActual<typeof import('react-i18next')>('react-i18next');
  return {
    ...actual,
    useTranslation: () => ({
      t: (key: string, opts?: { defaultValue?: string }) => opts?.defaultValue ?? key,
    }),
  };
});

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

// Import after the mocks are registered.
import { dashboardRoutes } from './core';

function renderDashboardAt(entry: string) {
  return render(
    <MemoryRouter initialEntries={[entry]}>
      <Routes>
        <Route path="/login" element={<div>Login Page</div>} />
        {dashboardRoutes()}
      </Routes>
    </MemoryRouter>
  );
}

describe('dashboardRoutes auth guard', () => {
  it('redirects an unauthenticated user away from /dashboard/manager to /login', () => {
    mockAuth = unauthenticated();
    renderDashboardAt('/dashboard/manager');
    expect(screen.getByText('Login Page')).toBeInTheDocument();
    // The manager dashboard heading must NOT render for an unauthenticated user.
    expect(screen.queryByText('dashboard.managerDashboard')).not.toBeInTheDocument();
  });

  it('redirects an unauthenticated user away from /dashboard/resident to /login', () => {
    mockAuth = unauthenticated();
    renderDashboardAt('/dashboard/resident');
    expect(screen.getByText('Login Page')).toBeInTheDocument();
    expect(screen.queryByText('dashboard.residentDashboard')).not.toBeInTheDocument();
  });
});
