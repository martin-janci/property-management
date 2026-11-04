/**
 * `/` for signed-in users (round-3 audit #15).
 *
 * `/` is the public landing page. It used to greet signed-in users with a
 * generic "Vitajte" page and an "Open dashboard" button; it now sends them
 * straight to their role's dashboard. The target comes from `isManagerRole`
 * (the same `MANAGER_ROLES` list that gates the manager routes), so every
 * manager-equivalent role lands on the manager dashboard and everyone else on
 * the resident dashboard. Anonymous visitors still get the landing page.
 */
import { render, screen } from '@testing-library/react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { describe, expect, it, vi } from 'vitest';
import { MANAGER_ROLES } from '../shared';
import { Home } from './core';

let currentAuth: { isAuthenticated: boolean; user: { role?: string } | null } = {
  isAuthenticated: true,
  user: { role: 'manager' },
};

vi.mock('../../contexts', () => ({
  useAuth: () => currentAuth,
}));

// Home only renders its own markup; stub the heavy sibling imports that
// core.tsx pulls in at module load so the test stays light. These stubs never
// render (Home is mounted on its own), so returning null keeps the mock
// factories free of JSX — which vitest hoists above the module imports.
vi.mock('../../components', () => ({ ProtectedRoute: () => null }));
vi.mock('../../features/dashboard', () => ({
  ManagerDashboardPage: () => null,
  ResidentDashboardPage: () => null,
}));
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

function renderHomeAt(auth: typeof currentAuth) {
  currentAuth = auth;
  render(
    <MemoryRouter initialEntries={['/']}>
      <Routes>
        <Route path="/" element={<Home />} />
        <Route path="/dashboard/manager" element={<div data-testid="manager-dashboard" />} />
        <Route path="/dashboard/resident" element={<div data-testid="resident-dashboard" />} />
      </Routes>
    </MemoryRouter>
  );
}

describe('`/` sends signed-in users to their dashboard', () => {
  it.each([...MANAGER_ROLES])('redirects the manager-equivalent role "%s"', (role) => {
    renderHomeAt({ isAuthenticated: true, user: { role } });
    expect(screen.getByTestId('manager-dashboard')).toBeInTheDocument();
  });

  it.each(['owner', 'tenant', 'resident', undefined])(
    'redirects the non-manager role "%s" to the resident dashboard',
    (role) => {
      renderHomeAt({ isAuthenticated: true, user: role ? { role } : {} });
      expect(screen.getByTestId('resident-dashboard')).toBeInTheDocument();
    }
  );

  it('keeps the landing page for anonymous visitors', () => {
    renderHomeAt({ isAuthenticated: false, user: null });
    expect(screen.queryByTestId('manager-dashboard')).not.toBeInTheDocument();
    expect(screen.queryByTestId('resident-dashboard')).not.toBeInTheDocument();
    expect(screen.getByRole('heading', { level: 1 })).toBeInTheDocument();
  });
});
