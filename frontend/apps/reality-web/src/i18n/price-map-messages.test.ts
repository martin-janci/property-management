import { describe, expect, it } from 'vitest';
import cs from '../../messages/cs.json';
import de from '../../messages/de.json';
import en from '../../messages/en.json';
import hu from '../../messages/hu.json';
import pl from '../../messages/pl.json';
import sk from '../../messages/sk.json';
import { locales } from './config';

// Regression guard for issue #3502 (follow-up to PR #3489): the price-map
// transaction-type toggle was migrated to the @ppt/ui-kit SegmentedControl
// but its option labels were hardcoded Slovak literals ("Predaj"/"Prenájom").
// They now route through next-intl (`pages.priceMap.sale` / `.rent`). If any
// supported locale is missing one of these keys, next-intl leaks the raw key
// (or English) to that locale's users on the price-map toggle.
// This test fails on `main` (pre-extraction the keys did not exist).

const messages: Record<string, Record<string, unknown>> = { cs, de, en, hu, pl, sk };

// Toggle option labels expected directly under pages.priceMap — one per
// TransactionType variant rendered in the SegmentedControl.
const TOGGLE_KEYS = ['sale', 'rent'] as const;

function priceMapNode(locale: string): Record<string, unknown> {
  const pages = messages[locale].pages as Record<string, unknown> | undefined;
  return (pages?.priceMap ?? {}) as Record<string, unknown>;
}

describe('pages.priceMap toggle i18n coverage', () => {
  it('ships a messages file for every configured locale', () => {
    for (const locale of locales) {
      expect(messages[locale], `missing messages/${locale}.json`).toBeDefined();
    }
  });

  for (const locale of locales) {
    it(`locale ${locale} defines every SegmentedControl toggle label as a non-empty string`, () => {
      const node = priceMapNode(locale);
      for (const key of TOGGLE_KEYS) {
        expect(typeof node[key], `pages.priceMap.${key} missing in ${locale}`).toBe('string');
        expect(
          (node[key] as string).length,
          `pages.priceMap.${key} empty in ${locale}`
        ).toBeGreaterThan(0);
      }
    });
  }
});
