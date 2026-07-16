// Type declarations for Expo modules
// These will be properly typed when packages are installed

declare module 'expo-local-authentication' {
  export enum AuthenticationType {
    FINGERPRINT = 1,
    FACIAL_RECOGNITION = 2,
    IRIS = 3,
  }

  export enum SecurityLevel {
    NONE = 0,
    SECRET = 1,
    BIOMETRIC = 2,
  }

  export interface LocalAuthenticationOptions {
    promptMessage?: string;
    cancelLabel?: string;
    disableDeviceFallback?: boolean;
    fallbackLabel?: string;
  }

  export interface LocalAuthenticationResult {
    success: boolean;
    error?: string;
    warning?: string;
  }

  export function hasHardwareAsync(): Promise<boolean>;
  export function supportedAuthenticationTypesAsync(): Promise<AuthenticationType[]>;
  export function isEnrolledAsync(): Promise<boolean>;
  export function getEnrolledLevelAsync(): Promise<SecurityLevel>;
  export function authenticateAsync(
    options?: LocalAuthenticationOptions
  ): Promise<LocalAuthenticationResult>;
}

declare module 'expo-secure-store' {
  export interface SecureStoreOptions {
    keychainService?: string;
    keychainAccessible?: number;
    requireAuthentication?: boolean;
    authenticationPrompt?: string;
  }

  export function getItemAsync(key: string, options?: SecureStoreOptions): Promise<string | null>;
  export function setItemAsync(
    key: string,
    value: string,
    options?: SecureStoreOptions
  ): Promise<void>;
  export function deleteItemAsync(key: string, options?: SecureStoreOptions): Promise<void>;
  export function isAvailableAsync(): Promise<boolean>;
}

declare module 'expo-constants' {
  interface ExpoConfig {
    extra?: {
      apiUrl?: string;
      eas?: {
        projectId?: string;
      };
      [key: string]: unknown;
    };
    [key: string]: unknown;
  }

  interface Constants {
    expoConfig: ExpoConfig | null;
    appOwnership: string | null;
    executionEnvironment: string;
    [key: string]: unknown;
  }

  const constants: Constants;
  export default constants;
}

declare module 'expo-device' {
  export const isDevice: boolean;
  export const brand: string | null;
  export const manufacturer: string | null;
  export const modelName: string | null;
  export const modelId: string | null;
  export const designName: string | null;
  export const productName: string | null;
  export const deviceYearClass: number | null;
  export const totalMemory: number | null;
  export const supportedCpuArchitectures: string[] | null;
  export const osName: string | null;
  export const osVersion: string | null;
  export const osBuildId: string | null;
  export const osInternalBuildId: string | null;
  export const osBuildFingerprint: string | null;
  export const platformApiLevel: number | null;
  export const deviceName: string | null;
}

declare module 'expo-notifications' {
  export interface Notification {
    date: number;
    request: NotificationRequest;
  }

  export interface NotificationRequest {
    identifier: string;
    content: NotificationContent;
    trigger: NotificationTrigger | null;
  }

  export interface NotificationContent {
    title: string | null;
    subtitle: string | null;
    body: string | null;
    data: Record<string, unknown>;
    sound: string | 'default' | null;
    launchImageName: string | null;
    badge: number | null;
    attachments: NotificationContentAttachment[];
    categoryIdentifier: string | null;
  }

  export interface NotificationContentAttachment {
    identifier: string | null;
    url: string | null;
    type: string | null;
  }

  export type NotificationTrigger = unknown;

  export interface NotificationResponse {
    notification: Notification;
    actionIdentifier: string;
    userText?: string;
  }

  export interface Subscription {
    remove: () => void;
  }

  export interface NotificationBehavior {
    shouldShowAlert: boolean;
    shouldPlaySound: boolean;
    shouldSetBadge: boolean;
  }

  export interface NotificationHandler {
    handleNotification: (notification: Notification) => Promise<NotificationBehavior>;
    handleSuccess?: (notificationId: string) => void;
    handleError?: (notificationId: string, error: Error) => void;
  }

  export interface NotificationChannelInput {
    name: string;
    importance?: AndroidImportance;
    bypassDnd?: boolean;
    description?: string;
    groupId?: string;
    lightColor?: string;
    lockscreenVisibility?: number;
    showBadge?: boolean;
    sound?: string | null;
    audioAttributes?: unknown;
    vibrationPattern?: number[];
    enableLights?: boolean;
    enableVibrate?: boolean;
  }

