import { describe, expect, it } from 'vitest';
import cs from '../../messages/cs.json';
import de from '../../messages/de.json';
import en from '../../messages/en.json';
import hu from '../../messages/hu.json';
import pl from '../../messages/pl.json';
import sk from '../../messages/sk.json';
import { locales } from './config';

// Regression guard for the realtor CRUD i18n extraction (code-review finding:
// reality-web realtor create/update/delete surfaces hardcoded English error
// strings). `realtor-api.ts` threw English messages and the profile / create /
// edit pages rendered English fallbacks directly, so cs/de/hu/pl/sk visitors
// saw English error copy on a 6-language portal. The error surfaces now render
// via next-intl (`realtor.*`), mapped from `RealtorApiError.code` by
// `useRealtorErrorMessage`. If any supported locale is missing one of these
// keys, next-intl leaks the raw key (or the fallback locale) to that locale's
// users. This test fails on `dev` (pre-extraction the `realtor` namespace did
// not exist).

const messages: Record<string, Record<string, unknown>> = { cs, de, en, hu, pl, sk };

// Flat keys expected directly under the `realtor` namespace.
const ERROR_KEYS = [
  'errorNetwork',
  'errorMalformed',
  'errorRequestFailed',
  'errorLoadProfile',
  'errorSaveProfile',
  'errorNameRequired',
  'errorLoadListing',
  'errorSaveListing',
  'errorCreateListing',
  'errorMissingListingId',
  'errorLoadAnalytics',
] as const;

function realtorNode(locale: string): Record<string, unknown> {
  return (messages[locale].realtor ?? {}) as Record<string, unknown>;
}

describe('realtor.* error i18n coverage', () => {
  it('every supported locale has a message file loaded', () => {
    for (const locale of locales) {
      expect(messages[locale], `missing messages for locale "${locale}"`).toBeDefined();
    }
  });

  for (const locale of locales) {
    describe(`locale: ${locale}`, () => {
      it('defines all realtor error keys as non-empty strings', () => {
        const node = realtorNode(locale);
        for (const key of ERROR_KEYS) {
          expect(typeof node[key], `realtor.${key} missing in ${locale}`).toBe('string');
          expect((node[key] as string).length, `realtor.${key} empty in ${locale}`).toBeGreaterThan(
            0
          );
        }
      });
    });
  }

  it('keeps the {status} placeholder on errorRequestFailed in every locale', () => {
    for (const locale of locales) {
      const node = realtorNode(locale);
      expect(
        node.errorRequestFailed as string,
        `errorRequestFailed {status} placeholder in ${locale}`
      ).toMatch(/\{status\}/);
    }
  });
});
