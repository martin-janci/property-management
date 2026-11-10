/**
 * Regression: acknowledging an AI insight (tenant-sentiment alert or
 * predictive-maintenance prediction) must surface an error toast when the
 * mutation fails — otherwise the operator clicks "Acknowledge", the request
 * 500s, and nothing tells them it didn't take. Mirrors the error-surfacing
 * convention in routes/groups/iot.tsx.
 */
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter, Routes } from 'react-router-dom';
import { describe, expect, it, vi } from 'vitest';
import type { AuthContextValue } from '../../contexts/AuthContext';
import { aiDashboardRoutes } from './ai-dashboards';

const authState = vi.hoisted(() => ({ current: null as unknown as AuthContextValue }));
const showToast = vi.hoisted(() => vi.fn());
const ackMutation = vi.hoisted(() => ({
  mutate: vi.fn(),
  mutateAsync: vi.fn().mockRejectedValue(new Error('boom')),
  isPending: false,
  variables: undefined as string | undefined,
}));

vi.mock('../../contexts/AuthContext', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../../contexts/AuthContext')>();
  return { ...actual, useAuth: () => authState.current };
});

vi.mock('../../contexts', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../../contexts')>();
  return { ...actual, useAuth: () => authState.current };
});

vi.mock('../../components', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../../components')>();
  return { ...actual, useToast: () => ({ showToast, removeToast: vi.fn() }) };
});

const query = { data: undefined, isLoading: false };

vi.mock('../../features/sentiment/hooks', () => ({
  useSentimentDashboard: () => query,
  useSentimentTrends: () => query,
  useSentimentAlerts: () => query,
  useAcknowledgeSentimentAlert: () => ackMutation,
}));

vi.mock('../../features/predictive-maintenance/hooks', () => ({
  useEquipmentList: () => query,
  useMaintenancePredictions: () => query,
  useAcknowledgeMaintenancePrediction: () => ackMutation,
}));

vi.mock('../lazyRoutes', () => ({
  SentimentDashboardPage: ({ onAcknowledge }: { onAcknowledge: (id: string) => void }) => (
    <button data-testid="ack-sentiment" type="button" onClick={() => onAcknowledge('alert-1')}>
      ack
    </button>
  ),
  PredictiveMaintenancePage: ({
    onAcknowledgePrediction,
  }: {
    onAcknowledgePrediction: (id: string) => void;
  }) => (
    <button
      data-testid="ack-maintenance"
      type="button"
      onClick={() => onAcknowledgePrediction('pred-1')}
    >
      ack
    </button>
  ),
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

describe('AI dashboard acknowledge error surfacing', () => {
  it.each([
    ['/ai/sentiment', 'ack-sentiment'],
    ['/ai/predictive-maintenance', 'ack-maintenance'],
  ])('shows an error toast when acknowledge fails on %s', async (path, testId) => {
    showToast.mockClear();
    authState.current = authed('manager');
    renderAt(path);

    fireEvent.click(screen.getByTestId(testId));

    await waitFor(() =>
      expect(showToast).toHaveBeenCalledWith(expect.objectContaining({ type: 'error' }))
    );
  });
});
