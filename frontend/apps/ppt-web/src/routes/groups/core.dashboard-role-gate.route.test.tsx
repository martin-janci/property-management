/// <reference types="vitest/globals" />
/**
 * Regression tests for the manager-dashboard route role gate (#2998).
 *
 * Follow-up to PR #2994, which wrapped /dashboard/* in <ProtectedRoute> but
 * left the manager shell **auth-only** — any authenticated user, resident
 * included, landed on the manager dashboard. This suite pins the role gate:
 *
 *   (a) a resident who navigates directly to /dashboard/manager is DENIED
 *       (ProtectedRoute Access-Denied surface), never the manager shell —
 *       this is the IG3 check that fails on `dev` (no gate → shell renders);
 *   (b) every MANAGER_ROLES member reaches the manager shell;
 *   (c) the bare /dashboard redirect fans out by role (resident →
 *       /dashboard/resident, manager → /dashboard/manager) so residents typing
 *       the obvious URL are not caught by the gate;
 *   (d) an unauthenticated hit on /dashboard/manager still redirects to /login.
 *
 * The real <ProtectedRoute> is exercised (only its auth source is mocked), so
 * the test proves the actual route wiring, not a stand-in.
 */
import { render, screen } from '@testing-library/react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { AuthContextValue } from '../../contexts/AuthContext';
import { MANAGER_ROLES } from '../shared';
import { dashboardRoutes } from './core';

// Mutable auth injected per-test, read by BOTH the barrel `useAuth`
// (core.tsx → DashboardIndexRedirect) and the direct AuthContext `useAuth`
// (the real ProtectedRoute).
const authState = vi.hoisted(() => ({
  current: null as unknown as AuthContextValue,
}));

vi.mock('../../contexts/AuthContext', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../../contexts/AuthContext')>();
  return { ...actual, useAuth: () => authState.current };
});

vi.mock('../../contexts', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../../contexts')>();
  return { ...actual, useAuth: () => authState.current };
});

// Identifiable stubs for the (eagerly imported) dashboard shells.
vi.mock('../../features/dashboard', () => ({
  ManagerDashboardPage: () => <div data-testid="manager-shell" />,
  ResidentDashboardPage: () => <div data-testid="resident-shell" />,
}));

// core.tsx pulls a large lazyRoutes barrel at module load; stub it so the
// test stays light. None of these render in the dashboard routes under test.
vi.mock('../lazyRoutes', () => {
  const stub = () => null;
  const names = [
    'AccessibilitySettingsPage',
    'AdvancedNotificationSettingsPage',
    'AiChatPage',
    'AuthCallbackPage',
    'AutomationRulesPage',
    'ChangePasswordPage',
    'CreateRulePage',
    'EditRulePage',
    'EmergencyContactDirectoryPage',
    'ExecutionMonitoringPage',
    'ForbiddenPage',
    'ForgotPasswordPage',
    'LoginPage',
    'NotFoundPage',
    'NotificationSettingsPage',
    'OAuthGrantsPage',
    'PrivacySettingsPage',
    'ProfileEditPage',
    'RegisterPage',
    'ResetPasswordPage',
    'ServerErrorPage',
    'SessionExpiredPage',
    'TemplateLibraryPage',
    'TwoFactorAuthPage',
  ];
  return Object.fromEntries(names.map((name) => [name, stub]));
});

function baseAuth(): Omit<AuthContextValue, 'isAuthenticated' | 'isLoading' | 'user'> {
  return {
    login: vi.fn(),
    loginWithSsoCode: vi.fn(),
    logout: vi.fn(),
    refreshToken: vi.fn(),
    getAccessToken: () => null,
    setUser: vi.fn(),
  };
}

function authedAs(role: string | undefined): AuthContextValue {
  return {
    ...baseAuth(),
    isAuthenticated: true,
    isLoading: false,
    user: { id: 'u1', email: 'x@y.z', ...(role ? { role } : {}) } as AuthContextValue['user'],
  };
}

function unauthenticated(): AuthContextValue {
  return { ...baseAuth(), isAuthenticated: false, isLoading: false, user: null };
}

/** Render the dashboard route group at `entry` with a recognisable /login. */
function renderDashboardAt(entry: string) {
  return render(
    <MemoryRouter initialEntries={[entry]}>
      <Routes>
        <Route path="/login" element={<div data-testid="login-page" />} />
        {dashboardRoutes()}
      </Routes>
    </MemoryRouter>
  );
}

afterEach(() => {
  vi.clearAllMocks();
});

describe('/dashboard/manager role gate (#2998)', () => {
  it('DENIES a resident who navigates directly to the manager shell', () => {
    authState.current = authedAs('resident');
    renderDashboardAt('/dashboard/manager');

    // The regression: on `dev` (auth-only) the manager shell renders for a
    // resident. With the role gate, the Access-Denied surface shows instead.
    expect(screen.getByText('Access Denied')).toBeInTheDocument();
    expect(screen.queryByTestId('manager-shell')).not.toBeInTheDocument();
  });

  it.each([...MANAGER_ROLES])('admits the manager-equivalent role "%s"', (role) => {
    authState.current = authedAs(role);
    renderDashboardAt('/dashboard/manager');

    expect(screen.getByTestId('manager-shell')).toBeInTheDocument();
    expect(screen.queryByText('Access Denied')).not.toBeInTheDocument();
  });

  it.each(['resident', 'owner', 'tenant'])('denies the non-manager role "%s"', (role) => {
    authState.current = authedAs(role);
    renderDashboardAt('/dashboard/manager');

    expect(screen.getByText('Access Denied')).toBeInTheDocument();
    expect(screen.queryByTestId('manager-shell')).not.toBeInTheDocument();
  });

  it('redirects an unauthenticated visitor to /login', () => {
    authState.current = unauthenticated();
    renderDashboardAt('/dashboard/manager');

    expect(screen.getByTestId('login-page')).toBeInTheDocument();
    expect(screen.queryByTestId('manager-shell')).not.toBeInTheDocument();
    expect(screen.queryByText('Access Denied')).not.toBeInTheDocument();
  });
});

describe('bare /dashboard role fan-out (#2998)', () => {
  it('sends a resident to the resident dashboard (not denied)', () => {
    authState.current = authedAs('resident');
    renderDashboardAt('/dashboard');

    expect(screen.getByTestId('resident-shell')).toBeInTheDocument();
    expect(screen.queryByText('Access Denied')).not.toBeInTheDocument();
    expect(screen.queryByTestId('manager-shell')).not.toBeInTheDocument();
  });

  it.each([...MANAGER_ROLES])('sends the manager role "%s" to the manager dashboard', (role) => {
    authState.current = authedAs(role);
    renderDashboardAt('/dashboard');

    expect(screen.getByTestId('manager-shell')).toBeInTheDocument();
    expect(screen.queryByTestId('resident-shell')).not.toBeInTheDocument();
  });
});