  export enum AndroidImportance {
    UNKNOWN = 0,
    UNSPECIFIED = -1000,
    // NONE = 0, // Duplicate of UNKNOWN
    MIN = 1,
    LOW = 2,
    DEFAULT = 3,
    HIGH = 4,
    MAX = 5,
  }

  export interface ExpoPushToken {
    type: 'expo';
    data: string;
  }

  export interface PermissionResponse {
    status: 'granted' | 'denied' | 'undetermined';
    expires: 'never' | number;
    granted: boolean;
    canAskAgain: boolean;
  }

  export function setNotificationHandler(handler: NotificationHandler | null): void;
  export function getPermissionsAsync(): Promise<PermissionResponse>;
  export function requestPermissionsAsync(): Promise<PermissionResponse>;
  export function getExpoPushTokenAsync(options?: { projectId?: string }): Promise<ExpoPushToken>;
  export function setNotificationChannelAsync(
    channelId: string,
    channel: NotificationChannelInput
  ): Promise<unknown>;
  export function addNotificationReceivedListener(
    listener: (notification: Notification) => void
  ): Subscription;
  export function addNotificationResponseReceivedListener(
    listener: (response: NotificationResponse) => void
  ): Subscription;
  export function removeNotificationSubscription(subscription: Subscription): void;
  export function scheduleNotificationAsync(options: {
    content: {
      title?: string;
      body?: string;
      data?: Record<string, unknown>;
    };
    trigger: unknown;
  }): Promise<string>;
  export function cancelAllScheduledNotificationsAsync(): Promise<void>;
  export function getBadgeCountAsync(): Promise<number>;
  export function setBadgeCountAsync(count: number): Promise<void>;

  export interface DevicePushToken {
    type: 'ios' | 'android' | 'web';
    data: string;
  }
  export function getDevicePushTokenAsync(): Promise<DevicePushToken>;

  /**
   * Returns the notification response that launched the app from a killed
   * (cold-start) state, or `null` when the app was launched normally. The
   * runtime-listener `addNotificationResponseReceivedListener` does NOT fire
   * for the cold-start case, so this must be polled once on startup.
   */
  export function getLastNotificationResponseAsync(): Promise<NotificationResponse | null>;

  /**
   * Registers a background task (defined via `expo-task-manager`'s
   * `defineTask`) to be invoked by the OS when a data notification is
   * delivered while the app is backgrounded or killed.
   */
  export function registerTaskAsync(taskName: string): Promise<void>;

  /**
   * Unregisters a previously-registered background notification task.
   */
  export function unregisterTaskAsync(taskName: string): Promise<void>;
}

declare module '@react-native-async-storage/async-storage' {
  // Scoped storage API introduced in async-storage v3.
  interface AsyncStorage {
    getItem(key: string): Promise<string | null>;
    setItem(key: string, value: string): Promise<void>;
    removeItem(key: string): Promise<void>;
    getMany(keys: string[]): Promise<Record<string, string | null>>;
    setMany(entries: Record<string, string>): Promise<void>;
    removeMany(keys: string[]): Promise<void>;
    getAllKeys(): Promise<string[]>;
    clear(): Promise<void>;
  }

  export function createAsyncStorage(databaseName: string): AsyncStorage;
  const AsyncStorage: AsyncStorage;
  export default AsyncStorage;
}

declare module 'expo-sharing' {
  export function isAvailableAsync(): Promise<boolean>;
  export function shareAsync(
    url: string,
    options?: {
      mimeType?: string;
      dialogTitle?: string;
      UTI?: string;
    }
  ): Promise<void>;
}

declare module 'expo-image-picker' {
  export interface ImagePickerOptions {
    mediaTypes?: 'images' | 'videos' | 'all';
    allowsEditing?: boolean;
    aspect?: [number, number];
    quality?: number;
    base64?: boolean;
    exif?: boolean;
    allowsMultipleSelection?: boolean;
    selectionLimit?: number;
    videoMaxDuration?: number;
    videoQuality?: number;
    presentationStyle?:
      | 'fullScreen'
      | 'pageSheet'
      | 'formSheet'
      | 'currentContext'
      | 'overFullScreen'
      | 'overCurrentContext'
      | 'popover';
  }

