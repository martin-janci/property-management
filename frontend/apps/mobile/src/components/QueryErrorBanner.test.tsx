/**
 * QueryErrorBanner — unit tests (#2323).
 *
 * The banner is the extracted, reusable form of the inline error strip
 * DashboardScreen shipped for #2282/#2304. These tests lock its contract:
 * it renders the caller-provided (already-localized) message, exposes a
 * retry button wired to `onRetry`, and never renders a raw error string.
 *
 * The global test setup (src/test/setup.ts) mocks react-i18next so `t`
 * returns the key — hence the assertion on the `common.retry` key.
 */

import { fireEvent, render, screen } from '@testing-library/react-native';
import { QueryErrorBanner } from './QueryErrorBanner';

describe('QueryErrorBanner', () => {
  it('renders the provided localized message and a retry button', () => {
    render(<QueryErrorBanner message="dashboard.loadError" onRetry={jest.fn()} />);

    expect(screen.getByText('dashboard.loadError')).toBeTruthy();
    expect(screen.getByText('common.retry')).toBeTruthy();
  });

  it('calls onRetry when the retry button is pressed', () => {
    const onRetry = jest.fn();
    render(<QueryErrorBanner message="dashboard.loadError" onRetry={onRetry} />);

    fireEvent.press(screen.getByText('common.retry'));

    expect(onRetry).toHaveBeenCalledTimes(1);
  });
});
