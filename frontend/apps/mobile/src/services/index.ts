export type { LaunchNotificationHandler, PushNotificationData } from './backgroundNotifications';
export {
  consumeLaunchNotification,
  extractNotificationData,
  syncBadgeFromData,
} from './backgroundNotifications';
export type { Screen } from './deepLinkRouting';
export { resolveDeepLinkTarget } from './deepLinkRouting';
