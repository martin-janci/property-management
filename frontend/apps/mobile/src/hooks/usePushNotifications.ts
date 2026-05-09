import Constants from 'expo-constants';
import * as Device from 'expo-device';
import * as Notifications from 'expo-notifications';
import { useEffect, useRef, useState } from 'react';
import { Platform } from 'react-native';
import { createDeepLink } from '../qrcode/DeepLinkHandler';
import { colors } from '../screens/shared/screenStyles';

// Configure notification handling
Notifications.setNotificationHandler({
  handleNotification: async () => ({
    shouldShowAlert: true,
    shouldPlaySound: true,
    shouldSetBadge: true,
  }),
});

export interface PushNotificationState {
  expoPushToken: string | null;
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
  const [notification, setNotification] = useState<Notifications.Notification | null>(null);
  const [isRegistered, setIsRegistered] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const notificationListener = useRef<Notifications.Subscription>();
  const responseListener = useRef<Notifications.Subscription>();

  useEffect(() => {
    // Listen for incoming notifications
    notificationListener.current = Notifications.addNotificationReceivedListener(
      (notif: Notifications.Notification) => {
        setNotification(notif);
      }
    );

    // Listen for notification responses (when user taps)
    responseListener.current = Notifications.addNotificationResponseReceivedListener(
      (response: Notifications.NotificationResponse) => {
        const data = response.notification.request.content.data as Record<string, unknown>;
        handleNotificationNavigation(data);
      }
    );

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
    // Handle deep linking based on notification data
    const { type, id } = data;
    const idString = id ? String(id) : undefined;

    // Map notification types to screen names and create deep links
    let deepLinkUrl: string;

    switch (type) {
      case 'announcement':
        deepLinkUrl = idString
          ? createDeepLink('Announcements', { id: idString })
          : createDeepLink('Announcements');
        break;
      case 'fault':
        deepLinkUrl = idString
          ? createDeepLink('Faults', { id: idString })
          : createDeepLink('Faults');
        break;
      case 'vote':
        deepLinkUrl = idString
          ? createDeepLink('Voting', { id: idString })
          : createDeepLink('Voting');
        break;
      case 'message':
        deepLinkUrl = idString
          ? createDeepLink('Messages', { id: idString })
          : createDeepLink('Messages');
        break;
      case 'outage':
        deepLinkUrl = idString
          ? createDeepLink('Outages', { id: idString })
          : createDeepLink('Outages');
        break;
      default:
        deepLinkUrl = createDeepLink('Dashboard');
    }

    // Use the deep link manager to navigate (handles auth state)
    // The deep link will be processed by DeepLinkManager which
    // dispatches to registered handlers
    const { Linking } = require('react-native');
    Linking.openURL(deepLinkUrl);
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

      // Configure Android channel
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

      // Get Expo push token
      const projectId = Constants.expoConfig?.extra?.eas?.projectId;
      const token = await Notifications.getExpoPushTokenAsync({
        projectId,
      });

      setExpoPushToken(token.data);
      setIsRegistered(true);

      // In a real app, you would send this token to your backend
      await sendTokenToBackend(token.data);

      return token.data;
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to register';
      setError(message);
      return null;
    }
  };

  const sendTokenToBackend = async (_token: string): Promise<void> => {
    // This would call your API to register the device token.
    // NOTE: never log the token itself — it grants push-send authority for this device.
    if (__DEV__) {
      console.log('[push] registering device token with backend');
    }
    // await api.registerPushToken(_token, Platform.OS);
  };

  const unregisterPushNotifications = async (): Promise<void> => {
    try {
      if (expoPushToken) {
        // Remove token from backend
        if (__DEV__) {
          console.log('[push] unregistering device token with backend');
        }
        // await api.unregisterPushToken(expoPushToken);
      }

      setExpoPushToken(null);
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
