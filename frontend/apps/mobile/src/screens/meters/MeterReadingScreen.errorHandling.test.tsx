/**
 * Regression (render layer): MeterReadingScreen error paths must surface to the
 * resident via a user-facing alert and must NOT leak a raw `console.error`
 * (code-review-mobile-rn-meter-console-error).
 *
 * Two paths previously logged `console.error(...)` — a dev-only call that never
 * reaches the user and is stripped from release builds:
 *   1. a failed meter-photo upload (online), and
 *   2. a failed submit (offline-queue write error).
 * Both already showed an Alert; this guard pins that behaviour and asserts the
 * raw console.error is gone, matching ReportFaultScreen's `void error`
 * convention.
 *
 * NOTE: render-layer test on the jest-expo pipeline — see the flakiness note in
 * MeterReadingScreen.validation.test.tsx. Mocking style mirrors that file.
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

// Camera returns a picked photo so the upload path is exercised.
jest.mock('expo-image-picker', () => ({
  requestCameraPermissionsAsync: jest.fn().mockResolvedValue({ status: 'granted' }),
  requestMediaLibraryPermissionsAsync: jest.fn().mockResolvedValue({ status: 'granted' }),
  launchCameraAsync: jest
    .fn()
    .mockResolvedValue({ canceled: false, assets: [{ uri: 'file://photo.jpg' }] }),
  launchImageLibraryAsync: jest.fn(),
}));

const mockUploadMeterPhoto = jest.fn();
jest.mock('./meterPhotoUpload', () => ({
  uploadMeterPhoto: (uri: string) => mockUploadMeterPhoto(uri),
}));

jest.mock('../../components/sync', () => ({
  PendingSyncIndicator: () => null,
}));

jest.mock('./useMyMeters', () => ({
  useMyMeters: () => ({
    data: {
      unitIds: ['unit-1'],
      meters: [
        { id: 'meter-1', meter_number: 'CW-001', meter_type: 'cold_water', unit_of_measure: 'm³' },
      ],
    },
    isLoading: false,
    error: null,
  }),
}));

import { MeterReadingScreen } from './MeterReadingScreen';

describe('MeterReadingScreen error surfacing', () => {
  let alertSpy: jest.SpyInstance;
  let consoleErrorSpy: jest.SpyInstance;

  beforeEach(() => {
    jest.clearAllMocks();
    mockAddToQueue.mockResolvedValue(undefined);
    alertSpy = jest.spyOn(Alert, 'alert').mockImplementation(() => {});
    consoleErrorSpy = jest.spyOn(console, 'error').mockImplementation(() => {});
  });

  afterEach(() => {
    alertSpy.mockRestore();
    consoleErrorSpy.mockRestore();
  });

  it('warns the resident when the photo upload fails and keeps the reading, without a raw console.error', async () => {
    mockUploadMeterPhoto.mockRejectedValue(new Error('upload failed'));

    await render(<MeterReadingScreen />);

    // Capture a photo so the upload path runs.
    await fireEvent.press(screen.getByText('faults.cameraButton'));
    await waitFor(() => expect(screen.getByText('✕')).toBeTruthy());

    const input = screen.getByPlaceholderText('meters.readingPlaceholder');
    await fireEvent.changeText(input, '142.4');
    await fireEvent.press(screen.getByText('meters.submit'));

    // The reading is still queued (a failed photo must not drop the reading)...
    await waitFor(() => expect(mockAddToQueue).toHaveBeenCalledTimes(1));
    expect(mockAddToQueue).toHaveBeenCalledWith(
      expect.objectContaining({
        endpoint: '/api/v1/meters/readings',
        // Photo was dropped, so no storage key accompanies the reading.
        body: { meter_id: 'meter-1', reading: '142.4', photo_url: undefined },
      })
    );
    // ...and the dropped photo is surfaced to the resident.
    expect(alertSpy).toHaveBeenCalledWith(
      'meters.photoNotUploadedTitle',
      'meters.photoNotUploadedWarning',
      expect.anything()
    );
    // The failure must not leak through a raw console.error.
    expect(consoleErrorSpy).not.toHaveBeenCalledWith(
      'Failed to upload meter photo:',
      expect.anything()
    );
  });

  it('alerts the resident when the submit (queue write) fails, without a raw console.error', async () => {
    mockAddToQueue.mockRejectedValue(new Error('storage write failed'));

    await render(<MeterReadingScreen />);

    const input = screen.getByPlaceholderText('meters.readingPlaceholder');
    await fireEvent.changeText(input, '142.4');
    await fireEvent.press(screen.getByText('meters.submit'));

    await waitFor(() =>
      expect(alertSpy).toHaveBeenCalledWith('common.error', 'meters.submitFailed')
    );
    expect(consoleErrorSpy).not.toHaveBeenCalledWith(
      'Failed to submit meter reading:',
      expect.anything()
    );
  });
});
