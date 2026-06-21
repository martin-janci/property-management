/// <reference types="vitest/globals" />
/**
 * Dispute draft auto-save — i18n key-presence regression (#1360, #1364).
 *
 * The draft auto-save UI (`DraftSavedIndicator.tsx`, `FileDisputePage.tsx`)
 * renders all of its copy via `t(key, defaultValue)` with hard-coded English
 * fallbacks. The keys originally shipped (PR #1359) in *none* of the locale
 * bundles, so every non-English locale silently fell back to English (#1360 /
 * #1364 "Conventions (i18n)"). The fast-follow added them to all six bundles.
 *
 * This test locks that in: every `disputes.draft*` key the components read must
 * exist — and be a non-empty string — in en/sk/cs/de/pl/hu. Without it a future
 * bundle edit (or a new locale) could re-drop a key and regress to the silent
 * English fallback, which is invisible to typecheck and lint.
 *
 * It also asserts that the two interpolated, count-pluralized keys keep their
 * placeholders ({{time}} / {{count}}) in every locale, so a translation can't
 * quietly strip the interpolation and render a literal "{{count}}s ago".
 */

import cs from '../../../messages/cs.json';
import de from '../../../messages/de.json';
import en from '../../../messages/en.json';
import hu from '../../../messages/hu.json';
import pl from '../../../messages/pl.json';
import sk from '../../../messages/sk.json';

type Bundle = { disputes?: Record<string, unknown> };

const BUNDLES: Record<string, Bundle> = { en, sk, cs, de, pl, hu };

/** Every disputes.draft* key the auto-save UI reads (DraftSavedIndicator + FileDisputePage). */
const DRAFT_KEYS = [
  'draftSavedAgo',
  'draftJustNow',
  'draftSecondsAgo',
  'draftMinutesAgo',
  'draftRestored',
] as const;

/** Keys that interpolate a value — must keep their {{placeholder}} per locale. */
const INTERPOLATED: Record<string, string> = {
  draftSavedAgo: '{{time}}',
  draftSecondsAgo: '{{count}}',
  draftMinutesAgo: '{{count}}',
};

describe('dispute draft auto-save i18n keys', () => {
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

      for (const key of DRAFT_KEYS) {
        it(`defines a non-empty disputes.${key}`, () => {
          const value = bundle.disputes?.[key];
          expect(value, `disputes.${key} missing in ${locale}.json`).toBeTypeOf('string');
          expect((value as string).trim().length).toBeGreaterThan(0);
        });
      }

      for (const [key, placeholder] of Object.entries(INTERPOLATED)) {
        it(`keeps the ${placeholder} placeholder in disputes.${key}`, () => {
          const value = bundle.disputes?.[key] as string | undefined;
          expect(value, `disputes.${key} missing in ${locale}.json`).toBeTypeOf('string');
          expect(
            (value as string).includes(placeholder),
            `disputes.${key} in ${locale}.json dropped ${placeholder}`
          ).toBe(true);
        });
      }
    });
  }

  it('keeps every locale in lockstep — no locale has extra or missing draft keys', () => {
    const reference = DRAFT_KEYS.slice().sort();
    for (const [locale, bundle] of Object.entries(BUNDLES)) {
      const present = DRAFT_KEYS.filter((k) => typeof bundle.disputes?.[k] === 'string').sort();
      expect(present, `${locale}.json draft-key set drifted from the reference`).toEqual(reference);
    }
  });
});
