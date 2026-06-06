import Constants from 'expo-constants';
import * as Device from 'expo-device';
import * as Notifications from 'expo-notifications';
import { useEffect, useRef, useState } from 'react';
import { Linking, Platform } from 'react-native';
import { colors } from '../screens/shared/screenStyles';
import {
  consumeLaunchNotification,
  deepLinkForNotification,
  type PushNotificationData,
  syncBadgeFromData,
} from '../services/backgroundNotifications';
import { apiRequest } from './useApi';

// Configure notification handling
Notifications.setNotificationHandler({
  handleNotification: async () => ({
    shouldShowAlert: true,
    shouldPlaySound: true,
    shouldSetBadge: true,
  }),
});

// -- Types matching backend RegisterPushTokenRequest / PushTokenResponse -------

export interface RegisterPushTokenRequest {
  token: string;
  platform: 'fcm' | 'apns';
  appId?: string;
  deviceName?: string;
}

interface PushTokenResponse {
  id: string;
  platform: 'fcm' | 'apns';
  appId?: string;
  deviceName?: string;
  lastSeenAt: string;
  createdAt: string;
}

/**
 * Build the `RegisterPushTokenRequest` body for a device's native push token.
 *
 * Extracted as a pure function so the registration payload can be unit-tested
 * without rendering the hook. `platform` is `fcm` on Android and `apns`
 * elsewhere; `app_id` lets the backend fan out to the right FCM project /
 * APNs topic.
 */
export function buildPushTokenRegistration(opts: {
  token: string;
  os: typeof Platform.OS;
  appId?: string;
  deviceName?: string;
}): RegisterPushTokenRequest {
  return {
    token: opts.token,
    platform: opts.os === 'android' ? 'fcm' : 'apns',
    appId: opts.appId,
    deviceName: opts.deviceName,
  };
}

/**
 * POST a device's native push token to the api-server.
 *
 * Critical: pass the body as an OBJECT, not a pre-stringified JSON string.
 * `apiRequest` serialises the body itself (`JSON.stringify(init.body)`), so a
 * pre-encoded string would be JSON-encoded twice and the server would see a
 * quoted string instead of an object (regression fixed in PR #918).
 */
export async function sendPushTokenToBackend(
  body: RegisterPushTokenRequest
): Promise<PushTokenResponse> {
  return apiRequest<PushTokenResponse>('/api/v1/users/me/push-tokens', {
    method: 'POST',
    body,
  });
}

export interface PushNotificationState {
  expoPushToken: string | null;
  /** Native FCM/APNs token registered with the backend. */
  nativePushToken: string | null;
  notification: Notifications.Notification | null;
  isRegistered: boolean;
  error: string | null;
}

export interface UsePushNotificationsReturn extends PushNotificationState {
  registerForPushNotifications: () => Promise<string | null>;
  unregisterPushNotifications: () => Promise<void>;
  schedulePushNotification: (
    title: string,
    body: string,
    data?: Record<string, unknown>
  ) => Promise<void>;
  cancelAllNotifications: () => Promise<void>;
  getBadgeCount: () => Promise<number>;
  setBadgeCount: (count: number) => Promise<void>;
}