  export interface ImagePickerAsset {
    uri: string;
    width: number;
    height: number;
    type?: 'image' | 'video';
    fileName?: string;
    fileSize?: number;
    // MIME type the picker reports for the selected asset (e.g. `image/heic`
    // for an iOS capture), or `undefined` when it couldn't be determined.
    // DocumentUploadScreen prefers this over an extension guess (GitHub #2368).
    mimeType?: string;
    base64?: string;
    exif?: Record<string, unknown>;
    duration?: number;
    assetId?: string;
  }

  export interface ImagePickerResult {
    canceled: boolean;
    assets: ImagePickerAsset[];
  }

  export interface PermissionResponse {
    status: 'granted' | 'denied' | 'undetermined';
    expires: 'never' | number;
    granted: boolean;
    canAskAgain: boolean;
  }

  export function requestCameraPermissionsAsync(): Promise<PermissionResponse>;
  export function requestMediaLibraryPermissionsAsync(): Promise<PermissionResponse>;
  export function getCameraPermissionsAsync(): Promise<PermissionResponse>;
  export function getMediaLibraryPermissionsAsync(): Promise<PermissionResponse>;
  export function launchCameraAsync(options?: ImagePickerOptions): Promise<ImagePickerResult>;
  export function launchImageLibraryAsync(options?: ImagePickerOptions): Promise<ImagePickerResult>;
}

declare module 'expo-location' {
  export interface LocationObject {
    coords: {
      latitude: number;
      longitude: number;
      altitude: number | null;
      accuracy: number | null;
      altitudeAccuracy: number | null;
      heading: number | null;
      speed: number | null;
    };
    timestamp: number;
  }

  export interface LocationGeocodedAddress {
    city: string | null;
    country: string | null;
    district: string | null;
    isoCountryCode: string | null;
    name: string | null;
    postalCode: string | null;
    region: string | null;
    street: string | null;
    streetNumber: string | null;
    subregion: string | null;
    timezone: string | null;
  }

  export interface LocationOptions {
    accuracy?: number;
    distanceInterval?: number;
    timeInterval?: number;
    mayShowUserSettingsDialog?: boolean;
  }

  export interface PermissionResponse {
    status: 'granted' | 'denied' | 'undetermined';
    expires: 'never' | number;
    granted: boolean;
    canAskAgain: boolean;
  }

  export function requestForegroundPermissionsAsync(): Promise<PermissionResponse>;
  export function requestBackgroundPermissionsAsync(): Promise<PermissionResponse>;
  export function getForegroundPermissionsAsync(): Promise<PermissionResponse>;
  export function getBackgroundPermissionsAsync(): Promise<PermissionResponse>;
  export function getCurrentPositionAsync(options?: LocationOptions): Promise<LocationObject>;
  export function getLastKnownPositionAsync(
    options?: LocationOptions
  ): Promise<LocationObject | null>;
  export function reverseGeocodeAsync(location: {
    latitude: number;
    longitude: number;
  }): Promise<LocationGeocodedAddress[]>;
  export function geocodeAsync(address: string): Promise<
    Array<{
      latitude: number;
      longitude: number;
      altitude: number | null;
      accuracy: number | null;
    }>
  >;
}

declare module '@react-native-community/netinfo' {
  export interface NetInfoState {
    type: string;
    isConnected: boolean | null;
    isInternetReachable: boolean | null;
    details: unknown;
  }

  export interface NetInfoConfiguration {
    reachabilityUrl?: string;
    reachabilityTest?: (response: Response) => Promise<boolean>;
    reachabilityLongTimeout?: number;
    reachabilityShortTimeout?: number;
    reachabilityRequestTimeout?: number;
    reachabilityShouldRun?: () => boolean;
    shouldFetchWiFiSSID?: boolean;
    useNativeReachability?: boolean;
  }

  interface NetInfo {
    configure(configuration: NetInfoConfiguration): void;
    fetch(requestedInterface?: string): Promise<NetInfoState>;
    refresh(): Promise<NetInfoState>;
    addEventListener(listener: (state: NetInfoState) => void): () => void;
  }

  const netInfo: NetInfo;
  export default netInfo;
  export type { NetInfoState };
}
