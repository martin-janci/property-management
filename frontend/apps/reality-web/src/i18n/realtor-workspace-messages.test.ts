import { describe, expect, it } from 'vitest';
import cs from '../../messages/cs.json';
import de from '../../messages/de.json';
import en from '../../messages/en.json';
import hu from '../../messages/hu.json';
import pl from '../../messages/pl.json';
import sk from '../../messages/sk.json';
import { locales } from './config';

// Regression guard for the realtor workspace (UC-51) i18n extraction: the
// signed-in realtor pages — hub (/realtor), profile (/realtor/profile),
// analytics (/realtor/analytics) and the create/edit listing pages — render
// all of their chrome via next-intl (`pages.realtor*`). Before extraction
// these pages hardcoded English literals, so sk/cs/de/pl/hu realtors saw
// English copy. If any supported locale is missing one of these keys,
// next-intl leaks the raw key (or English) to that locale's users.
// This test fails on `main` (pre-extraction the keys did not exist).

const messages: Record<string, Record<string, unknown>> = { cs, de, en, hu, pl, sk };

// Flat keys expected directly under each namespace (parity across all locales).
const NAMESPACE_KEYS: Record<string, readonly string[]> = {
  realtorHub: [
    'title',
    'greeting',
    'greetingNamed',
    'listingsSummary',
    'profileTitle',
    'profileDesc',
    'listingsTitle',
    'listingsDesc',
    'createTitle',
    'createDesc',
    'analyticsTitle',
    'analyticsDesc',
  ],
  realtorProfile: [
    'loading',
    'loadError',
    'title',
    'subtitle',
    'nameRequired',
    'saveError',
    'saved',
    'labelName',
    'labelEmail',
    'emailHint',
    'labelPhone',
    'labelTitle',
    'labelBio',
    'labelLicense',
    'labelSpecializations',
    'specializationsHint',
    'save',
    'saving',
  ],
  realtorAnalytics: [
    'title',
    'subtitle',
    'period7d',
    'period30d',
    'period90d',
    'period1y',
    'loading',
    'error',
    'totalListings',
    'activeListings',
    'totalViews',
    'inquiries',
    'conversionRate',
  ],
  realtorListingCreate: ['title', 'subtitle', 'submit', 'error'],
  realtorListingEdit: [
    'missingId',
    'title',
    'subtitle',
    'back',
    'loading',
    'loadError',
    'saved',
    'submit',
    'submitError',
  ],
};

function pageNode(locale: string, namespace: string): Record<string, unknown> {
  const pages = messages[locale].pages as Record<string, unknown> | undefined;
  return (pages?.[namespace] ?? {}) as Record<string, unknown>;
}

describe('realtor workspace i18n coverage', () => {
  it('every supported locale has a message file loaded', () => {
    for (const locale of locales) {
      expect(messages[locale], `missing messages for locale "${locale}"`).toBeDefined();
    }
  });

  for (const locale of locales) {
    describe(`locale: ${locale}`, () => {
      for (const [namespace, keys] of Object.entries(NAMESPACE_KEYS)) {
        it(`defines all pages.${namespace} keys as non-empty strings`, () => {
          const node = pageNode(locale, namespace);
          for (const key of keys) {
            expect(typeof node[key], `pages.${namespace}.${key} missing in ${locale}`).toBe(
              'string'
            );
            expect(
              (node[key] as string).length,
              `pages.${namespace}.${key} empty in ${locale}`
            ).toBeGreaterThan(0);
          }
        });
      }
    });
  }

  it('keeps interpolation placeholders in every locale', () => {
    for (const locale of locales) {
      const hub = pageNode(locale, 'realtorHub');
      expect(hub.greetingNamed as string, `greetingNamed {name} in ${locale}`).toMatch(/\{name\}/);
      expect(hub.listingsSummary as string, `listingsSummary {count} in ${locale}`).toMatch(
        /\{count\}/
      );
    }
  });

  it('exposes the exact same key set in every locale (parity)', () => {
    for (const [namespace, keys] of Object.entries(NAMESPACE_KEYS)) {
      const reference = [...keys].sort();
      for (const locale of locales) {
        const actual = Object.keys(pageNode(locale, namespace)).sort();
        expect(actual, `pages.${namespace} key set drift in ${locale}`).toEqual(reference);
      }
    }
  });
});
