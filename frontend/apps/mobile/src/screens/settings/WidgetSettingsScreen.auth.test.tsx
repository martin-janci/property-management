/**
 * WidgetSettingsScreen — widget-bridge auth-token wiring regression test
 * (code-review-mobile-rn-widget-bridge-auth-token-never-set).
 *
 * The widget bridge exposes `setAuthToken()` (which forwards to its
 * WidgetDataProvider, the thing that actually sends the `Authorization: Bearer`
 * header). But nothing ever called it: the screen constructed a `WidgetBridge`
 * and never fed it the signed-in user's access token, so every widget data
 * request (configure / refresh) went out unauthenticated and fell back to empty
 * data.
 *
 * This suite locks the contract: the screen must propagate the `accessToken`
 * from `useAuth()` into the bridge via `setAuthToken()`, and keep it in sync
 * when the token changes (e.g. logout → null).
 */
import { render, waitFor } from '@testing-library/react-native';
import { WidgetSettingsScreen } from './WidgetSettingsScreen';

// Single shared spy so the assertions don't depend on which bridge instance the
// screen constructed (it builds exactly one via useMemo).
const mockSetAuthToken = jest.fn();

jest.mock('../../widgets', () => {
  class MockWidgetBridge {
    setAuthToken = mockSetAuthToken;
    getAvailableWidgetTypes = jest.fn(() => []);
    isWidgetSupported = jest.fn(async () => true);
    getAllConfigs = jest.fn(async () => []);
    configureWidget = jest.fn(async () => {});
    removeWidget = jest.fn(async () => {});
    updateAllWidgets = jest.fn(async () => {});
  }
  return { WidgetBridge: MockWidgetBridge };
});

// Drives the token the mocked useAuth() returns; set per-test before render.
let mockAccessToken: string | null = 'widget-bearer-token';
jest.mock('../../contexts', () => ({
  useAuth: () => ({ accessToken: mockAccessToken }),
}));

describe('WidgetSettingsScreen auth-token wiring', () => {
  beforeEach(() => {
    mockSetAuthToken.mockClear();
  });

  it('propagates the signed-in access token into the widget bridge', async () => {
    mockAccessToken = 'widget-bearer-token';

    render(<WidgetSettingsScreen onNavigate={jest.fn()} />);

    await waitFor(
      () => {
        expect(mockSetAuthToken).toHaveBeenCalledWith('widget-bearer-token');
      },
      { timeout: 5000 }
    );
  });

  it('propagates a null token when signed out so the bridge drops the bearer', async () => {
    mockAccessToken = null;

    render(<WidgetSettingsScreen onNavigate={jest.fn()} />);

    await waitFor(
      () => {
        expect(mockSetAuthToken).toHaveBeenCalledWith(null);
      },
      { timeout: 5000 }
    );
  });
});
