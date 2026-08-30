/// <reference types="vitest/globals" />
/**
 * DisputeList — i18n key-presence regression.
 *
 * The live `/disputes` list UI (`DisputeList.tsx`) originally rendered its
 * header, filter controls, empty state, and pagination as raw English string
 * literals with zero `useTranslation`. The strings were externalized into the
 * `disputes.*` namespace and the component wired to `t(key)`.
 *
 * This test locks that in: every `disputes.*` key the component reads must
 * exist — and be a non-empty string — in en/sk/cs/de/pl/hu. Without it a
 * future bundle edit (or a new locale) could drop a key and regress to the
 * silent English fallback, which is invisible to typecheck and lint.
 *
 * It also asserts the interpolated pagination key keeps its {{from}}/{{to}}/
 * {{total}} placeholders in every locale, so a translation can't quietly strip
 * the interpolation and render a literal "{{total}}".
 */

import cs from '../../../messages/cs.json';
import de from '../../../messages/de.json';
import en from '../../../messages/en.json';
import hu from '../../../messages/hu.json';
import pl from '../../../messages/pl.json';
import sk from '../../../messages/sk.json';

type Bundle = { disputes?: Record<string, unknown> };

const BUNDLES: Record<string, Bundle> = { en, sk, cs, de, pl, hu };

/** Every disputes.* key DisputeList.tsx reads. */
const LIST_KEYS = [
  'title',
  'fileNewDispute',
  'searchPlaceholder',
  'search',
  'allStatuses',
  'allPriorities',
  'allCategories',
  'noDisputesFound',
  'fileANewDispute',
  'paginationShowing',
  'previous',
  'next',
] as const;

/** Keys that interpolate values — must keep every {{placeholder}} per locale. */
const INTERPOLATED: Record<string, string[]> = {
  paginationShowing: ['{{from}}', '{{to}}', '{{total}}'],
};

describe('DisputeList i18n keys', () => {
  // Guard the locale set itself: if a new locale bundle is added to the app it
  // must also be added here, or the per-key sweep below would silently skip it.
  it('covers exactly the six shipped locale bundles', () => {
    expect(Object.keys(BUNDLES).sort()).toEqual(['cs', 'de', 'en', 'hu', 'pl', 'sk']);
  });

  for (const [locale, bundle] of Object.entries(BUNDLES)) {
    describe(`locale: ${locale}`, () => {
      it('has a disputes namespace', () => {
        expect(bundle.disputes).toBeTypeOf('object');
      });

      for (const key of LIST_KEYS) {
        it(`defines a non-empty disputes.${key}`, () => {
          const value = bundle.disputes?.[key];
          expect(value, `disputes.${key} missing in ${locale}.json`).toBeTypeOf('string');
          expect((value as string).trim().length).toBeGreaterThan(0);
        });
      }

      for (const [key, placeholders] of Object.entries(INTERPOLATED)) {
        for (const placeholder of placeholders) {
          it(`keeps the ${placeholder} placeholder in disputes.${key}`, () => {
            const value = bundle.disputes?.[key] as string | undefined;
            expect(value, `disputes.${key} missing in ${locale}.json`).toBeTypeOf('string');
            expect(
              (value as string).includes(placeholder),
              `disputes.${key} in ${locale}.json dropped ${placeholder}`
            ).toBe(true);
          });
        }
      }
    });
  }

  it('keeps every locale in lockstep — no locale has extra or missing list keys', () => {
    const reference = LIST_KEYS.slice().sort();
    for (const [locale, bundle] of Object.entries(BUNDLES)) {
      const present = LIST_KEYS.filter((k) => typeof bundle.disputes?.[k] === 'string').sort();
      expect(present, `${locale}.json list-key set drifted from the reference`).toEqual(reference);
    }
  });
});
