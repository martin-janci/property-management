import { describe, expect, it } from 'vitest';
import cs from '../../messages/cs.json';
import de from '../../messages/de.json';
import en from '../../messages/en.json';
import hu from '../../messages/hu.json';
import pl from '../../messages/pl.json';
import sk from '../../messages/sk.json';
import { locales } from './config';

// Regression guard for the auth button (Epic 10A-SSO) i18n extraction: the
// AuthButton component (src/components/auth-button.tsx) hardcoded English
// literals ("Sign In", "My Favorites", "Profile", "Sign Out") for its login
// button and user dropdown, so cs/de/pl/sk/hu visitors saw English copy on a
// 6-language portal. The component now renders every string via next-intl
// (`auth.menu.*`). If any supported locale is missing one of these keys,
// next-intl leaks the raw key (or the fallback locale) to that locale's users.
// This test fails on `main` (pre-extraction the auth.menu keys did not exist).

const messages: Record<string, Record<string, unknown>> = { cs, de, en, hu, pl, sk };

// Flat keys expected directly under auth.menu.
const MENU_KEYS = ['signIn', 'favorites', 'profile', 'signOut'] as const;

function authMenuNode(locale: string): Record<string, unknown> {
  const auth = messages[locale].auth as Record<string, unknown> | undefined;
  return (auth?.menu ?? {}) as Record<string, unknown>;
}

describe('auth.menu i18n coverage', () => {
  it('every supported locale has a message file loaded', () => {
    for (const locale of locales) {
      expect(messages[locale], `missing messages for locale "${locale}"`).toBeDefined();
    }
  });

  for (const locale of locales) {
    describe(`locale: ${locale}`, () => {
      it('defines all auth-menu keys as non-empty strings', () => {
        const node = authMenuNode(locale);
        for (const key of MENU_KEYS) {
          expect(typeof node[key], `auth.menu.${key} missing in ${locale}`).toBe('string');
          expect(
            (node[key] as string).length,
            `auth.menu.${key} empty in ${locale}`
          ).toBeGreaterThan(0);
        }
      });
    });
  }

  it('exposes the exact same key set in every locale (parity)', () => {
    const reference = Object.keys(authMenuNode('en')).sort();
    for (const locale of locales) {
      expect(Object.keys(authMenuNode(locale)).sort(), `auth.menu key parity in ${locale}`).toEqual(
        reference
      );
    }
  });
});
