/// <reference types="vitest/globals" />
/**
 * `ProtectedRoute` i18n regression — defaultValue-guard parity + locale lockstep.
 *
 * PR #2737 (code-review-ppt-web-ui-protectedroute-i18n) migrated the
 * `ProtectedRoute` component (`src/components/ProtectedRoute.tsx`) to
 * react-i18next but shipped with NO regression test — unlike its sibling
 * ppt-web/mobile i18n PRs (#2736 OfflineIndicator, #2739 MeterDetail), which
 * each locked their strings in with a lockstep bundle test. It also left two of
 * its four `t(...)` calls (`common.loading`, `errors.unauthorized`) WITHOUT the
 * `defaultValue` guard that the other two keys (`accessibility.checkingAuthentication`,
 * `errors.accessDenied`) carry. An unguarded `t('missing.key')` renders the raw
 * key string to the user if the key is ever dropped from the bundle — invisible
 * to typecheck + lint. This follow-up (Closes #2754) adds the guards and this test.
 *
 * Two things are locked in here:
 *   1. Guard parity — every `t('...')` call in ProtectedRoute.tsx passes a
 *      `defaultValue`, so a dropped key degrades to readable English rather
 *      than a raw dotted key. Fails on `dev` (two calls unguarded), passes
 *      once the guards are added.
 *   2. Locale lockstep — the keys ProtectedRoute actually resolves from the
 *      bundles (`common.loading`, `errors.unauthorized`) exist and are
 *      non-empty in all six shipped locales, matching the sibling PRs' test.
 */

import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import cs from '../../messages/cs.json';
import de from '../../messages/de.json';
import en from '../../messages/en.json';
import hu from '../../messages/hu.json';
import pl from '../../messages/pl.json';
import sk from '../../messages/sk.json';

type Json = Record<string, unknown>;

const BUNDLES: Record<string, Json> = { en, sk, cs, de, pl, hu };

/** Keys ProtectedRoute resolves from the shipped locale bundles (not defaultValue-only). */
const BUNDLE_KEYS = ['common.loading', 'errors.unauthorized'] as const;

function getPath(obj: unknown, path: string): unknown {
  return path.split('.').reduce<unknown>((acc, part) => {
    if (acc === null || typeof acc !== 'object') return undefined;
    return (acc as Json)[part];
  }, obj);
}

const PROTECTED_ROUTE_SRC = readFileSync(
  join(process.cwd(), 'src', 'components', 'ProtectedRoute.tsx'),
  'utf8'
);

/** Extract the first string-literal argument of every `t('...')` call. */
function translationKeys(src: string): string[] {
  return [...src.matchAll(/\bt\(\s*['"]([^'"]+)['"]/g)].map((m) => m[1]);
}

/** For a given key, does its `t('key'` call carry a `defaultValue` before the call closes? */
function isGuarded(src: string, key: string): boolean {
  const escaped = key.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const call = new RegExp(`\\bt\\(\\s*['"]${escaped}['"][\\s\\S]*?\\)`, 'm').exec(src);
  return call != null && /defaultValue\s*:/.test(call[0]);
}

describe('ProtectedRoute i18n', () => {
  it('covers exactly the six shipped locale bundles', () => {
    expect(Object.keys(BUNDLES).sort()).toEqual(['cs', 'de', 'en', 'hu', 'pl', 'sk']);
  });

  it('finds the ProtectedRoute translation calls', () => {
    const keys = translationKeys(PROTECTED_ROUTE_SRC);
    expect(keys).toEqual(
      expect.arrayContaining([
        'accessibility.checkingAuthentication',
        'common.loading',
        'errors.accessDenied',
        'errors.unauthorized',
      ])
    );
  });

  it('guards every t() call with a defaultValue (parity — no raw-key leaks)', () => {
    const keys = translationKeys(PROTECTED_ROUTE_SRC);
    const unguarded = keys.filter((k) => !isGuarded(PROTECTED_ROUTE_SRC, k));
    expect(unguarded, `t() calls missing a defaultValue guard: ${unguarded.join(', ')}`).toEqual(
      []
    );
  });

  for (const [locale, bundle] of Object.entries(BUNDLES)) {
    describe(`locale: ${locale}`, () => {
      for (const path of BUNDLE_KEYS) {
        it(`defines a non-empty ${path}`, () => {
          const value = getPath(bundle, path);
          expect(value, `${path} missing in ${locale}.json`).toBeTypeOf('string');
          expect((value as string).trim().length).toBeGreaterThan(0);
        });
      }
    });
  }

  it('keeps every locale in lockstep for the bundle-resolved keys', () => {
    for (const [locale, bundle] of Object.entries(BUNDLES)) {
      const present = BUNDLE_KEYS.filter((p) => typeof getPath(bundle, p) === 'string');
      expect(present, `${locale}.json ProtectedRoute-key set drifted from the reference`).toEqual([
        ...BUNDLE_KEYS,
      ]);
    }
  });
});
