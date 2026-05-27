/**
 * Expo Dynamic App Configuration
 *
 * Epic 85 - Story 85.1: Environment Variable Setup
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

/** Sentinel value written into .env.production — must be overridden by CI. */
const PROD_URL_SENTINEL = '__SET_BY_CI__';

/** RFC 2606 example domains — never valid in production. */
function isPlaceholderUrl(value: string): boolean {
  return value === PROD_URL_SENTINEL || /(^|\.)example\.(com|net|org)(:|\/|$)/i.test(value);
}

export default ({ config }: ConfigContext): ExpoConfig => {
  const appEnv = getAppEnv();
  const envVars = loadEnvFile(appEnv);

  // Resolved values — single source of truth is the .env.<appEnv> file loaded
  // via dotenv above. CI may also pre-seed process.env (e.g. eas.json `env`)
  // for the same names without the EXPO_PUBLIC_ prefix.
  const apiBaseUrl = envVars.API_BASE_URL ?? process.env.API_BASE_URL ?? 'http://localhost:8080';

  const wsBaseUrl =
    envVars.WS_BASE_URL ?? process.env.WS_BASE_URL ?? apiBaseUrl.replace(/^http/, 'ws');

  // Fail fast if a production build is about to ship with a placeholder URL.
  if (appEnv === 'production' && (isPlaceholderUrl(apiBaseUrl) || isPlaceholderUrl(wsBaseUrl))) {
    throw new Error(
      '[app.config.ts] Refusing to build production with placeholder API URL ' +
        `(API_BASE_URL=${apiBaseUrl}, WS_BASE_URL=${wsBaseUrl}). ` +
        'Override via CI (e.g. eas.json env) before building.'
    );
  }

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
    },
  };
};
