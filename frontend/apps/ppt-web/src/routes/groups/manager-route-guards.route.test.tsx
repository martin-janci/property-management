/// <reference types="vitest/globals" />
/**
 * Manager-only routes that residents could open by URL (round-3 audit #14,
 * #40): person-month data entry for the whole building and the Airbnb /
 * Booking.com integrations settings. The real <ProtectedRoute> is exercised;
 * only the auth source and the lazy pages are stubbed.
 */
import { render, screen } from '@testing-library/react';
import { Suspense } from 'react';
import { MemoryRouter, Routes } from 'react-router-dom';
import { describe, expect, it, vi } from 'vitest';
import type { AuthContextValue } from '../../contexts/AuthContext';

const authState = vi.hoisted(() => ({ role: 'tenant' as string }));

function auth(): AuthContextValue {
  return {
    isAuthenticated: true,
    isLoading: false,
    user: { id: 'u1', email: 'x@y.z', role: authState.role } as AuthContextValue['user'],
    login: vi.fn(),
    loginWithSsoCode: vi.fn(),
    logout: vi.fn(),
    refreshToken: vi.fn(),
    getAccessToken: () => null,
    setUser: vi.fn(),
  };
}

vi.mock('../../contexts/AuthContext', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../../contexts/AuthContext')>();
  return { ...actual, useAuth: () => auth() };
});
vi.mock('../../contexts', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../../contexts')>();
  return { ...actual, useAuth: () => auth() };
});

vi.mock('../../features/person-months', () => ({
  PersonMonthsPage: () => <div data-testid="pm-page" />,
  BulkEntryPage: () => <div data-testid="pm-bulk" />,
  UnitPersonMonthsPage: () => <div data-testid="pm-unit" />,
  EditPersonMonthPage: () => <div data-testid="pm-edit" />,
}));
vi.mock('../../features/settings', () => ({
  IntegrationsPage: () => <div data-testid="integrations-page" />,
  PortalWebhooksPage: () => null,
}));
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

import { settingsRoutes } from './core';
import { personMonthRoutes } from './person-months';

function renderAt(path: string) {
  return render(
    <MemoryRouter initialEntries={[path]}>
      <Suspense fallback={<div>loading…</div>}>
        <Routes>
          {personMonthRoutes()}
          {settingsRoutes()}
        </Routes>
      </Suspense>
    </MemoryRouter>
  );
}

describe.each(['owner', 'tenant'])('manager-only routes deny "%s"', (role) => {
  it.each([
    '/buildings/b-1/person-months/bulk',
    '/buildings/b-1/units/u-1/person-months/edit/2026/10',
    '/settings/integrations',
  ])('%s', async (path) => {
    authState.role = role;
    renderAt(path);
    expect(await screen.findByText('Access Denied')).toBeInTheDocument();
  });
});

describe('manager reaches the guarded routes', () => {
  it('/settings/integrations', async () => {
    authState.role = 'manager';
    renderAt('/settings/integrations');
    expect(await screen.findByTestId('integrations-page')).toBeInTheDocument();
  });
});
