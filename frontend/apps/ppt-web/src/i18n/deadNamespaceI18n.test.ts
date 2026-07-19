/// <reference types="vitest/globals" />
/**
 * Dead i18n namespace parity regression (#2405, follow-up to #2395).
 *
 * PR #2395 removed the retired `delegation` and `packages` namespaces from
 * `en.json`, claiming en was the sole holder and that parity with the other
 * five locales was "restored". That was correct for `delegation` (en-only) but
 * factually wrong for `packages`: the identical 11-key dead block still lived in
 * cs/de/hu/pl/sk, so removing it from `en` (the `fallbackLng`) actually inverted
 * parity — en lacked a namespace the other five still carried. The features are
 * retired (see `src/features/unwired-features.ts`); there are zero `packages.` /
 * `delegation.` references in any `.tsx`, so nothing failed at runtime and the
 * dead keys simply persisted.
 *
 * This test locks the cleanup in: neither retired top-level namespace may exist
 * in ANY of the six locale bundles. It fails on `main` (cs/de/hu/pl/sk still
 * carry `packages`) and passes once the block is deleted from all five.
 */

import cs from '../../messages/cs.json';
import de from '../../messages/de.json';
import en from '../../messages/en.json';
import hu from '../../messages/hu.json';
import pl from '../../messages/pl.json';
import sk from '../../messages/sk.json';

type Json = Record<string, unknown>;

const BUNDLES: Record<string, Json> = { en, sk, cs, de, pl, hu };

/** Retired feature namespaces that must not appear in any locale bundle. */
const DEAD_NAMESPACES = ['packages', 'delegation'] as const;

describe('dead i18n namespaces are gone from every locale', () => {
  // Guard the locale set itself: a newly added bundle must be listed here or the
  // sweep below would silently skip it.
  it('covers exactly the six shipped locale bundles', () => {
    expect(Object.keys(BUNDLES).sort()).toEqual(['cs', 'de', 'en', 'hu', 'pl', 'sk']);
  });

  for (const [locale, bundle] of Object.entries(BUNDLES)) {
    for (const ns of DEAD_NAMESPACES) {
      it(`${locale}.json has no top-level "${ns}" namespace`, () => {
        expect(
          Object.hasOwn(bundle, ns),
          `${locale}.json still carries the retired "${ns}" namespace`
        ).toBe(false);
      });
    }
  }
});
