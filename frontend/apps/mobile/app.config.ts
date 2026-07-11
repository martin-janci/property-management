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
// gap-85-2 (iOS build config): injects the per-environment iOS .xcconfig
// (ios/xcconfig/{Development,Staging,Production}.xcconfig) into the generated
// `ios/` project at prebuild and wires it as the baseConfigurationReference of
// every Xcode build configuration. See plugins/withIosBuildConfig.ts.
//
// `resolveAppEnv` is re-used here as the single source of truth for the
// APP_ENV -> environment mapping: the JS runtime layer (this file) and the
// native xcconfig build layer (the plugin) must never disagree about which
// environment a build is. Previously app.config.ts kept a private byte-for-byte
// copy (`getAppEnv`) that was untested and could silently diverge — see #2204.
import withIosBuildConfig, { resolveAppEnv } from './plugins/withIosBuildConfig';
// Custom Expo config plugin -- re-adds legacy storage perms with
// maxSdkVersion=32 so they apply only on API <= 32. Issue #626.
import withLegacyStoragePermissions from './plugins/withLegacyStoragePermissions';

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

/**
 * gap-85-2 (app icon variants): per-environment launcher icons.
 *
 * So a tester can tell a development / staging build apart from production on
 * the home screen at a glance, each environment ships its own badged icon:
 *
 *   development  =>  assets/images/icon-dev.png      / adaptive-icon-dev.png     (DEV badge)
 *   staging      =>  assets/images/icon-staging.png  / adaptive-icon-staging.png (STG badge)
 *   production   =>  assets/images/icon.png          / adaptive-icon.png         (no badge)
 *
 * The badged PNGs are *design assets* dropped into `assets/images/` by the
 * design team (1024x1024). They are intentionally not generated in code. If a
 * badged variant is missing (e.g. a fresh checkout that only has the base
 * production icon yet), we fall back to the un-badged production asset so
 * `expo prebuild` / EAS build never fails on a missing file. The fallback is
 * logged so the gap is visible in build output.
 *
 * See assets/images/README.md for the icon-asset convention.
 */
const ICON_DIR = 'assets/images';

interface IconVariant {
  /** Square launcher icon (iOS + Android legacy), relative project path. */
  icon: string;
  /** Android adaptive-icon foreground layer, relative project path. */
  adaptiveForeground: string;
}

/** Base (production, un-badged) icon assets — the fallback for every env. */
const BASE_ICON_VARIANT: IconVariant = {
  icon: `./${ICON_DIR}/icon.png`,
  adaptiveForeground: `./${ICON_DIR}/adaptive-icon.png`,
};

/** Per-environment badged-icon basenames (suffix appended before `.png`). */
const ICON_SUFFIX_BY_ENV: Record<AppEnvironment, string> = {
  development: '-dev',
  staging: '-staging',
  production: '',
};

/** Returns true when an icon asset exists on disk beside this config. */
function iconAssetExists(relativePath: string): boolean {
  // relativePath is like './assets/images/icon-dev.png'
  return fs.existsSync(path.resolve(__dirname, relativePath));
}

/**
 * Resolve the launcher-icon variant for the given environment, falling back to
 * the un-badged production assets when the badged variant file is absent.
 */
function resolveIconVariant(env: AppEnvironment): IconVariant {
  const suffix = ICON_SUFFIX_BY_ENV[env];
  if (suffix === '') {
    return BASE_ICON_VARIANT;
  }

  const candidate: IconVariant = {
    icon: `./${ICON_DIR}/icon${suffix}.png`,
    adaptiveForeground: `./${ICON_DIR}/adaptive-icon${suffix}.png`,
  };

  const resolved: IconVariant = {
    icon: iconAssetExists(candidate.icon) ? candidate.icon : BASE_ICON_VARIANT.icon,
    adaptiveForeground: iconAssetExists(candidate.adaptiveForeground)
      ? candidate.adaptiveForeground
      : BASE_ICON_VARIANT.adaptiveForeground,
  };

  if (
    resolved.icon !== candidate.icon ||
    resolved.adaptiveForeground !== candidate.adaptiveForeground
  ) {
    console.warn(
      `[app.config.ts] Badged ${env} icon variant missing — falling back to the un-badged ` +
        `production icon (expected ${candidate.icon} / ${candidate.adaptiveForeground}). ` +
        'Add the badged asset under assets/images/ — see assets/images/README.md.'
    );
  }

  return resolved;
}

/**
 * Sentinel value written into `.env.production` — must be overridden by CI
 * (e.g. eas.json `env` or `eas secret:create`) before a production build is
 * shipped. See issue #620 / #523.
 */
const PROD_URL_SENTINEL = '__SET_BY_CI__';

/**
 * Returns true for known-placeholder values that must never reach a production
 * `.ipa` / `.aab`: the CI sentinel above, or any RFC 2606 example domain.
 */
