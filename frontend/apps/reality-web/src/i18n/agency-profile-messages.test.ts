import { describe, expect, it } from 'vitest';
import cs from '../../messages/cs.json';
import de from '../../messages/de.json';
import en from '../../messages/en.json';
import hu from '../../messages/hu.json';
import pl from '../../messages/pl.json';
import sk from '../../messages/sk.json';
import { locales } from './config';

// Regression guard for the public agency profile page i18n extraction: the
// /agency/[slug] page renders all of its chrome (loading/empty states,
// section headings, contact labels) via next-intl (`pages.agencyProfile.*`).
// Before extraction the page hardcoded Slovak literals, so cs/de/en/hu/pl
// visitors saw Slovak copy. If any supported locale is missing one of these
// keys, next-intl leaks the raw key (or the fallback locale) to that locale's
// users. This test fails on `main` (pre-extraction the keys did not exist).

const messages: Record<string, Record<string, unknown>> = { cs, de, en, hu, pl, sk };

// Flat keys expected directly under pages.agencyProfile.
const CHROME_KEYS = [
  'loading',
  'notFound',
  'verified',
  'about',
  'team',
  'activeListings',
  'noListings',
  'rentSuffix',
  'contact',
  'visitWebsite',
  'license',
] as const;

function agencyProfileNode(locale: string): Record<string, unknown> {
  const pages = messages[locale].pages as Record<string, unknown> | undefined;
  return (pages?.agencyProfile ?? {}) as Record<string, unknown>;
}

describe('pages.agencyProfile i18n coverage', () => {
  it('every supported locale has a message file loaded', () => {
    for (const locale of locales) {
      expect(messages[locale], `missing messages for locale "${locale}"`).toBeDefined();
    }
  });

  for (const locale of locales) {
    describe(`locale: ${locale}`, () => {
      it('defines all agency-profile chrome keys as non-empty strings', () => {
        const node = agencyProfileNode(locale);
        for (const key of CHROME_KEYS) {
          expect(typeof node[key], `pages.agencyProfile.${key} missing in ${locale}`).toBe(
            'string'
          );
          expect(
            (node[key] as string).length,
            `pages.agencyProfile.${key} empty in ${locale}`
          ).toBeGreaterThan(0);
        }
      });
    });
  }

  it('team / activeListings / license keep their interpolation placeholders in every locale', () => {
    for (const locale of locales) {
      const node = agencyProfileNode(locale);
      expect(node.team as string, `team {count} placeholder in ${locale}`).toMatch(/\{count\}/);
      expect(
        node.activeListings as string,
        `activeListings {count} placeholder in ${locale}`
      ).toMatch(/\{count\}/);
      expect(node.license as string, `license {number} placeholder in ${locale}`).toMatch(
        /\{number\}/
      );
    }
  });
});
