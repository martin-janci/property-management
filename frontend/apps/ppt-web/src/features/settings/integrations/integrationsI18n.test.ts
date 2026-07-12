/// <reference types="vitest/globals" />
/**
 * Integrations i18n key-presence regression (#2224, follow-up to #2217).
 *
 * PR #2217 (building on #2194) extracted every Integrations string to
 * `settings.integrations.*`, `nav.integrations`, and
 * `commandPalette.{commands,descriptions}.goToIntegrations` keys — but landed
 * them in `en.json` *only*. The other five locale bundles (sk/cs/de/hu/pl) were
 * untouched, so the entire Integrations surface silently fell back to English
 * for those users. That divergence is invisible to typecheck and lint because
 * the `t(...)` calls degrade gracefully via `fallbackLng`.
 *
 * This test locks the fix in: every key present under `settings.integrations`
 * in `en.json`, plus `nav.integrations` and the two command-palette keys, must
 * exist — and be a non-empty string — in all six locale bundles. It also
 * asserts the interpolated keys keep their `{{placeholder}}` in every locale so
 * a translation can't quietly strip the interpolation.
 *
 * Fails on `main` (sk/cs/de/hu/pl lack the keys); passes once mirrored.
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

/** The full set of integrations leaf-key paths, taken from en.json as the reference. */
const INTEGRATION_PATHS: string[] = [
  'nav.integrations',
  'commandPalette.commands.goToIntegrations',
  'commandPalette.descriptions.goToIntegrations',
  ...leafPaths((en as Json).settings, 'settings').filter((p) =>
    p.startsWith('settings.integrations.')
  ),
];

/** Keys that interpolate a value — must keep their {{placeholder}} per locale. */
const INTERPOLATED: Record<string, string> = {
  'settings.integrations.common.lastSyncError': '{{error}}',
  'settings.integrations.common.itemsSynced': '{{items}}',
  'settings.integrations.booking.conflicts': '{{count}}',
};

describe('integrations i18n keys', () => {
  // Guard the locale set itself: if a new locale bundle is added to the app it
  // must also be added here, or the per-key sweep below would silently skip it.
  it('covers exactly the six shipped locale bundles', () => {
    expect(Object.keys(BUNDLES).sort()).toEqual(['cs', 'de', 'en', 'hu', 'pl', 'sk']);
  });

  it('reference key set is non-trivial', () => {
    // 54 settings.integrations.* leaves + nav + 2 command-palette keys = 57.
    expect(INTEGRATION_PATHS.length).toBeGreaterThanOrEqual(57);
  });

  for (const [locale, bundle] of Object.entries(BUNDLES)) {
    describe(`locale: ${locale}`, () => {
      for (const path of INTEGRATION_PATHS) {
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

  it('keeps every locale in lockstep — no locale has missing integrations keys', () => {
    for (const [locale, bundle] of Object.entries(BUNDLES)) {
      const present = INTEGRATION_PATHS.filter((p) => typeof getPath(bundle, p) === 'string');
      expect(present, `${locale}.json integrations-key set drifted from the reference`).toEqual(
        INTEGRATION_PATHS
      );
    }
  });
});
