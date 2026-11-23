/// <reference types="vitest/globals" />
/**
 * FileDisputePage static copy — i18n key-presence regression.
 *
 * `FileDisputePage.tsx` (the live /disputes/new form) originally left its h1
 * title and the subject/description input placeholders as hard-coded English
 * while the rest of the page already went through `t(key, defaultValue)`. The
 * i18n fast-follow routed those last strings through `useTranslation` and added
 * the backing keys to all six locale bundles.
 *
 * This locks that in the same way `draftAutosaveI18n.test.ts` guards the draft
 * keys: every `disputes.*` key the page reads must exist — and be a non-empty
 * string — in en/sk/cs/de/pl/hu. Without it a future bundle edit (or a new
 * locale) could re-drop a key and silently regress to the English fallback,
 * which is invisible to typecheck and lint.
 *
 * It also guards the page's *cross-namespace* user-facing strings
 * (`common.backToDisputes`, `common.cancel`, `aria.disputeType`). Those are
 * rendered by the File Dispute form but live outside the `disputes.*`
 * namespace, so `disputesI18nCompleteness.test.ts` — which only scans
 * `disputes.*` — never sees them. A locale dropping one of these would let the
 * page fall back to English with nothing in the disputes feature catching it.
 */

import cs from '../../../messages/cs.json';
import de from '../../../messages/de.json';
import en from '../../../messages/en.json';
import hu from '../../../messages/hu.json';
import pl from '../../../messages/pl.json';
import sk from '../../../messages/sk.json';

type Bundle = {
  disputes?: Record<string, unknown>;
  common?: Record<string, unknown>;
  aria?: Record<string, unknown>;
};

const BUNDLES: Record<string, Bundle> = { en, sk, cs, de, pl, hu };

/** Every disputes.* key the FileDisputePage static copy reads. */
const KEYS = ['filePageTitle', 'subjectPlaceholder', 'descriptionPlaceholder'] as const;

/**
 * User-facing keys the page renders from *outside* the disputes namespace.
 * These are invisible to `disputesI18nCompleteness.test.ts`, so they get their
 * own per-locale presence guard here. `[namespace, key]` pairs.
 */
const CROSS_NAMESPACE_KEYS = [
  ['common', 'backToDisputes'],
  ['common', 'cancel'],
  ['aria', 'disputeType'],
] as const;

describe('FileDisputePage i18n keys', () => {
  // Guard the locale set itself: a new locale bundle must be added here too, or
  // the per-key sweep below would silently skip it.
  it('covers exactly the six shipped locale bundles', () => {
    expect(Object.keys(BUNDLES).sort()).toEqual(['cs', 'de', 'en', 'hu', 'pl', 'sk']);
  });

  for (const [locale, bundle] of Object.entries(BUNDLES)) {
    describe(`locale: ${locale}`, () => {
      it('has a disputes namespace', () => {
        expect(bundle.disputes).toBeTypeOf('object');
      });

      for (const key of KEYS) {
        it(`defines a non-empty disputes.${key}`, () => {
          const value = bundle.disputes?.[key];
          expect(value, `disputes.${key} missing in ${locale}.json`).toBeTypeOf('string');
          expect((value as string).trim().length).toBeGreaterThan(0);
        });
      }
    });
  }

  it('keeps every locale in lockstep — no locale has extra or missing keys', () => {
    const reference = KEYS.slice().sort();
    for (const [locale, bundle] of Object.entries(BUNDLES)) {
      const present = KEYS.filter((k) => typeof bundle.disputes?.[k] === 'string').sort();
      expect(present, `${locale}.json key set drifted from the reference`).toEqual(reference);
    }
  });

  describe('cross-namespace user-facing keys', () => {
    for (const [locale, bundle] of Object.entries(BUNDLES)) {
      for (const [namespace, key] of CROSS_NAMESPACE_KEYS) {
        it(`defines a non-empty ${namespace}.${key} in ${locale}`, () => {
          const value = bundle[namespace]?.[key];
          expect(value, `${namespace}.${key} missing in ${locale}.json`).toBeTypeOf('string');
          expect((value as string).trim().length).toBeGreaterThan(0);
        });
      }
    }
  });
});
