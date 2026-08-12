/// <reference types="vitest/globals" />
/**
 * `offlineIndicator.*` namespace i18n key-presence regression.
 *
 * The `OfflineIndicator` banner (`src/components/OfflineIndicator.tsx`) shipped
 * with two hardcoded English strings ("You are offline. Some features may be
 * unavailable." and "Connection restored") instead of going through
 * react-i18next like the rest of ppt-web (see
 * code-review-ppt-web-ui-offline-ind-i18n). Non-English users saw English text
 * in a `role="alert"` banner — invisible to typecheck + lint because the
 * literals were valid JSX.
 *
 * The strings now live under the `offlineIndicator` namespace and are consumed
 * via `t('offlineIndicator.offline')` / `t('offlineIndicator.reconnected')`.
 * This test locks it in: every leaf key present under `offlineIndicator` in
 * `en.json` must exist — and be a non-empty string — in all six shipped locale
 * bundles. It fails on `main` (the namespace does not exist in any bundle) and
 * passes once the block is added to all six.
 */

import cs from '../../messages/cs.json';
import de from '../../messages/de.json';
import en from '../../messages/en.json';
import hu from '../../messages/hu.json';
import pl from '../../messages/pl.json';
import sk from '../../messages/sk.json';

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

/** The full set of offlineIndicator.* leaf-key paths, taken from en.json. */
const OFFLINE_PATHS: string[] = leafPaths((en as Json).offlineIndicator, 'offlineIndicator');

describe('offlineIndicator.* i18n keys', () => {
  it('covers exactly the six shipped locale bundles', () => {
    expect(Object.keys(BUNDLES).sort()).toEqual(['cs', 'de', 'en', 'hu', 'pl', 'sk']);
  });

  it('reference key set includes the banner strings', () => {
    expect(OFFLINE_PATHS).toEqual(
      expect.arrayContaining(['offlineIndicator.offline', 'offlineIndicator.reconnected'])
    );
  });

  for (const [locale, bundle] of Object.entries(BUNDLES)) {
    describe(`locale: ${locale}`, () => {
      for (const path of OFFLINE_PATHS) {
        it(`defines a non-empty ${path}`, () => {
          const value = getPath(bundle, path);
          expect(value, `${path} missing in ${locale}.json`).toBeTypeOf('string');
          expect((value as string).trim().length).toBeGreaterThan(0);
        });
      }
    });
  }

  it('keeps every locale in lockstep — no locale has missing offlineIndicator keys', () => {
    for (const [locale, bundle] of Object.entries(BUNDLES)) {
      const present = OFFLINE_PATHS.filter((p) => typeof getPath(bundle, p) === 'string');
      expect(present, `${locale}.json offlineIndicator-key set drifted from the reference`).toEqual(
        OFFLINE_PATHS
      );
    }
  });
});
