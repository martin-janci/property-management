/**
 * Unit tests for background / cold-start FCM push handling.
 *
 * Epic 85 - Story 85.3.
 */

import * as Notifications from 'expo-notifications';
import {
  consumeLaunchNotification,
  extractNotificationData,
  syncBadgeFromData,
} from './backgroundNotifications';

jest.mock('expo-notifications', () => ({
  getLastNotificationResponseAsync: jest.fn(),
  setBadgeCountAsync: jest.fn(),
}));

const mockGetLast = Notifications.getLastNotificationResponseAsync as jest.Mock;
const mockSetBadge = Notifications.setBadgeCountAsync as jest.Mock;

function makeResponse(data: unknown): Notifications.NotificationResponse {
  return {
    actionIdentifier: 'default',
    notification: {
      date: 0,
      request: {
        identifier: 'id',
        // biome-ignore lint/suspicious/noExplicitAny: test fixture intentionally loose
        content: { data } as any,
        trigger: null,
      },
    },
  } as Notifications.NotificationResponse;
}

beforeEach(() => {
  jest.clearAllMocks();
});

describe('extractNotificationData', () => {
  it('returns the data payload from a valid response', () => {
    expect(extractNotificationData(makeResponse({ type: 'fault', id: '7' }))).toEqual({
      type: 'fault',
      id: '7',
    });
  });

  it('returns null for a null response', () => {
    expect(extractNotificationData(null)).toBeNull();
  });

  it('returns null when data is missing', () => {
    expect(extractNotificationData(makeResponse(undefined))).toBeNull();
  });
});

describe('consumeLaunchNotification', () => {
  it('invokes the handler with the launch payload when present', async () => {
    mockGetLast.mockResolvedValue(makeResponse({ type: 'announcement', id: '42' }));
    const handler = jest.fn();

    await consumeLaunchNotification(handler);

    expect(handler).toHaveBeenCalledWith({ type: 'announcement', id: '42' });
  });

  it('does nothing when the app was not launched from a notification', async () => {
    mockGetLast.mockResolvedValue(null);
    const handler = jest.fn();

    await consumeLaunchNotification(handler);

    expect(handler).not.toHaveBeenCalled();
  });

  it('swallows errors so app boot is never blocked', async () => {
    mockGetLast.mockRejectedValue(new Error('boom'));
    const handler = jest.fn();

    await expect(consumeLaunchNotification(handler)).resolves.toBeUndefined();
    expect(handler).not.toHaveBeenCalled();
  });
});

describe('syncBadgeFromData', () => {
  it('applies a valid badge count', async () => {
    await syncBadgeFromData({ badge: 5 });
    expect(mockSetBadge).toHaveBeenCalledWith(5);
  });

  it('ignores a missing badge', async () => {
    await syncBadgeFromData({ type: 'fault' });
    expect(mockSetBadge).not.toHaveBeenCalled();
  });

  it('ignores a negative badge', async () => {
    await syncBadgeFromData({ badge: -1 });
    expect(mockSetBadge).not.toHaveBeenCalled();
  });

  it('ignores null data', async () => {
    await syncBadgeFromData(null);
    expect(mockSetBadge).not.toHaveBeenCalled();
  });
});
