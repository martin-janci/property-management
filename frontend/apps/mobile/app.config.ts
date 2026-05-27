/**
 * Expo Dynamic App Configuration
 *
 * Epic 85 - Story 85.1: Environment Variable Setup
 * Epic 85 - Story 85.2: Android + iOS Release Build Configuration
 *
 * Reads environment-specific .env files and injects variables into:
 *   - `extra` block (accessible via expo-constants: Constants.expoConfig.extra)
 *   - `ios.infoPlist` (native iOS app Info.plist keys)
 *   - `android` block (package, permissions, adaptive icon, Google Services)
 *   - `EXPO_PUBLIC_*` env vars are handled automatically by the Expo build
 *     system when loaded via Metro (see metro.config.js)
 *
 * Environment selection:
 *   APP_ENV=development  =>  .env.development  (default in __DEV__ mode)
 *   APP_ENV=staging      =>  .env.staging
 *   APP_ENV=production   =>  .env.production   (default in release builds)
 *
 * Usage:
 *   APP_ENV=staging expo start                # run with staging config
 *   APP_ENV=production expo start             # run with production config
 *
 * EAS Build:
 *   eas build --platform android --profile staging
 *   eas build --platform android --profile production
 *   eas submit --platform android --profile production
 *
 * Android release signing:
 *   Keystore credentials managed via EAS secrets (credentialsSource=remote in eas.json).
 *   For local Gradle builds: set ANDROID_KEYSTORE_PATH, ANDROID_KEY_ALIAS,
 *   ANDROID_KEYSTORE_PASSWORD, ANDROID_KEY_PASSWORD in your shell or .env.local.
 *   See scripts/setup-android-keystore.sh for keystore generation instructions.
 *
 * Android Google Services:
 *   google-services.json must be at frontend/apps/mobile/google-services.json
 *   for Firebase push notifications (FCM). Gitignored -- obtain from Firebase Console
 *   (Project Settings > Android app > Download google-services.json).
 *   EAS Cloud builds receive it via the GOOGLE_SERVICES_JSON secret (base64-encoded).
 *   Template with placeholder values lives at google-services.json.template.
 */

import * as fs from 'node:fs';
import * as path from 'node:path';
import * as dotenv from 'dotenv';
import type { ConfigContext, ExpoConfig } from 'expo/config';

/** Supported environments */
type AppEnvironment = 'development' | 'staging' | 'production';

function loadEnvFile(env: AppEnvironment): dotenv.DotenvParseOutput {
  const envFile = path.resolve(__dirname, `.env.${env}`);
  const result = dotenv.config({ path: envFile });
  if (result.error) {
    console.warn(`[app.config.ts] Could not load ${envFile}: ${result.error.message}`);
    return {};
  }
  return result.parsed ?? {};
}

function getAppEnv(): AppEnvironment {
  const raw = process.env.APP_ENV ?? '';
  if (raw === 'staging' || raw === 'production' || raw === 'development') {
    return raw;
  }
  // Fall back to NODE_ENV-based detection
  return process.env.NODE_ENV === 'production' ? 'production' : 'development';
}

/**
 * Returns true when a real google-services.json exists beside this config file.
 * EAS Cloud builds inject this via GOOGLE_SERVICES_JSON secret; local dev
 * without Firebase falls back gracefully so `expo prebuild` does not fail.
 */
function hasGoogleServicesJson(): boolean {
  const p = path.resolve(__dirname, 'google-services.json');
  return fs.existsSync(p);
}

