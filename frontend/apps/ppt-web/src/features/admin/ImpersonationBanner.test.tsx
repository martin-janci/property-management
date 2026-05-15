/**
 * ImpersonationBanner — behaviour tests (E2).
 *
 * The banner ships in `@ppt/admin-ui`; the host wraps it in
 * `<ImpersonationWrapper>` to inject impersonation state, i18n labels,
 * and the stop-call. These tests exercise both layers from the host
 * side because admin-ui has no test harness:
 *
 *   1. wrapper renders nothing when there is no impersonation record.
 *   2. wrapper renders the banner when an active record is provided,
 *      including the target user label and the localized "stop" copy.
 *   3. clicking "End impersonation" invokes the stop override and
 *      removes the banner.
 *   4. raw `<ImpersonationBanner active={false}>` renders nothing —
 *      pins the package contract so future host wrappers cannot rely
 *      on accidental rendering.
 */

/// <reference types="vitest/globals" />
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { ImpersonationBanner } from '@ppt/admin-ui';
import type { ReactNode } from 'react';
import { ImpersonationWrapper } from './ImpersonationWrapper';

// AuthContext is consumed by the wrapper for the bearer token. Mock
// it locally so the test file does not need to mount AuthProvider.
vi.mock('../../contexts', () => ({
  useAuth: () => ({
    getAccessToken: () => 'test-token',
  }),
}));

function renderWithProviders(node: ReactNode) {
  return render(<>{node}</>);
}

describe('ImpersonationBanner (raw component)', () => {
  it('renders nothing when inactive', () => {
    render(<ImpersonationBanner active={false} />);
    expect(screen.queryByTestId('impersonation-banner')).toBeNull();
  });

  it('renders with custom labels when active', () => {
    render(
      <ImpersonationBanner
        active
        targetUserLabel="alice@example.com"
        labels={{ label: 'IMPERSONATING', asUser: 'as {name}', stop: 'End impersonation' }}
        onEnd={() => undefined}
      />,
    );
    expect(screen.getByTestId('impersonation-banner')).toBeInTheDocument();
    expect(screen.getByText(/IMPERSONATING/)).toBeInTheDocument();
    expect(screen.getByText(/as alice@example.com/)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /end impersonation/i })).toBeInTheDocument();
  });
});

describe('ImpersonationWrapper', () => {
  it('renders nothing when no impersonation is active', () => {
    renderWithProviders(<ImpersonationWrapper initialState={null} />);
    expect(screen.queryByTestId('impersonation-banner')).toBeNull();
  });

  it('renders the banner when an active record is provided', () => {
    renderWithProviders(
      <ImpersonationWrapper
        initialState={{
          tokenId: 'tok-1',
          // Far future so the wrapper does not garbage-collect it.
          expiresAt: new Date(Date.now() + 10 * 60_000).toISOString(),
          targetUserLabel: 'alice@example.com',
        }}
      />,
    );
    expect(screen.getByTestId('impersonation-banner')).toBeInTheDocument();
    expect(screen.getByText(/alice@example.com/)).toBeInTheDocument();
  });

  it('invokes the stop-call and hides the banner when "End" is clicked', async () => {
    const user = userEvent.setup();
    const stopOverride = vi.fn().mockResolvedValue(true);
    renderWithProviders(
      <ImpersonationWrapper
        initialState={{
          tokenId: 'tok-2',
          expiresAt: new Date(Date.now() + 10 * 60_000).toISOString(),
          targetUserLabel: 'bob@example.com',
        }}
        stopOverride={stopOverride}
      />,
    );
    expect(screen.getByTestId('impersonation-banner')).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: /end impersonation|stop/i }));
    await waitFor(() => expect(stopOverride).toHaveBeenCalledWith('tok-2'));
    await waitFor(() => expect(screen.queryByTestId('impersonation-banner')).toBeNull());
  });
});
