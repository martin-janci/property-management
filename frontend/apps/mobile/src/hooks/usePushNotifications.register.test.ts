// Regression for `code-review-mobile-rn-push-isregistered-premature`.
//
// `usePushNotifications` exposed `isRegistered` PREMATURELY — it reported a
// device as registered before the full registration had actually succeeded:
//
//   1. `registerForPushNotifications` set `setIsRegistered(true)` BEFORE the
//      `sendTokenToBackend` POST. When that POST rejected, the catch block
//      surfaced the error but left `isRegistered` stuck at `true`, so the
//      UI/state claimed the device could receive pushes when the backend had
//      never accepted its token.
//   2. The on-mount `checkRegistrationStatus` set `isRegistered = (permission
//      === 'granted')`. A granted OS permission is only a *pre-condition* for
//      registration (it survives reinstalls / account switches), not proof the
//      current backend ever accepted a token.
//
// The fix: `isRegistered` becomes `true` ONLY after a native token is obtained
// AND the backend accepts it (`registerForPushNotifications`); the mount check
// only ever *clears* the flag when permission is absent.
//
// The hook is a real React hook (useState/useEffect/useRef), so it is driven
// through a tiny `react-test-renderer` harness — the same approach as
// `useOfflineSupport.test.ts`, which deliberately avoids
// `@testing-library/react-native`. All native modules are mocked so the flow
// runs without a device runtime.

import { createElement } from 'react';
// @ts-expect-error -- no type declarations for react-test-renderer in this repo
import TestRenderer, { act } from 'react-test-renderer';
import { apiRequest } from './useApi';
import { type UsePushNotificationsReturn, usePushNotifications } from './usePushNotifications';

jest.mock('./useApi', () => ({ apiRequest: jest.fn() }));

// The hook imports these at module-eval time, so they must resolve under jest.
jest.mock('expo-constants', () => ({
  expoConfig: { extra: {}, ios: { bundleIdentifier: 'three.two.bit.ppt.management' } },
}));
jest.mock('expo-device', () => ({ isDevice: true, deviceName: 'Test Phone' }));

// Mutable permission state so individual tests control what the OS reports.
const TEST_GLOBAL = globalThis as typeof globalThis & { __pushPermission?: string };
TEST_GLOBAL.__pushPermission = 'granted';

jest.mock('expo-notifications', () => ({
  setNotificationHandler: jest.fn(),
  addNotificationReceivedListener: jest.fn(() => ({ remove: jest.fn() })),
  addNotificationResponseReceivedListener: jest.fn(() => ({ remove: jest.fn() })),
  removeNotificationSubscription: jest.fn(),
  getPermissionsAsync: jest.fn(async () => ({
    status: (globalThis as { __pushPermission?: string }).__pushPermission,
  })),
  requestPermissionsAsync: jest.fn(async () => ({
    status: (globalThis as { __pushPermission?: string }).__pushPermission,
  })),
  getDevicePushTokenAsync: jest.fn(async () => ({ data: 'native-token-abc', type: 'apns' })),
  getExpoPushTokenAsync: jest.fn(async () => ({ data: 'ExponentPushToken[xyz]' })),
  setNotificationChannelAsync: jest.fn(async () => undefined),
  AndroidImportance: { MAX: 5, HIGH: 4 },
}));

// The register flow does not exercise these, but the hook imports them at the
// top level; keep them inert so mount side-effects don't touch Linking/badges.
jest.mock('../services/backgroundNotifications', () => ({
  consumeLaunchNotification: jest.fn(async () => undefined),
  deepLinkForNotification: jest.fn(() => 'ppt://'),
  syncBadgeFromData: jest.fn(async () => undefined),
}));

const mockApiRequest = apiRequest as jest.Mock;

/**
 * Mount the hook through a throwaway renderer and return a live handle to its
 * latest return value plus the renderer (for unmount).
 */
async function mountHook(): Promise<{
  current: () => UsePushNotificationsReturn;
  unmount: () => Promise<void>;
}> {
  let latest: UsePushNotificationsReturn | undefined;
  function Harness() {
    latest = usePushNotifications();
    return null;
  }

  let renderer: { unmount: () => void } | undefined;
  await act(async () => {
    renderer = TestRenderer.create(createElement(Harness));
  });

  return {
    current: () => {
      if (!latest) throw new Error('hook never rendered');
      return latest;
    },
    unmount: async () => {
      await act(async () => {
        renderer?.unmount();
      });
    },
  };
}

beforeEach(() => {
  jest.clearAllMocks();
  TEST_GLOBAL.__pushPermission = 'granted';
  mockApiRequest.mockResolvedValue({
    id: 'pt-1',
    platform: 'apns',
    lastSeenAt: '2026-06-01T00:00:00Z',
    createdAt: '2026-06-01T00:00:00Z',
  });
});

describe('usePushNotifications — isRegistered is not set prematurely', () => {
  it('does NOT report registered on mount merely because OS permission is granted', async () => {
    const hook = await mountHook();
    // Permission is granted, but no token has been obtained or sent to the
    // backend, so the device is not actually registered.
    expect(hook.current().isRegistered).toBe(false);
    await hook.unmount();
  });

  it('stays NOT registered when the backend registration POST fails', async () => {
    mockApiRequest.mockRejectedValueOnce(new Error('500 server error'));
    const hook = await mountHook();

    await act(async () => {
      await hook.current().registerForPushNotifications();
    });

    // The native token was obtained (permission granted, token fetched) …
    expect(hook.current().nativePushToken).toBe('native-token-abc');
    // … but the backend rejected it, so the device must NOT be reported as
    // registered. This is the core premature-flag regression.
    expect(hook.current().isRegistered).toBe(false);
    expect(hook.current().error).toBe('500 server error');
    await hook.unmount();
  });

  it('reports registered only after the backend accepts the token', async () => {
    const hook = await mountHook();

    await act(async () => {
      await hook.current().registerForPushNotifications();
    });

    expect(mockApiRequest).toHaveBeenCalledTimes(1);
    expect(hook.current().isRegistered).toBe(true);
    expect(hook.current().nativePushToken).toBe('native-token-abc');
    expect(hook.current().error).toBeNull();
    await hook.unmount();
  });
});