export default ({ config }: ConfigContext): ExpoConfig => {
  const appEnv = getAppEnv();
  const envVars = loadEnvFile(appEnv);

  // Resolved values (env file takes priority, then process.env fallback)
  const apiBaseUrl =
    envVars.API_BASE_URL ?? process.env.EXPO_PUBLIC_API_BASE_URL ?? 'https://api.ppt.example.com';

  const wsBaseUrl =
    envVars.WS_BASE_URL ?? process.env.EXPO_PUBLIC_WS_BASE_URL ?? apiBaseUrl.replace(/^http/, 'ws');

  const environment: AppEnvironment = (envVars.ENVIRONMENT as AppEnvironment | undefined) ?? appEnv;

  const debugMode =
    envVars.DEBUG_MODE === 'true' || envVars.DEBUG_MODE === '1' || appEnv === 'development';

  // Google Services JSON path -- only set when real file is present.
  // EAS Cloud injects via GOOGLE_SERVICES_JSON secret; omitted for plain local dev.
  const googleServicesFile = hasGoogleServicesJson() ? './google-services.json' : undefined;

  return {
    ...config,
    name: config.name ?? 'PPT Management',
    slug: config.slug ?? 'ppt-management',
    version: config.version ?? '0.2.627',
    orientation: 'portrait',
    scheme: 'ppt-management',
    platforms: ['ios', 'android'],

    ios: {
      ...config.ios,
      bundleIdentifier: 'three.two.bit.ppt.management',
      /**
       * EAS 85.2: iOS build number (CFBundleVersion).
       * Auto-incremented by EAS (autoIncrement in eas.json production profile).
       * JS-side baseline only; actual value in .ipa is managed by EAS Build.
       * iOS provisioning (distribution cert + provisioning profile) stored in
       * EAS remote credential store — never committed. Manage via:
       *   eas credentials --platform ios
       * App Store Connect API key set once in EAS (Key ID + Issuer ID + p8).
       */
      buildNumber: config.ios?.buildNumber ?? '1',
      infoPlist: {
        ...(config.ios?.infoPlist ?? {}),
        API_BASE_URL: apiBaseUrl,
        ENVIRONMENT: environment,
        NSPhotoLibraryUsageDescription:
          'The app needs access to your photos to upload property images.',
        NSCameraUsageDescription:
          'The app needs access to your camera to take photos of properties.',
        NSLocationWhenInUseUsageDescription:
          'The app needs your location to show nearby properties.',
        /**
         * EAS 85.2: App Transport Security (ATS).
         * Production enforces HTTPS; debug/staging allows arbitrary loads for
         * internal HTTP endpoints without signed TLS certificates.
         */
        NSAppTransportSecurity: debugMode
          ? { NSAllowsArbitraryLoads: true }
          : { NSAllowsArbitraryLoads: false },
      },
    },

    android: {
      ...config.android,
      package: 'three.two.bit.ppt.management',

      // -----------------------------------------------------------------------
      // Epic 85 - Story 85.2: Android Release Build Configuration
      // -----------------------------------------------------------------------

      // versionCode: auto-incremented by EAS (autoIncrement:true in eas.json).
      // JS-side fallback only; actual versionCode in APK/AAB managed by EAS.
      versionCode: config.android?.versionCode ?? 1,

      // Adaptive icon -- required for Android 8.0+ (API level 26+).
      // Asset: assets/images/adaptive-icon.png (1024x1024, transparent bg).
      adaptiveIcon: {
        foregroundImage: './assets/images/adaptive-icon.png',
        backgroundColor: '#1A73E8',
      },

      // Google Services JSON for FCM (Android push notifications).
      // EAS Cloud: via GOOGLE_SERVICES_JSON secret (base64-encoded).
      //   eas secret:create --scope project --name GOOGLE_SERVICES_JSON \
      //       --value "$(base64 -w0 google-services.json)"
      // Local: place google-services.json beside this file (gitignored).
      ...(googleServicesFile !== undefined ? { googleServicesFile } : {}),

      // Permissions declared in AndroidManifest.xml.
      permissions: [
        'android.permission.CAMERA',
        'android.permission.READ_EXTERNAL_STORAGE',
        'android.permission.WRITE_EXTERNAL_STORAGE',
        'android.permission.ACCESS_FINE_LOCATION',
        'android.permission.ACCESS_COARSE_LOCATION',
        'android.permission.RECEIVE_BOOT_COMPLETED',
        'android.permission.VIBRATE',
        'android.permission.POST_NOTIFICATIONS',
        'android.permission.INTERNET',
        'android.permission.ACCESS_NETWORK_STATE',
        'android.permission.USE_BIOMETRIC',
        'android.permission.USE_FINGERPRINT',
        'android.permission.NFC',
      ],

      // Intent filters: ppt-management:// custom scheme + App Links.
      // App Links require assetlinks.json on server with keystore SHA-256.
      intentFilters: [
        {
          action: 'VIEW',
          autoVerify: true,
          data: [{ scheme: 'ppt-management' }],
          category: ['BROWSABLE', 'DEFAULT'],
        },
        {
          action: 'VIEW',
          autoVerify: true,
          data: [{ scheme: 'https', host: 'app.ppt.example.com', pathPrefix: '/' }],
          category: ['BROWSABLE', 'DEFAULT'],
        },
      ],
    },

    plugins: [
      'expo-localization',
      'expo-secure-store',
      // expo-notifications: FCM (Android) + APNs (iOS).
      // Android: requires google-services.json (see googleServicesFile above).
      [
        'expo-notifications',
        { icon: './assets/images/notification-icon.png', color: '#1A73E8', sounds: [] },
      ],
    ],

    extra: {
      ...config.extra,
      API_BASE_URL: apiBaseUrl,
      WS_BASE_URL: wsBaseUrl,
      ENVIRONMENT: environment,
      DEBUG_MODE: debugMode,
      /**
       * EAS 85.2: EAS project ID at runtime for update channel routing.
       * Populated from EXPO_PROJECT_ID env var (set in CI via GitHub secret).
       */
      eas: {
        projectId: process.env.EXPO_PROJECT_ID ?? '',
      },
    },

    /**
     * EAS 85.2: OTA update policy.
     * Builds sharing the same appVersion receive OTA updates from the same channel.
     * Channel is set per-build via `channel` in eas.json profiles.
     * Updates disabled for local development to avoid stale bundle issues.
     */
    runtimeVersion: {
      policy: 'appVersion',
    },

    updates: {
      url: (() => {
        const id = process.env.EXPO_PROJECT_ID;
        if (!id && environment !== 'development') {
          console.warn(
            '[app.config.ts] EXPO_PROJECT_ID is unset — OTA updates will not work in this build'
          );
        }
        return `https://u.expo.dev/${id ?? ''}`;
      })(),
      fallbackToCacheTimeout: 0,
      enabled: environment !== 'development',
    },
  };
};
