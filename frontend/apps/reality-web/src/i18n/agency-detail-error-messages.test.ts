import { describe, expect, it } from 'vitest';
import cs from '../../messages/cs.json';
import de from '../../messages/de.json';
import en from '../../messages/en.json';
import hu from '../../messages/hu.json';
import pl from '../../messages/pl.json';
import sk from '../../messages/sk.json';
import { locales } from './config';

// Regression guard for the realtor detail-modal error-handling fix (code-review
// finding: reality-web RealtorDetailModal performed save / remove / status
// mutations with no try/catch, so a failed mutation surfaced nothing — a silent
// failure on a 6-language portal). The modal now renders an inline next-intl
// error (`agency.detail*Error`). If any supported locale is missing one of these
// keys, next-intl leaks the raw key (or the fallback locale) to that locale's
// users. This test fails on `dev` (pre-fix the keys did not exist).

const messages: Record<string, Record<string, unknown>> = { cs, de, en, hu, pl, sk };

// Flat keys expected directly under the `agency` namespace.
const ERROR_KEYS = ['detailUpdateError', 'detailRemoveError', 'detailStatusError'] as const;

function agencyNode(locale: string): Record<string, unknown> {
  return (messages[locale].agency ?? {}) as Record<string, unknown>;
}

describe('agency.detail*Error i18n coverage', () => {
  it('every supported locale has a message file loaded', () => {
    for (const locale of locales) {
      expect(messages[locale], `missing messages for locale "${locale}"`).toBeDefined();
    }
  });

  for (const locale of locales) {
    describe(`locale: ${locale}`, () => {
      it('defines all detail-modal error keys as non-empty strings', () => {
        const node = agencyNode(locale);
        for (const key of ERROR_KEYS) {
          expect(typeof node[key], `agency.${key} missing in ${locale}`).toBe('string');
          expect((node[key] as string).length, `agency.${key} empty in ${locale}`).toBeGreaterThan(
            0
          );
        }
      });
    });
  }
});
