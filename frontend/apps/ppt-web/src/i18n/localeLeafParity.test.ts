/// <reference types="vitest/globals" />
/**
 * Locale leaf-key parity regression (de/pl/hu catch-up + dead-key guard).
 *
 * `localeNamespaceParity.test.ts` locks the *top-level namespace* structure of
 * the six message bundles but deliberately does NOT assert leaf parity, on the
 * grounds that de/pl/hu are partially translated and fall back to en at runtime.
 * That left a blind spot in the other direction: a non-reference locale can
 * carry a *leaf* key the en reference (the `fallbackLng`) does not have. Such a
 * key is dead — it can never be reached through the fallback chain and no code
 * path references it — but nothing failed CI when one appeared. Two shipped this
 * way inside `dashboard.activity`: `meterReadingSubmitted` (sk + cs) and
 * `faultReported` (cs), neither present in en.json nor referenced in
 * `RecentActivity.tsx` (which uses `faultReportedByYou`).
 *
 * This test asserts leaf-key parity against the en reference for every
 * non-reference locale, with two tolerances that match how the bundles are
 * legitimately allowed to differ:
 *
 *   1. CLDR plural variants. en carries English plural forms (`key`, `key_one`);
 *      sk/cs/pl additionally carry `_few`/`_many`, de/hu carry `_other`. These
 *      are required by each language's plural rules, not drift, so leaf paths are
 *      compared with their trailing `_zero|_one|_two|_few|_many|_other` suffix
 *      stripped (base-key parity).
 *   2. Whole missing namespaces. de/pl/hu do not yet carry four top-level
 *      namespaces (documentTemplates/help/onboarding/subscription — see
 *      `localeNamespaceParity.test.ts` INV3). Missing-leaf checks are therefore
 *      restricted to the namespaces a locale actually carries; the absent
 *      namespaces are the namespace test's job, not this one.
 *
 * Within those tolerances the invariants are:
 *   NO-EXTRA — no locale carries a base-leaf key absent from the en reference
 *              (generalises the namespace suite's INV2 down to the leaf level).
 *   NO-GAP   — within the namespaces a locale carries, every en base-leaf key is
 *              present (the de/pl/hu catch-up guarantee: a partially-translated
 *              locale is partial by whole-namespace, never by a hole punched in a
 *              namespace it does carry).
 *
 * NO-EXTRA fails on the pre-fix tree (the three dead `dashboard.activity` keys)
 * and passes once they are removed. NO-GAP already holds and locks it going
 * forward.
 */

import cs from '../../messages/cs.json';
import de from '../../messages/de.json';
import en from '../../messages/en.json';
import hu from '../../messages/hu.json';
import pl from '../../messages/pl.json';
import sk from '../../messages/sk.json';

type Json = Record<string, unknown>;

const BUNDLES: Record<string, Json> = { en, sk, cs, de, pl, hu };
const NON_REFERENCE = ['sk', 'cs', 'de', 'pl', 'hu'] as const;

/** Trailing i18next/CLDR plural-category suffix. */
const PLURAL_SUFFIX = /_(zero|one|two|few|many|other)$/;

/** Collect every leaf path (dot-joined) in a bundle. */
function leafPaths(obj: Json, prefix = '', out: Set<string> = new Set()): Set<string> {
  for (const [key, value] of Object.entries(obj)) {
    const path = prefix ? `${prefix}.${key}` : key;
    if (value !== null && typeof value === 'object' && !Array.isArray(value)) {
      leafPaths(value as Json, path, out);
    } else {
      out.add(path);
    }
  }
  return out;
}

/** Leaf paths with their trailing plural suffix stripped (base-key set). */
function baseLeaves(bundle: Json): Set<string> {
  return new Set([...leafPaths(bundle)].map((p) => p.replace(PLURAL_SUFFIX, '')));
}

const EN_BASE = baseLeaves(en);

describe('locale leaf-key parity', () => {
  it('covers exactly the six shipped locale bundles', () => {
    expect(Object.keys(BUNDLES).sort()).toEqual(['cs', 'de', 'en', 'hu', 'pl', 'sk']);
  });

  // NO-EXTRA — no base-leaf key outside the en reference.
  for (const locale of NON_REFERENCE) {
    it(`${locale}.json carries no base-leaf key absent from en.json`, () => {
      const extra = [...baseLeaves(BUNDLES[locale])].filter((k) => !EN_BASE.has(k)).sort();
      expect(
        extra,
        `${locale}.json carries leaf key(s) missing from the en fallback (dead keys): ${extra.join(', ')}`
      ).toEqual([]);
    });
  }

  // NO-GAP — within carried namespaces, every en base-leaf key is present.
  for (const locale of NON_REFERENCE) {
    it(`${locale}.json has no leaf gap inside the namespaces it carries`, () => {
      const carried = new Set(Object.keys(BUNDLES[locale]));
      const localeBase = baseLeaves(BUNDLES[locale]);
      const gaps = [...EN_BASE]
        .filter((k) => carried.has(k.split('.')[0]))
        .filter((k) => !localeBase.has(k))
        .sort();
      expect(
        gaps,
        `${locale}.json is missing leaf key(s) inside a namespace it carries ` +
          `(partial locales must be partial by whole namespace, not by a hole in a carried one): ${gaps.join(', ')}`
      ).toEqual([]);
    });
  }
});
