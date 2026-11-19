import { describe, expect, it } from 'vitest';
import cs from '../../messages/cs.json';
import de from '../../messages/de.json';
import en from '../../messages/en.json';
import hu from '../../messages/hu.json';
import pl from '../../messages/pl.json';
import sk from '../../messages/sk.json';
import { locales } from './config';

// Regression guard for issue `code-review-reality-web-comparison-i18n-gap`:
// the comparison tray (`ComparisonTray`) and the listing-card compare toggle
// (`CompareButton`) rendered hardcoded English literals ("Compare",
// "Comparing", "Clear all", "N/4 properties to compare", "Maximum 4
// properties. Remove one first.", the share-tray CTA, the thumbnail alt) that
// never routed through next-intl. They now read `comparison.button.*` and
// `comparison.tray.*`. If any supported locale is missing one of these keys,
// next-intl leaks the raw key (or English) to that locale's users.
// This test fails on `main` (pre-extraction the keys did not exist).

const messages: Record<string, Record<string, unknown>> = { cs, de, en, hu, pl, sk };

// Keys expected directly under comparison.button and comparison.tray — one per
// user-facing string extracted from CompareButton / ComparisonTray.
const BUTTON_KEYS = ['add', 'comparing', 'addTitle', 'removeTitle', 'maxWarning'] as const;
const TRAY_KEYS = ['count', 'clearAll', 'thumbnailAlt', 'compareNeedMore', 'compareReady'] as const;

function comparisonNode(locale: string): Record<string, unknown> {
  return (messages[locale].comparison ?? {}) as Record<string, unknown>;
}

function subNode(locale: string, key: 'button' | 'tray'): Record<string, unknown> {
  return (comparisonNode(locale)[key] ?? {}) as Record<string, unknown>;
}

describe('comparison tray + compare-button i18n coverage', () => {
  it('ships a messages file for every configured locale', () => {
    for (const locale of locales) {
      expect(messages[locale], `missing messages/${locale}.json`).toBeDefined();
    }
  });

  for (const locale of locales) {
    it(`locale ${locale} defines every comparison.button label as a non-empty string`, () => {
      const node = subNode(locale, 'button');
      for (const key of BUTTON_KEYS) {
        expect(typeof node[key], `comparison.button.${key} missing in ${locale}`).toBe('string');
        expect(
          (node[key] as string).length,
          `comparison.button.${key} empty in ${locale}`
        ).toBeGreaterThan(0);
      }
    });

    it(`locale ${locale} defines every comparison.tray label as a non-empty string`, () => {
      const node = subNode(locale, 'tray');
      for (const key of TRAY_KEYS) {
        expect(typeof node[key], `comparison.tray.${key} missing in ${locale}`).toBe('string');
        expect(
          (node[key] as string).length,
          `comparison.tray.${key} empty in ${locale}`
        ).toBeGreaterThan(0);
      }
    });
  }
});
