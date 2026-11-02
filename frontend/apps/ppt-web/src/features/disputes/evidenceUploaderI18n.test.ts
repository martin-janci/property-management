/// <reference types="vitest/globals" />
/**
 * EvidenceUploader copy — i18n key-presence regression.
 *
 * `EvidenceUploader.tsx` (the dispute-filing evidence surface, Epic 80 Story
 * 80.2) originally rendered its `validate()` error strings, drop-zone hint,
 * format hint, description placeholder, and per-file description/remove aria
 * labels as hard-coded English while the rest of the component already went
 * through `useTranslation`. The i18n fast-follow routed those strings through
 * `t()` / `<Trans>` under the `disputes.evidence.*` namespace and added the
 * backing keys to all six locale bundles.
 *
 * This locks that in the same way `fileDisputeI18n.test.ts` guards the file
 * form keys: every `disputes.evidence.*` key the component reads must exist —
 * and be a non-empty string — in en/sk/cs/de/pl/hu. Without it a future bundle
 * edit (or a new locale) could re-drop a key and silently regress to the
 * English fallback, which is invisible to typecheck and lint.
 */

import cs from '../../../messages/cs.json';
import de from '../../../messages/de.json';
import en from '../../../messages/en.json';
import hu from '../../../messages/hu.json';
import pl from '../../../messages/pl.json';
import sk from '../../../messages/sk.json';

type EvidenceBundle = { disputes?: { evidence?: Record<string, unknown> } };

const BUNDLES: Record<string, EvidenceBundle> = { en, sk, cs, de, pl, hu };

/** Every disputes.evidence.* key EvidenceUploader reads. */
const KEYS = [
  'unsupportedType',
  'fileTooLarge',
  'dropHint',
  'formatHint',
  'descriptionPlaceholder',
  'descriptionLabel',
  'removeLabel',
] as const;

describe('EvidenceUploader i18n keys', () => {
  // Guard the locale set itself: a new locale bundle must be added here too, or
  // the per-key sweep below would silently skip it.
  it('covers exactly the six shipped locale bundles', () => {
    expect(Object.keys(BUNDLES).sort()).toEqual(['cs', 'de', 'en', 'hu', 'pl', 'sk']);
  });

  for (const [locale, bundle] of Object.entries(BUNDLES)) {
    describe(`locale: ${locale}`, () => {
      it('has a disputes.evidence namespace', () => {
        expect(bundle.disputes?.evidence).toBeTypeOf('object');
      });

      for (const key of KEYS) {
        it(`defines a non-empty disputes.evidence.${key}`, () => {
          const value = bundle.disputes?.evidence?.[key];
          expect(value, `disputes.evidence.${key} missing in ${locale}.json`).toBeTypeOf('string');
          expect((value as string).trim().length).toBeGreaterThan(0);
        });
      }
    });
  }

  it('keeps the interpolation placeholders consistent across locales', () => {
    // The component passes {{type}}, {{maxMb}}, {{name}}, {{maxFiles}} — a locale
    // that drops a placeholder would render a literal gap, so every locale must
    // carry the same placeholder set per key as the en reference.
    const placeholders = (s: string) =>
      (s.match(/{{\s*\w+\s*}}/g) ?? []).map((p) => p.replace(/\s/g, '')).sort();
    for (const key of KEYS) {
      const reference = placeholders(en.disputes.evidence[key] as string);
      for (const [locale, bundle] of Object.entries(BUNDLES)) {
        const value = bundle.disputes?.evidence?.[key] as string;
        expect(
          placeholders(value),
          `${locale}.json disputes.evidence.${key} placeholders drifted`
        ).toEqual(reference);
      }
    }
  });

  it('keeps every locale in lockstep — no locale has extra or missing keys', () => {
    const reference = KEYS.slice().sort();
    for (const [locale, bundle] of Object.entries(BUNDLES)) {
      const present = KEYS.filter((k) => typeof bundle.disputes?.evidence?.[k] === 'string').sort();
      expect(present, `${locale}.json key set drifted from the reference`).toEqual(reference);
    }
  });
});
