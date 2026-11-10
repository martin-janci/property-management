/**
 * Regression (render layer): MeterReadingScreen must not queue a non-numeric
 * reading (code-review-mobile-rn-meter-reading-no-numeric-validation).
 *
 * Proves the submit guard is wired: typing garbage and pressing Submit shows
 * the `meters.readingInvalid` alert and never calls `addToQueue`, whereas a
 * valid number does queue. Mirrors the mocking style of
 * MeterDetailScreen.i18n.test.tsx (react-i18next `t(key) => key` from setup.ts).
 *
 * NOTE: this render-layer test depends on the jest-expo pipeline. Parts of the
 * mobile render suite are flaky under jest-expo@56 / react-native@0.87 (#2951);
 * the pure-function suite in meterReadingInput.test.ts is the
 * environment-independent regression guard.
 */
import { fireEvent, render, screen, waitFor } from '@testing-library/react-native';
import { Alert } from 'react-native';

const mockAddToQueue = jest.fn();

jest.mock('../../hooks', () => ({
  useOfflineSupport: () => ({
    isConnected: true,
    addToQueue: mockAddToQueue,
  }),
}));

jest.mock('expo-image-picker', () => ({
  requestCameraPermissionsAsync: jest.fn(),
  requestMediaLibraryPermissionsAsync: jest.fn(),
  launchCameraAsync: jest.fn(),
  launchImageLibraryAsync: jest.fn(),
}));

// PendingSyncIndicator pulls in sync UI we don't exercise here.
jest.mock('../../components/sync', () => ({
  PendingSyncIndicator: () => null,
}));

import { MeterReadingScreen } from './MeterReadingScreen';

describe('MeterReadingScreen numeric-validation guard', () => {
  let alertSpy: jest.SpyInstance;

  beforeEach(() => {
    jest.clearAllMocks();
    alertSpy = jest.spyOn(Alert, 'alert').mockImplementation(() => {});
  });

  afterEach(() => {
    alertSpy.mockRestore();
  });

  it('does not queue a reading when the input is non-numeric', async () => {
    await render(<MeterReadingScreen />);

    const input = screen.getByPlaceholderText('meters.readingPlaceholder');
    // Simulate garbage that a hardware keyboard / paste can inject despite
    // keyboardType="decimal-pad": normalization strips it to empty.
    await fireEvent.changeText(input, 'abc');
    await fireEvent.press(screen.getByText('meters.submit'));

    expect(mockAddToQueue).not.toHaveBeenCalled();
    // Empty-after-normalize hits the "required" guard; either way nothing queues.
    expect(alertSpy).toHaveBeenCalled();
  });

  it('shows the invalid alert for a lone decimal separator and does not queue', async () => {
    await render(<MeterReadingScreen />);

    const input = screen.getByPlaceholderText('meters.readingPlaceholder');
    await fireEvent.changeText(input, '.');
    await fireEvent.press(screen.getByText('meters.submit'));

    expect(mockAddToQueue).not.toHaveBeenCalled();
    expect(alertSpy).toHaveBeenCalledWith('common.error', 'meters.readingInvalid');
  });

  it('queues a valid numeric reading', async () => {
    await render(<MeterReadingScreen />);

    const input = screen.getByPlaceholderText('meters.readingPlaceholder');
    await fireEvent.changeText(input, '142.4');
    await fireEvent.press(screen.getByText('meters.submit'));

    // handleSubmit is async (awaits addToQueue), so wait for the effect.
    await waitFor(() => expect(mockAddToQueue).toHaveBeenCalledTimes(1));
    expect(mockAddToQueue).toHaveBeenCalledWith(
      expect.objectContaining({
        // Backend route is POST /api/v1/meters/readings (api-server lib.rs
        // nests "/api/v1/meters" + meters.rs "/readings"); the @ppt/api-client
        // `submitReading` posts to the same path. The old "/api/v1/meter-readings"
        // did not exist and 404'd on sync.
        endpoint: '/api/v1/meters/readings',
        body: expect.objectContaining({ reading: '142.4' }),
      })
    );
  });
});
