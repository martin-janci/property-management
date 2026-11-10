/**
 * The AI dashboards (tenant sentiment, predictive maintenance) are operator
 * tooling: residents must get the Access-Denied surface, not the dashboards.
 */
import { render, screen } from '@testing-library/react';
import { MemoryRouter, Routes } from 'react-router-dom';
import { describe, expect, it, vi } from 'vitest';
import type { AuthContextValue } from '../../contexts/AuthContext';
import { aiDashboardRoutes } from './ai-dashboards';

const authState = vi.hoisted(() => ({ current: null as unknown as AuthContextValue }));

vi.mock('../../contexts/AuthContext', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../../contexts/AuthContext')>();
  return { ...actual, useAuth: () => authState.current };
});

vi.mock('../../contexts', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../../contexts')>();
  return { ...actual, useAuth: () => authState.current };
});

const query = { data: undefined, isLoading: false };
const mutation = { mutate: vi.fn(), isPending: false, variables: undefined };

vi.mock('../../features/sentiment/hooks', () => ({
  useSentimentDashboard: () => query,
  useSentimentTrends: () => query,
  useSentimentAlerts: () => query,
  useAcknowledgeSentimentAlert: () => mutation,
}));

vi.mock('../../features/predictive-maintenance/hooks', () => ({
  useEquipmentList: () => query,
  useMaintenancePredictions: () => query,
  useAcknowledgeMaintenancePrediction: () => mutation,
}));

vi.mock('../lazyRoutes', () => ({
  SentimentDashboardPage: () => <div data-testid="sentiment-page" />,
  PredictiveMaintenancePage: () => <div data-testid="maintenance-page" />,
}));

function authed(role: string): AuthContextValue {
  return {
    login: vi.fn(),
    loginWithSsoCode: vi.fn(),
    logout: vi.fn(),
    refreshToken: vi.fn(),
    getAccessToken: () => null,
    setUser: vi.fn(),
    isAuthenticated: true,
    isLoading: false,
    user: { id: 'u1', email: 'x@y.z', role, organizationId: 'org-1' } as AuthContextValue['user'],
  };
}

function renderAt(entry: string) {
  return render(
    <MemoryRouter initialEntries={[entry]}>
      <Routes>{aiDashboardRoutes()}</Routes>
    </MemoryRouter>
  );
}

describe('AI dashboard routes role gate', () => {
  it.each([
    ['tenant', '/ai/sentiment', 'sentiment-page'],
    ['owner', '/ai/predictive-maintenance', 'maintenance-page'],
  ])('denies the %s role on %s', (role, path, testId) => {
    authState.current = authed(role);
    renderAt(path);
    expect(screen.queryByTestId(testId)).toBeNull();
  });

  it.each([
    ['/ai/sentiment', 'sentiment-page'],
    ['/ai/predictive-maintenance', 'maintenance-page'],
  ])('admits a manager on %s', (path, testId) => {
    authState.current = authed('manager');
    renderAt(path);
    expect(screen.getByTestId(testId)).toBeTruthy();
  });
});
