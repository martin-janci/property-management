export type { LaunchNotificationHandler, PushNotificationData } from './backgroundNotifications';
export {
  consumeLaunchNotification,
  extractNotificationData,
  syncBadgeFromData,
} from './backgroundNotifications';
