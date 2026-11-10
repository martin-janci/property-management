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
 */

import cs from '../../../messages/cs.json';
import de from '../../../messages/de.json';
import en from '../../../messages/en.json';
import hu from '../../../messages/hu.json';
import pl from '../../../messages/pl.json';
import sk from '../../../messages/sk.json';

type Bundle = { disputes?: Record<string, unknown> };

const BUNDLES: Record<string, Bundle> = { en, sk, cs, de, pl, hu };

/** Every disputes.* key the FileDisputePage static copy reads. */
const KEYS = ['filePageTitle', 'subjectPlaceholder', 'descriptionPlaceholder'] as const;

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
});
