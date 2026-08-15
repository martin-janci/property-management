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
import i18next, { type i18n as I18nInstance } from 'i18next';
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

/**
 * Keys ProtectedRoute renders which are intentionally NOT in any bundle — they
 * ride entirely on the `defaultValue` guard. If one of these ever leaks into a
 * bundle it must land in ALL six (see `deadNamespaceI18n.test.ts` for the
 * inverse-parity bug that a partial add causes), so the classification test
 * below asserts they stay absent everywhere.
 */
const DEFAULTVALUE_ONLY_KEYS = [
  'accessibility.checkingAuthentication',
  'errors.accessDenied',
] as const;

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

/** Extract the `defaultValue` string literal from a key's `t('key', { defaultValue: '...' })` call. */
function defaultValueOf(src: string, key: string): string | undefined {
  const escaped = key.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const call = new RegExp(`\\bt\\(\\s*['"]${escaped}['"][\\s\\S]*?\\)`, 'm').exec(src);
  if (call == null) return undefined;
  const dv = /defaultValue\s*:\s*(['"])([\s\S]*?)\1/.exec(call[0]);
  return dv?.[2];
}

/**
 * A standalone i18next instance mirroring `src/i18n/index.ts` (six bundles,
 * `fallbackLng: 'en'`), so the tests exercise the SAME resolution + fallback
 * path the component sees at runtime rather than only reading the raw JSON.
 * `createInstance` keeps it isolated from the app-global i18n singleton.
 */
function makeI18n(): I18nInstance {
  const instance = i18next.createInstance();
  instance.init({
    resources: {
      en: { translation: en },
      sk: { translation: sk },
      cs: { translation: cs },
      de: { translation: de },
      pl: { translation: pl },
      hu: { translation: hu },
    },
    lng: 'en',
    fallbackLng: 'en',
    supportedLngs: ['en', 'sk', 'cs', 'de', 'pl', 'hu'],
    interpolation: { escapeValue: false },
  });
  return instance;
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

/**
 * Key classification integrity.
 *
 * The two lists above (`BUNDLE_KEYS`, `DEFAULTVALUE_ONLY_KEYS`) encode an
 * assumption about which of ProtectedRoute's four `t()` calls are bundle-backed
 * vs guard-only. The static suite above trusts that split; these tests make it
 * self-verifying against the source and the bundles, so a fifth `t()` call — or
 * a stray add of a guard-only key into one locale — can't slip through
 * unnoticed.
 */
describe('ProtectedRoute i18n — key classification', () => {
  it('classifies every t() call as bundle-backed or defaultValue-only (no unclassified key)', () => {
    const keys = new Set(translationKeys(PROTECTED_ROUTE_SRC));
    const classified = new Set<string>([...BUNDLE_KEYS, ...DEFAULTVALUE_ONLY_KEYS]);
    const unclassified = [...keys].filter((k) => !classified.has(k));
    expect(
      unclassified,
      `ProtectedRoute t() keys not covered by BUNDLE_KEYS/DEFAULTVALUE_ONLY_KEYS: ${unclassified.join(', ')}`
    ).toEqual([]);
    // ...and every classified key is actually used by the component (lists don't rot).
    const unused = [...classified].filter((k) => !keys.has(k));
    expect(
      unused,
      `classified keys no longer referenced in ProtectedRoute.tsx: ${unused.join(', ')}`
    ).toEqual([]);
  });

  for (const [locale, bundle] of Object.entries(BUNDLES)) {
    for (const key of DEFAULTVALUE_ONLY_KEYS) {
      it(`${locale}.json does not carry the defaultValue-only key ${key}`, () => {
        expect(
          getPath(bundle, key),
          `${key} unexpectedly present in ${locale}.json — add it to ALL six locales (and BUNDLE_KEYS) or drop it, never partially`
        ).toBeUndefined();
      });
    }
  }
});

/**
 * defaultValue ↔ English-bundle parity.
 *
 * For a bundle-backed key the component renders the localized bundle string on
 * the happy path and the `defaultValue` only if the key is ever dropped. If the
 * two diverge, an English user sees different copy depending on whether the key
 * still resolves — a silent inconsistency. Locking the guard's English text to
 * `en.json` keeps the degraded path indistinguishable from the resolved path
 * for English readers.
 */
describe('ProtectedRoute i18n — defaultValue guards are meaningful English', () => {
  for (const key of translationKeys(PROTECTED_ROUTE_SRC)) {
    it(`${key} guard is a non-empty readable string, not a dotted key`, () => {
      const dv = defaultValueOf(PROTECTED_ROUTE_SRC, key);
      expect(dv, `${key} has no extractable defaultValue literal`).toBeTypeOf('string');
      expect((dv as string).trim().length).toBeGreaterThan(0);
      // A defaultValue equal to the key itself would defeat the guard.
      expect(dv).not.toBe(key);
    });
  }

  for (const key of BUNDLE_KEYS) {
    it(`${key} guard matches en.json (degraded English == resolved English)`, () => {
      expect(defaultValueOf(PROTECTED_ROUTE_SRC, key)).toBe(getPath(en, key));
    });
  }
});

/**
 * Runtime resolution + fallback behavior.
 *
 * The static tests prove keys EXIST in JSON; these prove the component's actual
 * `t()` calls RESOLVE the way the UI depends on — through a real i18next
 * instance with `fallbackLng: 'en'`. This is the behavior the `defaultValue`
 * guard exists to protect, and it was previously untested.
 */
describe('ProtectedRoute i18n — runtime resolution & fallback', () => {
  let i18n: I18nInstance;

  beforeAll(async () => {
    i18n = makeI18n();
  });

  for (const locale of Object.keys(BUNDLES)) {
    describe(`locale: ${locale}`, () => {
      for (const key of BUNDLE_KEYS) {
        it(`resolves ${key} to the ${locale} bundle string (not the raw key)`, async () => {
          await i18n.changeLanguage(locale);
          const resolved = i18n.t(key);
          expect(resolved).toBe(getPath(BUNDLES[locale], key));
          expect(resolved, `${key} leaked the raw key in ${locale}`).not.toBe(key);
        });
      }

      for (const key of DEFAULTVALUE_ONLY_KEYS) {
        it(`renders the readable guard for ${key} (never the raw key) in ${locale}`, async () => {
          await i18n.changeLanguage(locale);
          const dv = defaultValueOf(PROTECTED_ROUTE_SRC, key) as string;
          // The component passes the guard, so the user sees readable English…
          expect(i18n.t(key, { defaultValue: dv })).toBe(dv);
          // …whereas WITHOUT the guard the same missing key leaks its dotted path.
          // This is exactly the raw-key-leak the guard-parity test prevents.
          expect(i18n.t(key)).toBe(key);
        });
      }
    });
  }

  it('non-English locales resolve bundle keys to translated (non-English) copy', async () => {
    // Guards against a locale silently mirroring en — the "announces English to
    // non-English users" failure the sibling aria/offline tests were written for.
    await i18n.changeLanguage('sk');
    expect(i18n.t('common.loading')).not.toBe(getPath(en, 'common.loading'));
    expect(i18n.t('errors.unauthorized')).not.toBe(getPath(en, 'errors.unauthorized'));
  });

  it('falls back to English for an unsupported language (fallbackLng)', () => {
    // An unknown lng must not leak the raw key — it must degrade to en copy.
    for (const key of BUNDLE_KEYS) {
      expect(i18n.t(key, { lng: 'fr' })).toBe(getPath(en, key));
    }
  });
});
