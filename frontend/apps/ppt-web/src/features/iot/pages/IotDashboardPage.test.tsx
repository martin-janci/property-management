/// <reference types="vitest/globals" />
/**
 * Regression test for the IoT dashboard offline state (Epic 14 — FR72).
 *
 * `useIotWebSocket` exposes `hasGivenUp` — true once the live-telemetry socket
 * has exhausted its reconnect budget and permanently stopped retrying. That
 * flag must reach the UI: the route passes it in as `isOffline`, and the
 * dashboard must then surface an explicit "disconnected / offline" indication
 * instead of leaving the stale snapshot looking live. Before the fix the flag
 * was destructured away in the route and had no UI path at all.
 *
 * Presentational component — no network/context mocks needed; i18n is the real
 * English bundle from the shared test setup.
 */

import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { IotDashboardPage, type IotDashboardPageProps } from './IotDashboardPage';

function renderPage(overrides: Partial<IotDashboardPageProps> = {}) {
  const props: IotDashboardPageProps = {
    dashboard: undefined,
    dashboardLoading: false,
    sensors: [],
    sensorsLoading: false,
    selectedSensorId: null,
    selectedSensor: null,
    readings: [],
    readingsLoading: false,
    alerts: [],
    pendingAlertId: null,
    onSelectSensor: vi.fn(),
    onAcknowledgeAlert: vi.fn(),
    onResolveAlert: vi.fn(),
    ...overrides,
  };
  return { props, ...render(<IotDashboardPage {...props} />) };
}

describe('IotDashboardPage offline state', () => {
  it('surfaces a disconnected banner when the live socket has given up (isOffline)', () => {
    renderPage({ isOffline: true });

    const banner = screen.getByRole('status');
    expect(banner).toHaveTextContent(/disconnected/i);
    // The stale snapshot must not be advertised as live at the same time.
    expect(screen.queryByText(/^live$/i)).toBeNull();
    // An explicit "Offline" status badge appears (in addition to the KPI
    // rollup's "Offline devices" label), so there are two "Offline" texts.
    expect(screen.getAllByText(/^offline$/i)).toHaveLength(2);
  });

  it('shows no offline banner while the live socket is connected', () => {
    renderPage({ isOffline: false, isLive: true });

    expect(screen.queryByRole('status')).toBeNull();
    expect(screen.getByText(/^live$/i)).toBeInTheDocument();
  });

  it('shows no offline banner in the default (not-live, not-given-up) state', () => {
    renderPage({ isOffline: false, isLive: false });

    expect(screen.queryByRole('status')).toBeNull();
  });
});