function isPlaceholderUrl(value: string): boolean {
  return value === PROD_URL_SENTINEL || /(^|\.)example\.(com|net|org)(:|\/|$)/i.test(value);
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
  const appEnv = resolveAppEnv();
  const envVars = loadEnvFile(appEnv);

  // Resolved values — single source of truth is the `.env.<appEnv>` file loaded
  // via dotenv above. CI may also pre-seed process.env (e.g. eas.json `env` or
  // `eas secret:create`) for the same names. We deliberately do NOT use the
  // `EXPO_PUBLIC_` prefix here: those vars are inlined by Metro into JS only
  // and cannot reach `ios.infoPlist`, which would force hand-syncing two paths.
  // See issue #523 (architecture decision) and #620 (PR #535 regression).
  const apiBaseUrl = envVars.API_BASE_URL ?? process.env.API_BASE_URL ?? 'http://localhost:8080';

  const wsBaseUrl =
    envVars.WS_BASE_URL ?? process.env.WS_BASE_URL ?? apiBaseUrl.replace(/^http/, 'ws');

  // Fail fast if a production build is about to ship with a placeholder URL.
  // Without this guard a misconfigured CI run can silently produce a signed
  // `.ipa` / `.aab` that points at `__SET_BY_CI__` or `*.example.com` —
  // see issue #620 (regression from PR #535, originally introduced in #523).
  if (appEnv === 'production' && (isPlaceholderUrl(apiBaseUrl) || isPlaceholderUrl(wsBaseUrl))) {
    throw new Error(
      '[app.config.ts] Refusing to build production with placeholder API URL ' +
        `(API_BASE_URL=${apiBaseUrl}, WS_BASE_URL=${wsBaseUrl}). ` +
        'Override via CI (e.g. eas.json env or eas secret:create) before building.'
    );
  }

  const environment: AppEnvironment = (envVars.ENVIRONMENT as AppEnvironment | undefined) ?? appEnv;

  const debugMode =
    envVars.DEBUG_MODE === 'true' || envVars.DEBUG_MODE === '1' || appEnv === 'development';

  // Google Services JSON path -- only set when real file is present.
  // EAS Cloud injects via GOOGLE_SERVICES_JSON secret; omitted for plain local dev.
  const googleServicesFile = hasGoogleServicesJson() ? './google-services.json' : undefined;

  // -------------------------------------------------------------------------
  // gap-85-3: Universal Links (iOS) / App Links (Android) host.
  // -------------------------------------------------------------------------
  // The HTTPS host that the app associates with deep links. Single source of
  // truth is the `APP_LINK_HOST` env var (per-env in .env.<env>), falling back
  // to the convention placeholder domain. This SAME host must be used in:
  //   - iOS `associatedDomains` (`applinks:<host>`) below
  //   - the Android `https` intentFilter below
  //   - the served well-known files:
  //       public/.well-known/apple-app-site-association
  //       public/.well-known/assetlinks.json
  //   - the JS route matcher (src/qrcode/universalLinks.ts → extra.appLinkHost)
  const appLinkHost = envVars.APP_LINK_HOST ?? process.env.APP_LINK_HOST ?? 'app.ppt.example.com';

  // gap-85-2: per-environment launcher icon (DEV / STG badge vs. clean prod).
  const iconVariant = resolveIconVariant(appEnv);

  return {
    ...config,
    name: config.name ?? 'PPT Management',
    slug: config.slug ?? 'ppt-management',
    version: config.version ?? '0.2.627',
    orientation: 'portrait',
    scheme: 'ppt-management',
    platforms: ['ios', 'android'],

    // gap-85-2: top-level launcher icon — used by iOS and as the Android
    // legacy (pre-adaptive) icon. Per-environment badged variant resolved above.
    icon: iconVariant.icon,

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
      /**
       * gap-85-3: Universal Links entitlement.
       * `applinks:<host>` lets iOS route HTTPS taps on `https://<host>/...` to
       * the app when the apple-app-site-association file is served from that
       * host (see public/.well-known/apple-app-site-association). Expo writes
       * this into the generated `<app>.entitlements` at prebuild time.
       */
      associatedDomains: [...(config.ios?.associatedDomains ?? []), `applinks:${appLinkHost}`],
      /**
       * feat-mobile-push: APNs entitlement (required for iOS push).
       * Without `aps-environment`, iOS never issues an APNs device token, so
       * `Notifications.getDevicePushTokenAsync()` in `usePushNotifications`
       * cannot register the device with the backend notification pipeline.
       * `development` for debug/staging (APNs sandbox gateway), `production`
       * for release builds (APNs production gateway). Expo writes this into the
       * generated `<app>.entitlements` at prebuild time, alongside the
       * universal-links entitlement above.
       */
      entitlements: {
        ...(config.ios?.entitlements ?? {}),
        'aps-environment': debugMode ? 'development' : 'production',
      },
      infoPlist: {
        ...(config.ios?.infoPlist ?? {}),
        API_BASE_URL: apiBaseUrl,
        ENVIRONMENT: environment,
        /**
         * feat-mobile-push: enable background/silent push delivery.
         * `remote-notification` lets iOS wake the app for background (data-only)
         * FCM/APNs pushes so the notification-preference pipeline's background
         * path runs — e.g. `syncBadgeFromData` mirroring the badge count onto
         * the app icon for data-only messages. Without it, iOS drops the wake
         * event and only foreground/tapped notifications are handled.
         */
        UIBackgroundModes: [
          ...((config.ios?.infoPlist?.UIBackgroundModes as string[] | undefined) ?? []),
          'remote-notification',
        ],
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
      // gap-85-2: foreground layer is the per-environment badged variant
      // (DEV / STG) resolved above, falling back to the un-badged prod asset.
      adaptiveIcon: {
        foregroundImage: iconVariant.adaptiveForeground,
        backgroundColor: '#1A73E8',
      },

      // Google Services JSON for FCM (Android push notifications).
      // EAS Cloud: via GOOGLE_SERVICES_JSON secret (base64-encoded).
      //   eas secret:create --scope project --name GOOGLE_SERVICES_JSON \
      //       --value "$(base64 -w0 google-services.json)"
      // Local: place google-services.json beside this file (gitignored).
      ...(googleServicesFile !== undefined ? { googleServicesFile } : {}),

      // Permissions declared in AndroidManifest.xml.
      //
      // Issue #626 -- API 33+ (Android 13+) deprecates broad external-storage
      // perms and replaces them with granular media perms. The Play Store
      // requires targetSdkVersion >= 34 for new submissions; Expo SDK 55
      // (used here, see package.json) defaults compileSdkVersion=35 and
      // targetSdkVersion=34 via the prebuild-generated Gradle config, so
      // no override via `expo-build-properties` is required. The granular
      // perms below are the source of truth on modern devices.
      //
      // Legacy READ_EXTERNAL_STORAGE / WRITE_EXTERNAL_STORAGE are still
      // needed on API <= 32 (Android 12L and older). They are re-injected
      // via the `withLegacyStoragePermissions` plugin (registered below)
      // with `android:maxSdkVersion="32"` so they are automatically
      // suppressed on API 33+.
      permissions: [
        'android.permission.CAMERA',
        // API 33+ granular media permissions (replace READ_EXTERNAL_STORAGE).
        'android.permission.READ_MEDIA_IMAGES',
        'android.permission.READ_MEDIA_VIDEO',
        'android.permission.READ_MEDIA_AUDIO',
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
          // gap-85-3: HTTPS App Link. autoVerify drives Android to fetch
          // https://<host>/.well-known/assetlinks.json and verify the signing
          // SHA-256 before claiming the link without a chooser dialog.
          action: 'VIEW',
          autoVerify: true,
          data: [{ scheme: 'https', host: appLinkHost, pathPrefix: '/' }],
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
      // Issue #626 -- re-injects READ/WRITE_EXTERNAL_STORAGE into
      // AndroidManifest.xml with android:maxSdkVersion="32" so older
      // devices (API <= 32) keep working while API 33+ uses the granular
      // READ_MEDIA_* permissions declared in `android.permissions` above.
      withLegacyStoragePermissions,
      // gap-85-2: inject the per-environment iOS .xcconfig into the generated
      // `ios/` project at prebuild (selected by APP_ENV) and wire it as the
      // baseConfigurationReference of every Xcode build configuration.
      withIosBuildConfig,
    ],

    extra: {
      ...config.extra,
      API_BASE_URL: apiBaseUrl,
      WS_BASE_URL: wsBaseUrl,
      ENVIRONMENT: environment,
      DEBUG_MODE: debugMode,
      // gap-85-3: HTTPS host the app claims for universal links / App Links.
      // Read in JS via src/qrcode/universalLinks.ts → getAppLinkHost().
      appLinkHost,
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
      // Guard: only set url when EXPO_PROJECT_ID is present.
      // An empty or missing ID would produce a broken URL ("https://u.expo.dev/")
      // that prevents the build from resolving the OTA update channel.
      ...(process.env.EXPO_PROJECT_ID
        ? { url: `https://u.expo.dev/${process.env.EXPO_PROJECT_ID}` }
        : (() => {
            if (environment !== 'development') {
              console.warn(
                '[app.config.ts] EXPO_PROJECT_ID is unset — updates.url omitted; OTA updates will not work in this build'
              );
            }
            return {};
          })()),
      fallbackToCacheTimeout: 0,
      enabled: environment !== 'development',
    },
  };
};
