/// <reference types="vitest/globals" />
/**
 * Locale top-level namespace parity regression.
 *
 * The six ppt-web message bundles (`messages/{en,sk,cs,de,pl,hu}.json`) are
 * edited *identically* by i18n PRs — a single feature's keys land in all six
 * files in one change (e.g. #3218 EvidenceUploader, #3220 community). That
 * identical-edit pattern makes the bundles a chronic merge-train conflict
 * hotspot, and a mis-resolved conflict silently drops whole locale subtrees:
 * #2395 inverted `packages`/`delegation` parity (fixed, locked by
 * `deadNamespaceI18n.test.ts`), and on 2026-11-04 a merge-train resolution
 * dropped the sk/cs/hu bundles wholesale (restored in
 * `fix(ppt-web): restore sk/cs/hu locale bundles dropped during merge-train
 * resolution`). Nothing in the suite asserted cross-locale *structure*, so
 * those drops shipped silently and were only caught by eye.
 *
 * This test locks the three structural invariants that hold post-restore, so
 * the next dropped-bundle merge fails CI instead of shipping:
 *
 *   INV1 — en/sk/cs (the fully-translated primary trio) carry an identical
 *          top-level namespace set. A merge that drops a namespace from sk or
 *          cs breaks this.
 *   INV2 — no locale carries a top-level namespace absent from `en` (the
 *          `fallbackLng`). This is the #2395 inversion guard generalised: a
 *          namespace the fallback lacks can never resolve.
 *   INV3 — de/pl/hu (partially-translated) each lack exactly the known set of
 *          namespaces and nothing more. A merge that drops an additional
 *          namespace from one of them breaks this; translating one of the
 *          known-missing namespaces is a deliberate change that updates the
 *          list here.
 *
 * Leaf-key parity is intentionally NOT asserted — de/pl/hu are legitimately
 * incomplete at the key level and fall back to en at runtime. This guard is
 * about catching structural *drops*, not translation completeness.
 */

import cs from '../../messages/cs.json';
import de from '../../messages/de.json';
import en from '../../messages/en.json';
import hu from '../../messages/hu.json';
import pl from '../../messages/pl.json';
import sk from '../../messages/sk.json';

type Json = Record<string, unknown>;

const BUNDLES: Record<string, Json> = { en, sk, cs, de, pl, hu };

/** Locales kept in full top-level-namespace parity with one another. */
const FULL_TRIO = ['en', 'sk', 'cs'] as const;

/** Partially-translated locales and the top-level namespaces they do not yet carry. */
const PARTIAL_MISSING: Record<string, readonly string[]> = {
  de: ['criticalNotifications', 'documentTemplates', 'help', 'onboarding', 'subscription'],
  pl: ['criticalNotifications', 'documentTemplates', 'help', 'onboarding', 'subscription'],
  hu: ['criticalNotifications', 'documentTemplates', 'help', 'onboarding', 'subscription'],
};

const topLevel = (bundle: Json): Set<string> => new Set(Object.keys(bundle));

describe('locale top-level namespace parity', () => {
  // Guard the locale set itself: a newly added bundle must be listed here (and
  // classified as full-trio or partial) or the sweeps below would skip it.
  it('covers exactly the six shipped locale bundles', () => {
    expect(Object.keys(BUNDLES).sort()).toEqual(['cs', 'de', 'en', 'hu', 'pl', 'sk']);
    expect([...FULL_TRIO, ...Object.keys(PARTIAL_MISSING)].sort()).toEqual(
      Object.keys(BUNDLES).sort()
    );
  });

  // INV1 — the full trio shares one identical top-level namespace set.
  const enNamespaces = [...topLevel(en)].sort();
  for (const locale of FULL_TRIO) {
    it(`${locale}.json top-level namespaces match en.json (full trio)`, () => {
      expect([...topLevel(BUNDLES[locale])].sort()).toEqual(enNamespaces);
    });
  }

  // INV2 — no locale carries a namespace the fallbackLng (en) lacks.
  for (const [locale, bundle] of Object.entries(BUNDLES)) {
    it(`${locale}.json carries no top-level namespace absent from en.json`, () => {
      const extra = [...topLevel(bundle)].filter((ns) => !Object.hasOwn(en, ns)).sort();
      expect(
        extra,
        `${locale}.json carries namespace(s) missing from the en fallback: ${extra.join(', ')}`
      ).toEqual([]);
    });
  }

  // INV3 — partial locales lack exactly their documented namespace set.
  for (const [locale, missing] of Object.entries(PARTIAL_MISSING)) {
    it(`${locale}.json is missing exactly its documented namespaces`, () => {
      const actualMissing = [...topLevel(en)].filter((ns) => !Object.hasOwn(BUNDLES[locale], ns));
      expect(
        actualMissing.sort(),
        `${locale}.json namespace gap drifted from the documented set — a merge-train drop, ` +
          'or a deliberate translation that should update PARTIAL_MISSING'
      ).toEqual([...missing].sort());
    });
  }
});
