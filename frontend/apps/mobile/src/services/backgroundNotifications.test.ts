/**
 * Unit tests for background / cold-start FCM push handling.
 *
 * Epic 85 - Story 85.3.
 */

import * as Notifications from 'expo-notifications';
import {
  consumeLaunchNotification,
  deepLinkForNotification,
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

describe('deepLinkForNotification', () => {
  it('routes a typed push with an id to the matching detail deep link', () => {
    expect(deepLinkForNotification({ type: 'announcement', id: '42' })).toBe(
      'ppt://announcements/42'
    );
    expect(deepLinkForNotification({ type: 'fault', id: 7 })).toBe('ppt://faults/7');
    expect(deepLinkForNotification({ type: 'vote', id: '9' })).toBe('ppt://voting/9');
    expect(deepLinkForNotification({ type: 'message', id: '3' })).toBe('ppt://messages/3');
    expect(deepLinkForNotification({ type: 'outage', id: '5' })).toBe('ppt://outages/5');
  });

  it('routes a typed push without an id to the list deep link', () => {
    expect(deepLinkForNotification({ type: 'announcement' })).toBe('ppt://announcements');
    expect(deepLinkForNotification({ type: 'fault' })).toBe('ppt://faults');
  });

  it('falls back to the dashboard for an unknown type', () => {
    expect(deepLinkForNotification({ type: 'wat', id: '1' })).toBe('ppt://dashboard');
  });

  it('falls back to the dashboard for a missing type or null payload', () => {
    expect(deepLinkForNotification({ id: '1' })).toBe('ppt://dashboard');
    expect(deepLinkForNotification(null)).toBe('ppt://dashboard');
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
