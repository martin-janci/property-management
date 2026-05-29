/**
 * withLegacyStoragePermissions
 *
 * Issue #626 — Android permissions for API 33+ (Android 13+).
 *
 * Background:
 *   `READ_EXTERNAL_STORAGE` and `WRITE_EXTERNAL_STORAGE` were deprecated in
 *   Android 10 (API 29) and are silently ignored on Android 13+ (API 33+).
 *   On API 33+ the granular media permissions must be used instead:
 *     - READ_MEDIA_IMAGES
 *     - READ_MEDIA_VIDEO
 *     - READ_MEDIA_AUDIO
 *
 *   Older devices (API <= 32) still need the legacy permissions to read
 *   images/videos/audio from shared storage.
 *
 * What this plugin does:
 *   Re-injects the two legacy permissions into AndroidManifest.xml with
 *   `android:maxSdkVersion="32"` so they apply ONLY on API <= 32 and are
 *   suppressed on API 33+. The granular media permissions for API 33+ are
 *   declared in `app.config.ts` `android.permissions` directly.
 *
 * Why a custom plugin:
 *   Expo's `android.permissions` array does not support the `maxSdkVersion`
 *   attribute. The only way to scope a permission to <= API 32 is to edit
 *   the merged AndroidManifest.xml at prebuild time, which is what this
 *   `withAndroidManifest` mod does.
 */

import { AndroidConfig, type ConfigPlugin, withAndroidManifest } from '@expo/config-plugins';

/**
 * Subset of attributes we read/write on `<uses-permission/>`.
 * `@expo/config-plugins` types only declare `android:name`; the parser
 * accepts any string attribute, so we widen locally.
 */
type UsesPermissionAttributes = {
  'android:name'?: string;
  'android:maxSdkVersion'?: string;
};

const LEGACY_STORAGE_PERMISSIONS = [
  'android.permission.READ_EXTERNAL_STORAGE',
  'android.permission.WRITE_EXTERNAL_STORAGE',
] as const;

const MAX_SDK_LEGACY_STORAGE = '32';

export const withLegacyStoragePermissions: ConfigPlugin = (config) =>
  withAndroidManifest(config, (cfg) => {
    const manifest = cfg.modResults;

    // Ensure <manifest><uses-permission .../></manifest> list exists.
    manifest.manifest['uses-permission'] = manifest.manifest['uses-permission'] ?? [];
    const usesPermissions = manifest.manifest['uses-permission'];

    for (const permName of LEGACY_STORAGE_PERMISSIONS) {
      // If Expo already emitted the bare permission (no maxSdkVersion), strip
      // it so we can re-emit it with maxSdkVersion=32. Keep any other variants
      // (e.g. someone else has already added a different maxSdkVersion).
      for (let i = usesPermissions.length - 1; i >= 0; i--) {
        const entry = usesPermissions[i];
        const attrs = (entry?.$ ?? {}) as UsesPermissionAttributes;
        if (attrs['android:name'] === permName && attrs['android:maxSdkVersion'] === undefined) {
          usesPermissions.splice(i, 1);
        }
      }

      // Skip if a maxSdkVersion-scoped entry already exists.
      const alreadyScoped = usesPermissions.some((entry) => {
        const attrs = (entry?.$ ?? {}) as UsesPermissionAttributes;
        return (
          attrs['android:name'] === permName &&
          attrs['android:maxSdkVersion'] === MAX_SDK_LEGACY_STORAGE
        );
      });
      if (alreadyScoped) continue;

      usesPermissions.push({
        $: {
          'android:name': permName,
          'android:maxSdkVersion': MAX_SDK_LEGACY_STORAGE,
        } as unknown as AndroidConfig.Manifest.ManifestUsesPermission['$'],
      });
    }

    return cfg;
  });

export default withLegacyStoragePermissions;

// Re-export AndroidConfig for downstream typing convenience in tests.
export { AndroidConfig };