export function usePushNotifications(): UsePushNotificationsReturn {
  const [expoPushToken, setExpoPushToken] = useState<string | null>(null);
  const [nativePushToken, setNativePushToken] = useState<string | null>(null);
  const [notification, setNotification] = useState<Notifications.Notification | null>(null);
  const [isRegistered, setIsRegistered] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const notificationListener = useRef<Notifications.Subscription | null>(null);
  const responseListener = useRef<Notifications.Subscription | null>(null);

  // biome-ignore lint/correctness/useExhaustiveDependencies: run-once-on-mount; handlers are stable after first render
  useEffect(() => {
    // Listen for incoming notifications
    notificationListener.current = Notifications.addNotificationReceivedListener(
      (notif: Notifications.Notification) => {
        setNotification(notif);
        // Mirror any badge count carried by the (possibly FCM data-only)
        // payload onto the app icon — iOS does not auto-apply it.
        void syncBadgeFromData(notif.request.content.data);
      }
    );

    // Listen for notification responses (when user taps)
    responseListener.current = Notifications.addNotificationResponseReceivedListener(
      (response: Notifications.NotificationResponse) => {
        const data = response.notification.request.content.data as Record<string, unknown>;
        handleNotificationNavigation(data);
      }
    );

    // Cold-start: if the app was launched from a killed state by tapping a
    // push, the response listener above never fires. Route that launch
    // notification through the same handler once on mount.
    void consumeLaunchNotification((data) => {
      void syncBadgeFromData(data);
      handleNotificationNavigation(data as Record<string, unknown>);
    });

    // Check if already registered
    checkRegistrationStatus();

    return () => {
      if (notificationListener.current) {
        Notifications.removeNotificationSubscription(notificationListener.current);
      }
      if (responseListener.current) {
        Notifications.removeNotificationSubscription(responseListener.current);
      }
    };
  }, []);

  const checkRegistrationStatus = async () => {
    const { status } = await Notifications.getPermissionsAsync();
    setIsRegistered(status === 'granted');
  };

  const handleNotificationNavigation = (data: Record<string, unknown>) => {
    // Map the notification's {type, id} payload to a `ppt://` deep link and
    // hand it to the OS Linking layer; `deepLinkManager` (initialised in
    // App.tsx) picks up the `url` event and routes to the screen. Mapping
    // logic lives in `backgroundNotifications.deepLinkForNotification` so the
    // foreground, runtime-tap, and cold-start paths route identically.
    Linking.openURL(deepLinkForNotification(data as PushNotificationData));
  };

  const registerForPushNotifications = async (): Promise<string | null> => {
    try {
      setError(null);

      if (!Device.isDevice) {
        setError('Push notifications only work on physical devices');
        return null;
      }

      // Check/request permissions
      const { status: existingStatus } = await Notifications.getPermissionsAsync();
      let finalStatus = existingStatus;

      if (existingStatus !== 'granted') {
        const { status } = await Notifications.requestPermissionsAsync();
        finalStatus = status;
      }

      if (finalStatus !== 'granted') {
        setError('Permission to receive push notifications was denied');
        return null;
      }

      // Configure Android notification channels
      if (Platform.OS === 'android') {
        await Notifications.setNotificationChannelAsync('default', {
          name: 'Default',
          importance: Notifications.AndroidImportance.MAX,
          vibrationPattern: [0, 250, 250, 250],
          lightColor: colors.accent,
        });

        await Notifications.setNotificationChannelAsync('announcements', {
          name: 'Announcements',
          description: 'Building announcements and updates',
          importance: Notifications.AndroidImportance.HIGH,
        });

        await Notifications.setNotificationChannelAsync('urgent', {
          name: 'Urgent Alerts',
          description: 'Emergency and urgent notifications',
          importance: Notifications.AndroidImportance.MAX,
          vibrationPattern: [0, 500, 250, 500],
        });
      }

      // 1. Obtain the native FCM/APNs token for direct backend delivery.
      const nativeToken = await Notifications.getDevicePushTokenAsync();
      setNativePushToken(nativeToken.data);

      // 2. Also obtain the Expo push token for Expo-proxy compatibility.
      const projectId = Constants.expoConfig?.extra?.eas?.projectId;
      const expoToken = await Notifications.getExpoPushTokenAsync({ projectId });
      setExpoPushToken(expoToken.data);

      setIsRegistered(true);

      // 3. Register the native token with the backend so the notification
      //    pipeline can deliver FCM/APNs pushes directly to this device.
      await sendTokenToBackend(nativeToken.data);

      return expoToken.data;
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to register';
      setError(message);
      return null;
    }
  };

  const sendTokenToBackend = async (token: string): Promise<void> => {
    // SECURITY: never log the raw token -- it grants push-send authority for this device.
    if (__DEV__) {
      console.log('[push] registering native device token with backend');
    }

    // app_id helps the backend fan-out to the right FCM project / APNs topic.
    const appId =
      Platform.OS === 'android'
        ? (Constants.expoConfig?.android as { package?: string })?.package
        : (Constants.expoConfig?.ios as { bundleIdentifier?: string })?.bundleIdentifier;

    const body = buildPushTokenRegistration({
      token,
      os: Platform.OS,
      appId,
      deviceName: Device.deviceName ?? undefined,
    });

    // `sendPushTokenToBackend` passes the body object straight to `apiRequest`,
    // which serialises it once — passing a pre-stringified string here would
    // double-encode the payload (regression fixed in PR #918).
    await sendPushTokenToBackend(body);
  };

  const unregisterPushNotifications = async (): Promise<void> => {
    try {
      if (nativePushToken) {
        // SECURITY: never log the raw token value.
        if (__DEV__) {
          console.log('[push] unregistering native device token with backend');
        }
        const encodedToken = encodeURIComponent(nativePushToken);
        await apiRequest<void>(`/api/v1/users/me/push-tokens/${encodedToken}`, {
          method: 'DELETE',
        });
      }

      setExpoPushToken(null);
      setNativePushToken(null);
      setIsRegistered(false);
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to unregister';
      setError(message);
    }
  };

  const schedulePushNotification = async (
    title: string,
    body: string,
    data?: Record<string, unknown>
  ): Promise<void> => {
    await Notifications.scheduleNotificationAsync({
      content: {
        title,
        body,
        data: data || {},
      },
      trigger: null, // Immediate notification
    });
  };

  const cancelAllNotifications = async (): Promise<void> => {
    await Notifications.cancelAllScheduledNotificationsAsync();
  };

  const getBadgeCount = async (): Promise<number> => {
    return await Notifications.getBadgeCountAsync();
  };

  const setBadgeCount = async (count: number): Promise<void> => {
    await Notifications.setBadgeCountAsync(count);
  };

  return {
    expoPushToken,
    nativePushToken,
    notification,
    isRegistered,
    error,
    registerForPushNotifications,
    unregisterPushNotifications,
    schedulePushNotification,
    cancelAllNotifications,
    getBadgeCount,
    setBadgeCount,
  };
}
