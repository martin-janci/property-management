/**
 * Notifications route group — NotificationTriggersPageRoute wiring tests
 * (follow-up #2325). Verifies the route wrapper behaviour that the presentational
 * page tests can't reach:
 *   (a) 403 → forbidden notice rendered
 *   (b) 401 → redirect to /login (per-user page: expired session, not "no permission")
 *   (c) toggling a channel builds the correct PUT patch (push → pushEnabled)
 *   (d) a failed PUT surfaces an error toast
 *
 * Uses MemoryRouter + Routes + Suspense so the lazy-loaded page renders exactly
 * as in production. `@ppt/api-client` hooks are stubbed; `ProtectedRoute` is a
 * pass-through so the wrapper's own auth redirect doesn't mask the 401 test.
 */

/// <reference types="vitest/globals" />
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen } from '@testing-library/react';
import { Suspense } from 'react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';

// ── i18n: return the defaultValue verbatim ──
vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, opts?: { defaultValue?: string }) => opts?.defaultValue ?? key,
  }),
}));

// ── toast + pass-through ProtectedRoute ──
const mockShowToast = vi.fn();
vi.mock('../../components', () => ({
  useToast: () => ({ showToast: mockShowToast }),
  ProtectedRoute: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));

// ── api-client hook stubs ──
const mockUseTriggers = vi.fn();
const mockUpdateMutate = vi.fn();
const mockResetMutate = vi.fn();

vi.mock('@ppt/api-client', () => ({
  useNotificationTriggers: () => mockUseTriggers(),
  useUpdateNotificationTrigger: () => ({
    mutate: mockUpdateMutate,
    isPending: false,
    variables: undefined,
  }),
  useResetNotificationTriggers: () => ({ mutate: mockResetMutate, isPending: false }),
  notificationTriggerKeys: {
    all: ['notification-triggers'],
    events: () => ['notification-triggers', 'events'],
  },
}));

// Import the route group AFTER mocks are in place.
import { notificationAnalyticsRoutes } from './notifications';

const noopRefetch = vi.fn().mockResolvedValue(undefined);

function makeTriggers() {
  return {
    preferences: [
      {
        eventType: 'fault_created',
        category: 'fault',
        displayName: 'Fault created',
        isPriority: false,
        pushEnabled: false,
        emailEnabled: true,
        inAppEnabled: true,
      },
    ],
    categories: [{ category: 'fault', displayName: 'Fault', totalEvents: 1, enabledEvents: 1 }],
  };
}

function renderTriggersRoute() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <MemoryRouter initialEntries={['/notifications/triggers']}>
        <Suspense fallback={<div>loading…</div>}>
          <Routes>
            {notificationAnalyticsRoutes()}
            <Route path="/login" element={<div>LOGIN PAGE</div>} />
          </Routes>
        </Suspense>
      </MemoryRouter>
    </QueryClientProvider>
  );
}

describe('NotificationTriggersPageRoute wiring (#2325)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('(a) maps 403 to the forbidden notice', async () => {
    mockUseTriggers.mockReturnValue({
      data: undefined,
      isLoading: false,
      error: Object.assign(new Error('forbidden'), { status: 403 }),
      refetch: noopRefetch,
    });

    renderTriggersRoute();

    expect(
      await screen.findByText(
        'You do not have permission to manage notification triggers.',
        {},
        { timeout: 3000 }
      )
    ).toBeInTheDocument();
  });

  it('(b) redirects a 401 to /login instead of showing "no permission"', async () => {
    mockUseTriggers.mockReturnValue({
      data: undefined,
      isLoading: false,
      error: Object.assign(new Error('unauthorized'), { status: 401 }),
      refetch: noopRefetch,
    });

    renderTriggersRoute();

    expect(await screen.findByText('LOGIN PAGE', {}, { timeout: 3000 })).toBeInTheDocument();
    expect(
      screen.queryByText('You do not have permission to manage notification triggers.')
    ).not.toBeInTheDocument();
  });

  it('(c) builds a push→pushEnabled patch when a channel is toggled', async () => {
    mockUseTriggers.mockReturnValue({
      data: makeTriggers(),
      isLoading: false,
      error: null,
      refetch: noopRefetch,
    });

    renderTriggersRoute();

    const pushBox = await screen.findByLabelText('Fault created — Push', {}, { timeout: 3000 });
    fireEvent.click(pushBox);

    expect(mockUpdateMutate).toHaveBeenCalledTimes(1);
    expect(mockUpdateMutate.mock.calls[0][0]).toEqual({
      eventType: 'fault_created',
      patch: { pushEnabled: true },
    });
  });

  it('(d) surfaces an error toast when the PUT fails', async () => {
    mockUpdateMutate.mockImplementation((_vars, opts?: { onError?: (e: Error) => void }) => {
      opts?.onError?.(new Error('boom'));
    });
    mockUseTriggers.mockReturnValue({
      data: makeTriggers(),
      isLoading: false,
      error: null,
      refetch: noopRefetch,
    });

    renderTriggersRoute();

    const pushBox = await screen.findByLabelText('Fault created — Push', {}, { timeout: 3000 });
    fireEvent.click(pushBox);

    expect(mockShowToast).toHaveBeenCalledTimes(1);
    expect(mockShowToast.mock.calls[0][0]).toMatchObject({ type: 'error' });
  });
});
