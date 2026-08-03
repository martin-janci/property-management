// Regression for PR #918 (Critical #2): push-token registration.
//
// Two bugs were fixed in PR #918 and went out WITHOUT a regression test:
//
//  1. `sendTokenToBackend` passed `body: JSON.stringify(body)` while the shared
//     `apiRequest` already serialises the body (`JSON.stringify(init.body)`).
//     The payload was therefore JSON-encoded twice and the api-server saw a
//     quoted string instead of an object.
//  2. The Android/iOS split (`platform: 'fcm' | 'apns'`) and the registration
//     payload shape must match the backend `RegisterPushTokenRequest`.
//
// These tests pin both: the request body handed to `apiRequest` must be a
// plain OBJECT (never a string), and the payload fields must be correct per
// platform. The hook delegates to these pure helpers, so we can verify the
// behaviour without rendering React — important because the repo-wide
// `react-test-renderer` 19.2.0-vs-19.2.6 peer-dep mismatch currently breaks
// every test that imports `@testing-library/react-native` (PR #918 finding #7).

import { apiRequest } from './useApi';
import { buildPushTokenRegistration, sendPushTokenToBackend } from './usePushNotifications';

// The mobile tsconfig limits `types` to jest + testing-library (no @types/node),
// so declare the tiny node surface we use — same pattern as
// `screens/documents/DocumentUploadScreen.parity.test.ts`.
declare const __dirname: string;
declare function require(id: 'node:fs'): { readFileSync(path: string, encoding: 'utf8'): string };
declare function require(id: 'node:path'): { join(...parts: string[]): string };

jest.mock('./useApi', () => ({
  apiRequest: jest.fn(),
}));

// `usePushNotifications` imports these expo modules at module-eval time
// (e.g. `Notifications.setNotificationHandler(...)`), so they must be mocked
// for the import to succeed under jest.
jest.mock('expo-constants', () => ({ expoConfig: { extra: {} } }));
jest.mock('expo-device', () => ({ isDevice: true, deviceName: 'Test Phone' }));
jest.mock('expo-notifications', () => ({
  setNotificationHandler: jest.fn(),
  AndroidImportance: { MAX: 5, HIGH: 4 },
}));

const mockApiRequest = apiRequest as jest.Mock;

beforeEach(() => {
  jest.clearAllMocks();
  mockApiRequest.mockResolvedValue({
    id: 'pt-1',
    platform: 'fcm',
    lastSeenAt: '2026-06-01T00:00:00Z',
    createdAt: '2026-06-01T00:00:00Z',
  });
});

describe('buildPushTokenRegistration (PR #918 payload shape)', () => {
  it('maps Android to the `fcm` platform', () => {
    expect(
      buildPushTokenRegistration({
        token: 'native-token',
        os: 'android',
        appId: 'three.two.bit.ppt.management',
        deviceName: 'Pixel',
      })
    ).toEqual({
      token: 'native-token',
      platform: 'fcm',
      appId: 'three.two.bit.ppt.management',
      deviceName: 'Pixel',
    });
  });

  it('maps iOS to the `apns` platform', () => {
    expect(buildPushTokenRegistration({ token: 't', os: 'ios' }).platform).toBe('apns');
  });

  it('maps any non-android OS to `apns`', () => {
    expect(buildPushTokenRegistration({ token: 't', os: 'web' }).platform).toBe('apns');
  });
});

describe('sendPushTokenToBackend (PR #918 double-encode regression)', () => {
  it('POSTs to the push-tokens endpoint with the correct method', async () => {
    await sendPushTokenToBackend({ token: 't', platform: 'fcm' });

    expect(mockApiRequest).toHaveBeenCalledTimes(1);
    const [path, init] = mockApiRequest.mock.calls[0];
    expect(path).toBe('/api/v1/users/me/push-tokens');
    expect(init.method).toBe('POST');
  });

  it('passes the body as an OBJECT, not a pre-stringified JSON string', async () => {
    const body = {
      token: 'native-token',
      platform: 'fcm' as const,
      appId: 'three.two.bit.ppt.management',
      deviceName: 'Pixel',
    };

    await sendPushTokenToBackend(body);

    const init = mockApiRequest.mock.calls[0][1];
    // The core of the regression: had the caller passed `JSON.stringify(body)`,
    // `apiRequest` would double-encode it. The body must be the raw object.
    expect(typeof init.body).toBe('object');
    expect(init.body).toEqual(body);
    expect(typeof init.body).not.toBe('string');
  });
});

// Regression guard: no ungated debug logging on the push-token register /
// unregister production path (code-review-mobile-rn-push-debug-console-log).
//
// The two `console.log` calls on that path are intentional dev-only
// breadcrumbs, but they MUST stay wrapped in `if (__DEV__)` — Metro strips
// those blocks from release builds, so nothing reaches the production path.
// This is the same source-level contract lock used by `config/constants.test.ts`.
// A raw `console.log('...')` slipping back in un-gated (or one that logs the
// token itself) would re-open the finding, so we assert the shape of the
// source rather than rendering the hook.
describe('usePushNotifications — no ungated debug logging on the token path', () => {
  const { readFileSync } = require('node:fs');
  const { join } = require('node:path');
  const source = readFileSync(join(__dirname, 'usePushNotifications.ts'), 'utf8');
  const lines = source.split('\n');

  const consoleLineNumbers = lines
    .map((line, i) => ({ line, n: i }))
    .filter(({ line }) => /\bconsole\.\w+\s*\(/.test(line));

  it('has console calls to guard (fixture sanity)', () => {
    // If the calls are ever removed entirely that is also acceptable, but this
    // keeps the guard honest while they exist.
    expect(consoleLineNumbers.length).toBeGreaterThan(0);
  });

  it('gates every console.* call behind `if (__DEV__)`', () => {
    // Any console.* whose nearest non-blank preceding line is not `if (__DEV__) {`
    // is un-gated and would ship to the production path.
    const ungated = consoleLineNumbers.filter(({ n }) => {
      let prev = n - 1;
      while (prev >= 0 && lines[prev].trim() === '') prev--;
      return lines[prev]?.trim() !== 'if (__DEV__) {';
    });
    expect(ungated.map(({ line }) => line.trim())).toEqual([]);
  });

  it('never logs the raw push token value', () => {
    // SECURITY: the native FCM/APNs token grants push-send authority for the
    // device. A static breadcrumb string may say the word "token", but no
    // console statement may interpolate or concatenate the actual value.
    for (const { line } of consoleLineNumbers) {
      expect(line).not.toContain('${'); // no template-literal interpolation
      expect(line).not.toMatch(/\+\s*(?:native)?[Pp]ushToken|encodedToken\b|\btoken\b\s*\)/);
    }
  });
});
