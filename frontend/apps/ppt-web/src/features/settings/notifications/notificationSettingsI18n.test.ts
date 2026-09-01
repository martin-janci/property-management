/// <reference types="vitest/globals" />
/**
 * NotificationSettingsPage i18n key-presence regression.
 *
 * The wired Settings > Notifications surface (`NotificationSettingsPage`)
 * previously hard-coded English error strings (e.g.
 * "Failed to update preference. Please try again.") and page chrome directly in
 * the component, so non-English users always saw English. Those strings now
 * route through `t('settings.notifications.*')`.
 *
 * This test locks the fix in: every key under `settings.notifications` in
 * `en.json` must exist — and be a non-empty string — in all six locale
 * bundles (en/sk/cs/de/hu/pl). It also asserts the interpolated `lastUpdated`
 * key keeps its `{{when}}` placeholder in every locale so a translation can't
 * quietly strip the interpolation.
 *
 * Fails on `main` (the `settings.notifications` keys don't exist yet); passes
 * once the keys are mirrored across all locales.
 */

import cs from '../../../../messages/cs.json';
import de from '../../../../messages/de.json';
import en from '../../../../messages/en.json';
import hu from '../../../../messages/hu.json';
import pl from '../../../../messages/pl.json';
import sk from '../../../../messages/sk.json';

type Json = Record<string, unknown>;

const BUNDLES: Record<string, Json> = { en, sk, cs, de, pl, hu };

/** Dotted leaf-key paths reachable from a nested object. */
function leafPaths(obj: unknown, prefix = ''): string[] {
  if (obj === null || typeof obj !== 'object') return [prefix];
  return Object.entries(obj as Json).flatMap(([k, v]) =>
    leafPaths(v, prefix ? `${prefix}.${k}` : k)
  );
}

function getPath(obj: unknown, path: string): unknown {
  return path.split('.').reduce<unknown>((acc, part) => {
    if (acc === null || typeof acc !== 'object') return undefined;
    return (acc as Json)[part];
  }, obj);
}

/** The full set of notification-settings leaf-key paths, from en.json as reference. */
const NOTIFICATION_PATHS: string[] = leafPaths((en as Json).settings, 'settings').filter((p) =>
  p.startsWith('settings.notifications.')
);

/** Keys that interpolate a value — must keep their {{placeholder}} per locale. */
const INTERPOLATED: Record<string, string> = {
  'settings.notifications.lastUpdated': '{{when}}',
};

describe('notification settings i18n keys', () => {
  // Guard the locale set itself: if a new locale bundle is added to the app it
  // must also be added here, or the per-key sweep below would silently skip it.
  it('covers exactly the six shipped locale bundles', () => {
    expect(Object.keys(BUNDLES).sort()).toEqual(['cs', 'de', 'en', 'hu', 'pl', 'sk']);
  });

  it('reference key set is non-trivial', () => {
    // title, subtitle, loadError, updateError, disableError, dismiss, lastUpdated = 7.
    expect(NOTIFICATION_PATHS.length).toBeGreaterThanOrEqual(7);
  });

  for (const [locale, bundle] of Object.entries(BUNDLES)) {
    describe(`locale: ${locale}`, () => {
      for (const path of NOTIFICATION_PATHS) {
        it(`defines a non-empty ${path}`, () => {
          const value = getPath(bundle, path);
          expect(value, `${path} missing in ${locale}.json`).toBeTypeOf('string');
          expect((value as string).trim().length).toBeGreaterThan(0);
        });
      }

      for (const [path, placeholder] of Object.entries(INTERPOLATED)) {
        it(`keeps the ${placeholder} placeholder in ${path}`, () => {
          const value = getPath(bundle, path) as string | undefined;
          expect(value, `${path} missing in ${locale}.json`).toBeTypeOf('string');
          expect(
            (value as string).includes(placeholder),
            `${path} in ${locale}.json dropped ${placeholder}`
          ).toBe(true);
        });
      }
    });
  }

  it('keeps every locale in lockstep — no locale has missing notification-settings keys', () => {
    for (const [locale, bundle] of Object.entries(BUNDLES)) {
      const present = NOTIFICATION_PATHS.filter((p) => typeof getPath(bundle, p) === 'string');
      expect(
        present,
        `${locale}.json notification-settings key set drifted from the reference`
      ).toEqual(NOTIFICATION_PATHS);
    }
  });
});
