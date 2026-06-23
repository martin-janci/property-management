/// <reference types="vitest/globals" />
/**
 * Tests for the standalone IoT alerts page (Epic 14 — FR74, issue #1669).
 *
 * Pins the distinct render states:
 *   1. `isError` renders an error panel (role="alert") with a Retry affordance,
 *      NOT the "All clear" empty copy — a hard load failure must not be
 *      misreported as "no alerts";
 *   2. Retry invokes `onRetry`;
 *   3. the empty state (no error, no alerts) still shows the "All clear" copy.
 *
 * Presentational component — no network/context mocks needed; i18n is the real
 * English bundle from the shared test setup.
 */

import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { IotAlertsPage, type IotAlertsPageProps } from './IotAlertsPage';

function renderPage(overrides: Partial<IotAlertsPageProps> = {}) {
  const props: IotAlertsPageProps = {
    alerts: [],
    sensorNames: {},
    isLoading: false,
    pendingAlertId: null,
    filterSeverity: '',
    filterState: '',
    onFilterSeverityChange: vi.fn(),
    onFilterStateChange: vi.fn(),
    onAcknowledge: vi.fn(),
    onResolve: vi.fn(),
    onBackToDashboard: vi.fn(),
    ...overrides,
  };
  return { props, ...render(<IotAlertsPage {...props} />) };
}

describe('IotAlertsPage', () => {
  it('renders a distinct error panel (not the empty state) when isError', () => {
    renderPage({ isError: true, onRetry: vi.fn() });

    // Error panel present...
    const alert = screen.getByRole('alert');
    expect(alert).toHaveTextContent(/failed to load alerts/i);
    // ...and the "All clear" empty copy is NOT shown.
    expect(screen.queryByText(/all clear/i)).toBeNull();
  });

  it('invokes onRetry when the retry button is clicked', () => {
    const onRetry = vi.fn();
    renderPage({ isError: true, onRetry });

    fireEvent.click(screen.getByRole('button', { name: /retry/i }));
    expect(onRetry).toHaveBeenCalledTimes(1);
  });

  it('shows the empty "All clear" copy when there is no error and no alerts', () => {
    renderPage({ isError: false, alerts: [] });

    expect(screen.getByText(/all clear/i)).toBeInTheDocument();
    expect(screen.queryByRole('alert')).toBeNull();
  });
});
