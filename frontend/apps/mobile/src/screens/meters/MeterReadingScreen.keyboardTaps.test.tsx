/**
 * Regression (render layer): MeterReadingScreen's ScrollView must set
 * `keyboardShouldPersistTaps="handled"`
 * (code-review-mobile-rn-meter-reading-not-flushed).
 *
 * The reading lives in a controlled, focusable TextInput. With the React
 * Native default (`keyboardShouldPersistTaps="never"`), while that input is
 * focused the keyboard is open, and the FIRST tap on the Submit `Pressable`
 * inside the ScrollView is swallowed to dismiss the keyboard — the button's
 * `onPress` never fires, so the entered reading is silently NOT flushed to the
 * submit / offline-queue path on that tap. Sibling form screens
 * (ReportFaultScreen, FeedbackScreen, DocumentUploadScreen) all set
 * `keyboardShouldPersistTaps="handled"` so the submit press goes through on the
 * first tap; this screen must match.
 *
 * `fireEvent.press` does not model the keyboard tap-eating, so this guard
 * asserts the structural prop directly: it fails before the fix (the
 * `meter-reading-scroll` ScrollView carries no `keyboardShouldPersistTaps`,
 * defaulting to "never") and passes after.
 *
 * NOTE: this render-layer test depends on the jest-expo pipeline, parts of
 * which are flaky under jest-expo@56 / react-native@0.87 (#2951).
 */
import { render } from '@testing-library/react-native';

jest.mock('../../hooks', () => ({
  useOfflineSupport: () => ({
    isConnected: true,
    addToQueue: jest.fn(),
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

describe('MeterReadingScreen keyboard-persist-taps guard', () => {
  it('keeps the keyboard-open Submit tap alive so the reading is flushed', async () => {
    const { getByTestId } = await render(<MeterReadingScreen />);

    const scrollView = getByTestId('meter-reading-scroll');

    // Without this, the first Submit tap while the reading field is focused is
    // eaten by the keyboard dismissal and the entered reading never reaches the
    // submit path.
    expect(scrollView.props.keyboardShouldPersistTaps).toBe('handled');
  });
});
