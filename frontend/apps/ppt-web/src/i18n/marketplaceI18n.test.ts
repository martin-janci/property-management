/// <reference types="vitest/globals" />
/**
 * `marketplace.*` namespace i18n key-presence regression.
 *
 * The Service Provider Marketplace feature (`src/features/marketplace/**`)
 * shipped English-only: only `VerificationBadge.tsx` (and, partially, a single
 * `aria.urgent` lookup in `RfqCard.tsx`) called `useTranslation`. Every other
 * card, table, form and page — and the 16 shared service-category labels, the
 * RFQ/quote status labels, and all the input placeholders — rendered hardcoded
 * English literals in JSX, invisible to typecheck + lint because the literals
 * were valid JSX. Non-English users saw English text across the whole feature
 * in a four-plus-locale app.
 *
 * The strings now live under the `marketplace.*` namespace and are consumed via
 * `t('marketplace.*')` in the components. This test locks it in: every leaf key
 * present under `marketplace` in `en.json` must exist — and be a non-empty
 * string — in all six shipped locale bundles. It fails on `main` (the new
 * sub-namespaces do not exist in any bundle) and passes once the blocks are
 * added to all six.
 *
 * Sibling of `verificationBadgeI18n.test.ts` (the pre-existing
 * `marketplace.verificationBadge.*` guard) and `communityI18n.test.ts`.
 */

import cs from '../../messages/cs.json';
import de from '../../messages/de.json';
import en from '../../messages/en.json';
import hu from '../../messages/hu.json';
import pl from '../../messages/pl.json';
import sk from '../../messages/sk.json';

type Json = Record<string, unknown>;

const BUNDLES: Record<string, Json> = { en, sk, cs, de, pl, hu };

/** Dotted leaf-key paths reachable from a nested object. */
function leafPaths(obj: unknown, prefix = ''): string[] {
  if (obj === null || typeof obj !== 'object') return [prefix];
  return Object.entries(obj as Json).flatMap(([k, v]) =>
    leafPaths(v, prefix ? `${prefix}.${k}` : k)
  );
}

function getPath(obj: unknown, path: string): unknown {
  return path.split('.').reduce<unknown>((acc, part) => {
    if (acc === null || typeof acc !== 'object') return undefined;
    return (acc as Json)[part];
  }, obj);
}

/** The full set of marketplace.* leaf-key paths, from en.json. */
const MARKETPLACE_PATHS: string[] = leafPaths((en as Json).marketplace, 'marketplace');

describe('marketplace.* i18n keys', () => {
  it('covers exactly the six shipped locale bundles', () => {
    expect(Object.keys(BUNDLES).sort()).toEqual(['cs', 'de', 'en', 'hu', 'pl', 'sk']);
  });

  it('reference key set spans every marketplace surface', () => {
    expect(MARKETPLACE_PATHS).toEqual(
      expect.arrayContaining([
        // 16 shared service-category labels
        'marketplace.serviceCategory.plumbing',
        'marketplace.serviceCategory.general_maintenance',
        'marketplace.serviceCategory.waste_management',
        'marketplace.serviceCategory.other',
        // shared commons
        'marketplace.common.verified',
        'marketplace.common.viewDetails',
        // status label maps
        'marketplace.rfqStatus.quotes_received',
        'marketplace.quoteStatus.submitted',
        // cards
        'marketplace.providerCard.viewProfile',
        'marketplace.quoteCard.acceptQuote',
        'marketplace.rfqCard.compareQuotes',
        'marketplace.comparisonTable.bestValue',
        'marketplace.ratingBreakdown.title',
        'marketplace.reviewCard.respondToReview',
        // forms (incl. placeholders)
        'marketplace.searchFilters.searchPlaceholder',
        'marketplace.reviewForm.submitReview',
        'marketplace.rfqForm.titlePlaceholder',
        'marketplace.providerProfileForm.createProfile',
        'marketplace.verificationForm.submitForVerification',
        // pages
        'marketplace.marketplacePage.title',
        'marketplace.createRfqPage.step1',
        'marketplace.providerDetailPage.backToMarketplace',
        'marketplace.providerProfilePage.dashboard',
        'marketplace.quoteComparisonPage.compareQuotes',
        'marketplace.rfqListPage.title',
      ])
    );
  });

  for (const [locale, bundle] of Object.entries(BUNDLES)) {
    describe(`locale: ${locale}`, () => {
      for (const path of MARKETPLACE_PATHS) {
        it(`defines a non-empty ${path}`, () => {
          const value = getPath(bundle, path);
          expect(value, `${path} missing in ${locale}.json`).toBeTypeOf('string');
          expect((value as string).trim().length).toBeGreaterThan(0);
        });
      }
    });
  }

  it('keeps every locale in lockstep — no locale has missing marketplace keys', () => {
    for (const [locale, bundle] of Object.entries(BUNDLES)) {
      const present = MARKETPLACE_PATHS.filter((p) => typeof getPath(bundle, p) === 'string');
      expect(present, `${locale}.json marketplace-key set drifted from the reference`).toEqual(
        MARKETPLACE_PATHS
      );
    }
  });
});
