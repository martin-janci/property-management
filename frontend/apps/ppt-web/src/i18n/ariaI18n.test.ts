/// <reference types="vitest/globals" />
/**
 * `aria.*` namespace i18n key-presence regression.
 *
 * 104 hardcoded aria-label / aria-* strings across ppt-web feature components
 * (many English, some stray Slovak) were moved into the shared `aria.*`
 * i18n namespace so assistive tech reads sk/cs/de/pl/hu correctly instead of
 * hearing English (see code-review-ppt-web-ui-aria-label-i18n).
 *
 * Screen-reader labels degrade silently: `t('aria.foo')` falls back to
 * `en.json` via `fallbackLng`, so a key missing from sk/cs/de/pl/hu is
 * invisible to typecheck + lint but announces English to non-English users —
 * the exact bug this work fixed. This test locks it in: every leaf key present
 * under `aria` in `en.json` must exist — and be a non-empty string — in all
 * six shipped locale bundles.
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

/** The full set of aria.* leaf-key paths, taken from en.json as reference. */
const ARIA_PATHS: string[] = leafPaths((en as Json).aria, 'aria');

describe('aria.* i18n keys', () => {
  it('covers exactly the six shipped locale bundles', () => {
    expect(Object.keys(BUNDLES).sort()).toEqual(['cs', 'de', 'en', 'hu', 'pl', 'sk']);
  });

  it('reference key set is non-trivial', () => {
    expect(ARIA_PATHS.length).toBeGreaterThanOrEqual(80);
  });

  for (const [locale, bundle] of Object.entries(BUNDLES)) {
    describe(`locale: ${locale}`, () => {
      for (const path of ARIA_PATHS) {
        it(`defines a non-empty ${path}`, () => {
          const value = getPath(bundle, path);
          expect(value, `${path} missing in ${locale}.json`).toBeTypeOf('string');
          expect((value as string).trim().length).toBeGreaterThan(0);
        });
      }
    });
  }

  it('keeps every locale in lockstep — no locale has missing aria keys', () => {
    for (const [locale, bundle] of Object.entries(BUNDLES)) {
      const present = ARIA_PATHS.filter((p) => typeof getPath(bundle, p) === 'string');
      expect(present, `${locale}.json aria-key set drifted from the reference`).toEqual(ARIA_PATHS);
    }
  });
});
