/// <reference types="vitest/globals" />
/**
 * `DocumentSearchResult` card i18n key-presence regression.
 *
 * A code-review finding flagged the ppt-web document-search-result card
 * (`features/documents/components/DocumentSearchResult.tsx`) for "hardcoded
 * English strings". The component itself already routes every user-facing
 * string through `react-i18next` (`t(...)`) — the one remaining literal, the
 * byte-unit suffix (`B`/`KB`/`MB`), is deliberately locale-independent and
 * shared with `@ppt/shared`'s `formatFileSize`, so it is intentionally not
 * translated. What was missing was a guard locking the card's keys into every
 * shipped locale bundle: the sibling `DocumentSearch.i18n.test.tsx` only covers
 * the search *error* path, not the result card's field labels / relevance /
 * audience copy. Because a dropped locale subtree is invisible to typecheck +
 * lint (the keys resolve to the English fallback at runtime), such a drop would
 * ship silently (cf. the merge-train bundle drops locked by
 * `localeNamespaceParity.test.ts`).
 *
 * This test enumerates exactly the keys `DocumentSearchResult.tsx` (and its
 * `AudienceBadge` child) consume and asserts each resolves to a non-empty
 * string in all six shipped bundles. It fails if any of those keys is removed
 * from, or emptied in, any locale.
 */

import cs from '../../messages/cs.json';
import de from '../../messages/de.json';
import en from '../../messages/en.json';
import hu from '../../messages/hu.json';
import pl from '../../messages/pl.json';
import sk from '../../messages/sk.json';

type Json = Record<string, unknown>;

const BUNDLES: Record<string, Json> = { en, sk, cs, de, pl, hu };

function getPath(obj: unknown, path: string): unknown {
  return path.split('.').reduce<unknown>((acc, part) => {
    if (acc === null || typeof acc !== 'object') return undefined;
    return (acc as Json)[part];
  }, obj);
}

/**
 * Every i18n key `DocumentSearchResult.tsx` consumes. Kept as an explicit list
 * (rather than a namespace sweep) because `documents.*` is a large shared
 * namespace — this test guards only the keys this card renders.
 */
const SEARCH_RESULT_KEYS: readonly string[] = [
  // Highlighted-snippet field labels (getFieldLabel)
  'documents.searchResult.field.title',
  'documents.searchResult.field.description',
  'documents.searchResult.field.ocrText',
  'documents.searchResult.field.summary',
  // Relevance score tooltip + match pill
  'documents.searchResult.relevance',
  'documents.searchResult.match',
  // "Has summary" badge
  'documents.searchResult.hasSummary',
  // AudienceBadge pill labels (SCOPE_LABEL_KEYS) + its tooltip
  'documents.search.audience.organization',
  'documents.search.audience.building',
  'documents.search.audience.unit',
  'documents.search.audience.user',
  'documents.search.audience.public',
  'documents.detail.visibleTo',
];

describe('DocumentSearchResult card i18n keys', () => {
  it('covers exactly the six shipped locale bundles', () => {
    expect(Object.keys(BUNDLES).sort()).toEqual(['cs', 'de', 'en', 'hu', 'pl', 'sk']);
  });

  for (const [locale, bundle] of Object.entries(BUNDLES)) {
    describe(`locale: ${locale}`, () => {
      for (const path of SEARCH_RESULT_KEYS) {
        it(`defines a non-empty ${path}`, () => {
          const value = getPath(bundle, path);
          expect(value, `${path} missing in ${locale}.json`).toBeTypeOf('string');
          expect((value as string).trim().length).toBeGreaterThan(0);
        });
      }
    });
  }

  it('keeps every locale in lockstep — no locale drops a DocumentSearchResult key', () => {
    for (const [locale, bundle] of Object.entries(BUNDLES)) {
      const present = SEARCH_RESULT_KEYS.filter((p) => typeof getPath(bundle, p) === 'string');
      expect(
        present,
        `${locale}.json DocumentSearchResult key set drifted from the reference`
      ).toEqual(SEARCH_RESULT_KEYS);
    }
  });
});
