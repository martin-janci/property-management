/**
 * Regression test for the Home "Open dashboard" CTA role routing
 * (code-review finding `code-review-ppt-web-core-home-cta-role-drift`).
 *
 * The CTA used to branch on the literal `user?.role === 'manager'`, so every
 * manager-equivalent role that is NOT the string 'manager' — `org_admin`,
 * `property_manager`, `technical_manager`, `super_admin` — was misrouted to the
 * resident dashboard. It now branches on `isManagerRole(user?.role)`, the same
 * single source of truth (`MANAGER_ROLES`) that gates the manager routes via
 * `<ProtectedRoute requiredRoles={[...MANAGER_ROLES]}>`, so the CTA target and
 * the route guard can no longer drift apart.
 */
import { fireEvent, render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { MANAGER_ROLES } from '../shared';
import { Home } from './core';

const mockNavigate = vi.fn();
let currentRole: string | undefined = 'manager';

// Keep react-router real except for useNavigate, which we assert against.
vi.mock('react-router-dom', async (importOriginal) => {
  const actual = await importOriginal<typeof import('react-router-dom')>();
  return { ...actual, useNavigate: () => mockNavigate };
});

// Authenticated user whose role varies per test.
vi.mock('../../contexts', () => ({
  useAuth: () => ({ isAuthenticated: true, user: { role: currentRole } }),
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

function renderHomeAsRole(role: string | undefined) {
  currentRole = role;
  mockNavigate.mockClear();
  render(
    <MemoryRouter>
      <Home />
    </MemoryRouter>
  );
  fireEvent.click(screen.getByRole('button', { name: /dashboard/i }));
}

describe('Home "Open dashboard" CTA routes by manager-role membership', () => {
  afterEach(() => {
    vi.clearAllMocks();
  });

  it.each([...MANAGER_ROLES])(
    'routes the manager-equivalent role "%s" to the manager dashboard',
    (role) => {
      renderHomeAsRole(role);
      expect(mockNavigate).toHaveBeenCalledWith('/dashboard/manager');
    }
  );

  it.each(['resident', 'owner', undefined])(
    'routes the non-manager role "%s" to the resident dashboard',
    (role) => {
      renderHomeAsRole(role);
      expect(mockNavigate).toHaveBeenCalledWith('/dashboard/resident');
    }
  );
});
