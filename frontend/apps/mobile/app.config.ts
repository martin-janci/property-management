/**
 * Expo Dynamic App Configuration
 *
 * Epic 85 - Story 85.1: Environment Variable Setup
 *
 * Reads environment-specific .env files and injects variables into:
 *   - `extra` block (accessible via expo-constants: Constants.expoConfig.extra)
 *   - `ios.infoPlist` (native iOS Info.plist keys: API_BASE_URL, ENVIRONMENT)
 *
 * Environment selection via APP_ENV:
 *   APP_ENV=development  (default in __DEV__ mode) -> loads .env.development
 *   APP_ENV=staging                                -> loads .env.staging
 *   APP_ENV=production  (default in release builds) -> loads .env.production
 *
 * Usage:
 *   pnpm -F mobile start                        # loads .env.development
 *   APP_ENV=staging pnpm -F mobile start        # loads .env.staging
 *   APP_ENV=production pnpm -F mobile start     # loads .env.production
 */

import * as path from 'node:path';
import type { ConfigContext, ExpoConfig } from 'expo/config';
import * as dotenv from 'dotenv';

type AppEnvironment = 'development' | 'staging' | 'production';

function getAppEnv(): AppEnvironment {
  const raw = process.env.APP_ENV ?? '';
  if (raw === 'staging' || raw === 'production' || raw === 'development') {
    return raw;
  }
  return process.env.NODE_ENV === 'production' ? 'production' : 'development';
}

function loadEnvFile(env: AppEnvironment): Record<string, string> {
  const envFile = path.resolve(__dirname, `.env.${env}`);
  const result = dotenv.config({ path: envFile });
  if (result.error) {
    // Non-fatal: fall back to process.env values
    console.warn(`[app.config.ts] Could not load ${envFile}: ${result.error.message}`);
    return {};
  }
  return result.parsed ?? {};
}

export default ({ config }: ConfigContext): ExpoConfig => {
  const appEnv = getAppEnv();
  const envVars = loadEnvFile(appEnv);

  const apiBaseUrl =
    envVars.API_BASE_URL ??
    process.env.EXPO_PUBLIC_API_BASE_URL ??
    'https://api.ppt.example.com';

  const wsBaseUrl =
    envVars.WS_BASE_URL ??
    process.env.EXPO_PUBLIC_WS_BASE_URL ??
    apiBaseUrl.replace(/^http/, 'ws');

  const environment: AppEnvironment =
    (envVars.ENVIRONMENT as AppEnvironment | undefined) ?? appEnv;

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
      infoPlist: {
        ...(config.ios?.infoPlist ?? {}),
        /**
         * API_BASE_URL - injected into iOS Info.plist at build time.
         * Prefer reading via Constants.expoConfig.extra.API_BASE_URL in JS code.
         */
        API_BASE_URL: apiBaseUrl,
        /**
         * ENVIRONMENT - identifies the runtime environment natively.
         * Values: development | staging | production
         */
        ENVIRONMENT: environment,
        NSPhotoLibraryUsageDescription:
          'The app needs access to your photos to upload property images.',
        NSCameraUsageDescription:
          'The app needs access to your camera to take photos of properties.',
        NSLocationWhenInUseUsageDescription:
          'The app needs your location to show nearby properties.',
      },
    },

    android: {
      ...config.android,
      package: 'three.two.bit.ppt.management',
    },

    plugins: ['expo-localization', 'expo-secure-store'],

    /**
     * extra - accessible in JS via:
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
    },
  };
};
