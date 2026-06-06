/**
 * Background / cold-start FCM push handling.
 *
 * Epic 85 - Story 85.3: FCM push notification configuration.
 *
 * The `usePushNotifications` hook covers the two *runtime* cases:
 *   - a notification received while the app is in the foreground
 *     (`addNotificationReceivedListener`), and
 *   - a notification tapped while the app is running/backgrounded
 *     (`addNotificationResponseReceivedListener`).
 *
 * Neither of those listeners fires when the app is launched from a fully
 * killed state by tapping a push (the OS starts a fresh JS context after the
 * event already happened). This module fills that gap:
 *
 *   1. `consumeLaunchNotification()` polls `getLastNotificationResponseAsync`
 *      exactly once on startup and hands the notification's data payload to a
 *      caller-supplied handler. The actual screen routing is owned by the
 *      navigation/deep-link layer — this module deliberately does NOT import
 *      `Linking` so the two concerns stay decoupled.
 *
 *   2. `syncBadgeFromData()` keeps the app-icon badge in sync with the
 *      `badge` field a backgrounded data-notification may carry, since iOS
 *      only auto-applies the badge for *alert* notifications, not data-only
 *      ones routed through FCM.
 *
 *   3. `deepLinkForNotification()` maps a push `{type,id}` payload to the
 *      `ppt://` deep link that opens the matching screen, shared by every
 *      tap entry point in `usePushNotifications`.
 */

import * as Notifications from 'expo-notifications';
import { createDeepLink } from '../qrcode/DeepLinkHandler';

/** Shape of the data payload our backend attaches to every push. */
export interface PushNotificationData {
  /** Domain entity the push refers to (announcement, fault, vote, …). */
  type?: string;
  /** Entity id, used by the routing layer to open a detail screen. */
  id?: string | number;
  /** Optional badge count the OS should display for the app icon. */
  badge?: number;
  [key: string]: unknown;
}

/** Handler invoked with the data payload of a cold-start launch push. */
export type LaunchNotificationHandler = (data: PushNotificationData) => void;

/**
 * Extracts the data payload from a notification response, or `null` when the
 * response/notification is malformed.
 */
export function extractNotificationData(
  response: Notifications.NotificationResponse | null
): PushNotificationData | null {
  const data = response?.notification?.request?.content?.data;
  if (!data || typeof data !== 'object') {
    return null;
  }
  return data as PushNotificationData;
}

/**
 * If the app was launched from a killed state by tapping a push, hand the
 * notification's data payload to `handler`. No-op when launched normally.
 *
 * Safe to call unconditionally on startup; failures are swallowed so a missing
 * launch notification never blocks app boot.
 */
export async function consumeLaunchNotification(handler: LaunchNotificationHandler): Promise<void> {
  try {
    const response = await Notifications.getLastNotificationResponseAsync();
    const data = extractNotificationData(response);
    if (data) {
      handler(data);
    }
  } catch (err) {
    if (__DEV__) {
      console.warn('[push] failed to read launch notification', err);
    }
  }
}

/**
 * Map a push notification's `{ type, id }` data payload to the `ppt://` deep
 * link that opens the matching screen when the notification is tapped.
 *
 * Shared by the foreground tap handler, the runtime-tap handler, and the
 * cold-start launch handler in `usePushNotifications` so all three entry
 * points route identically. Unknown / missing types fall back to the
 * Dashboard so a tap is never a dead end.
 */
export function deepLinkForNotification(data: PushNotificationData | null): string {
  const type = data?.type;
  const idString = data?.id != null ? String(data.id) : undefined;

  // Each push `type` maps to a list screen; when an entity `id` is present we
  // pass it through so the deep-link layer can open the detail screen.
  const screenForType: Record<string, string> = {
    announcement: 'Announcements',
    fault: 'Faults',
    vote: 'Voting',
    message: 'Messages',
    outage: 'Outages',
  };

  const screen = type ? screenForType[type] : undefined;
  if (!screen) {
    return createDeepLink('Dashboard');
  }
  return idString ? createDeepLink(screen, { id: idString }) : createDeepLink(screen);
}

/**
 * Apply the `badge` count carried by a (possibly background-delivered) data
 * notification to the app icon. iOS does not auto-apply badges for data-only
 * FCM messages, so the client mirrors it explicitly.
 */
export async function syncBadgeFromData(data: PushNotificationData | null): Promise<void> {
  if (!data || typeof data.badge !== 'number' || data.badge < 0) {
    return;
  }
  try {
    await Notifications.setBadgeCountAsync(data.badge);
  } catch (err) {
    if (__DEV__) {
      console.warn('[push] failed to sync badge count', err);
    }
  }
}
