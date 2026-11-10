import { describe, expect, it } from 'vitest';
import cs from '../../messages/cs.json';
import de from '../../messages/de.json';
import en from '../../messages/en.json';
import hu from '../../messages/hu.json';
import pl from '../../messages/pl.json';
import sk from '../../messages/sk.json';
import { locales } from './config';

// Regression guard for the public realtor/agent profile page i18n extraction:
// the /realtor/[id] page (from PR #3225) hardcoded Slovak literals for all of
// its chrome (loading/not-found states, stat labels, section headings, contact
// labels, and the contact modal), so cs/de/en/hu/pl visitors saw Slovak copy
// on a 6-language portal (issue #3283). The page now renders every string via
// next-intl (`pages.agentProfile.*`). If any supported locale is missing one of
// these keys, next-intl leaks the raw key (or the fallback locale) to that
// locale's users. This test fails on `main` (pre-extraction those keys — e.g.
// loading, notFoundTitle, the modal keys — did not exist).

const messages: Record<string, Record<string, unknown>> = { cs, de, en, hu, pl, sk };

// Flat keys expected directly under pages.agentProfile.
const CHROME_KEYS = [
  'about',
  'activeListings',
  'contact',
  'contactDetails',
  'defaultName',
  'experience',
  'languages',
  'license',
  'loading',
  'mailtoSubject',
  'memberSince',
  'modalAriaLabel',
  'modalCancel',
  'modalClose',
  'modalEmailHint',
  'modalNoContact',
  'modalPhoneIntro',
  'modalPlaceholder',
  'modalSendEmail',
  'modalTitle',
  'noContact',
  'noReviews',
  'notFoundBody',
  'notFoundTitle',
  'rating',
  'reviewVerified',
  'reviews',
  'sendMessage',
  'specializations',
  'totalListings',
  'verified',
  'views',
] as const;

function agentProfileNode(locale: string): Record<string, unknown> {
  const pages = messages[locale].pages as Record<string, unknown> | undefined;
  return (pages?.agentProfile ?? {}) as Record<string, unknown>;
}

describe('pages.agentProfile i18n coverage', () => {
  it('every supported locale has a message file loaded', () => {
    for (const locale of locales) {
      expect(messages[locale], `missing messages for locale "${locale}"`).toBeDefined();
    }
  });

  for (const locale of locales) {
    describe(`locale: ${locale}`, () => {
      it('defines all agent-profile chrome keys as non-empty strings', () => {
        const node = agentProfileNode(locale);
        for (const key of CHROME_KEYS) {
          expect(typeof node[key], `pages.agentProfile.${key} missing in ${locale}`).toBe('string');
          expect(
            (node[key] as string).length,
            `pages.agentProfile.${key} empty in ${locale}`
          ).toBeGreaterThan(0);
        }
      });
    });
  }

  it('interpolated keys keep their placeholders in every locale', () => {
    for (const locale of locales) {
      const node = agentProfileNode(locale);
      expect(node.experience as string, `experience {count} placeholder in ${locale}`).toMatch(
        /\{count\}/
      );
      expect(node.license as string, `license {number} placeholder in ${locale}`).toMatch(
        /\{number\}/
      );
      expect(node.memberSince as string, `memberSince {date} placeholder in ${locale}`).toMatch(
        /\{date\}/
      );
      expect(node.modalTitle as string, `modalTitle {name} placeholder in ${locale}`).toMatch(
        /\{name\}/
      );
    }
  });
});
