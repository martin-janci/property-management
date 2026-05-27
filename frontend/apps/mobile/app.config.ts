/**
 * Expo Dynamic App Configuration
 *
 * Epic 85 - Story 85.1: Environment Variable Setup
 * Epic 85 - Story 85.2: iOS Release Build Configuration
 *
 * Reads environment-specific .env files and injects variables into:
 *   - `extra` block (accessible via expo-constants: Constants.expoConfig.extra)
 *   - `ios.infoPlist` (native iOS app Info.plist keys)
 *   - `EXPO_PUBLIC_*` env vars are handled automatically by the Expo build
 *     system when loaded via Metro (see metro.config.js)
 *
 * Environment selection:
 *   APP_ENV=development  →  .env.development  (default in __DEV__ mode)
 *   APP_ENV=staging      →  .env.staging
 *   APP_ENV=production   →  .env.production   (default in release builds)
 *
 * Usage:
 *   APP_ENV=staging expo start      # run with staging config
 *   APP_ENV=production expo build   # build with production config
 */

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
        // --------------- environment variable keys exposed natively ---------------
        /**
         * API base URL injected at build time.
         * Readable via NativeModules or as an Info.plist lookup.
         * Prefer reading via expo-constants (Constants.expoConfig.extra) in JS.
         */
        API_BASE_URL: apiBaseUrl,
        /**
         * Current environment identifier: development | staging | production
         */
        ENVIRONMENT: environment,
        // --------------------------------------------------------------------------
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
    },

    plugins: ['expo-localization', 'expo-secure-store'],

    /**
     * `extra` block — accessible anywhere in JS via:
     *   import Constants from 'expo-constants';
     *   Constants.expoConfig?.extra?.API_BASE_URL
     *
     * See src/config/api.ts which reads these values.
     */
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
