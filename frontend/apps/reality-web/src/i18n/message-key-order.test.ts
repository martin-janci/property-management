import { describe, expect, it } from 'vitest';
import cs from '../../messages/cs.json';
import de from '../../messages/de.json';
import en from '../../messages/en.json';
import hu from '../../messages/hu.json';
import pl from '../../messages/pl.json';
import sk from '../../messages/sk.json';
import { locales } from './config';
import { findOrderDivergence, flattenKeys } from './messageKeyOrder';

// Regression guard for issue #3309 (follow-up to PR #3260): the next-intl
// `messages/*.json` files must share a STABLE key order so edits append new
// keys rather than reorder the whole file (which produced ~9k lines of churn
// on cs.json / sk.json and buried the real translation diff).
//
// Canonical order = the reference locale (en.json). Every other locale must
// list the keys it shares with en in en's relative order, recursively. This
// test fails when a locale's keys are reordered; run
// `pnpm --filter @ppt/reality-web messages:normalize` to fix. It also shares
// its rules with scripts/normalize-message-order.mjs (the lint/autofix).

const REFERENCE_LOCALE = 'en';
const messages: Record<string, Record<string, unknown>> = { cs, de, en, hu, pl, sk };
const referenceKeys: string[] = flattenKeys(en);

describe('messages/*.json stable key order', () => {
  it('ships a messages file for every configured locale', () => {
    for (const locale of locales) {
      expect(messages[locale], `missing messages/${locale}.json`).toBeDefined();
    }
  });

  for (const locale of locales) {
    if (locale === REFERENCE_LOCALE) continue;
    it(`${locale}.json keys follow ${REFERENCE_LOCALE}.json order`, () => {
      const divergence = findOrderDivergence(referenceKeys, flattenKeys(messages[locale]));
      expect(
        divergence,
        divergence
          ? `messages/${locale}.json is out of order at shared index ${divergence.index}: ` +
              `expected "${divergence.expected}" before "${divergence.actual}". ` +
              'Run: pnpm --filter @ppt/reality-web messages:normalize'
          : undefined
      ).toBeNull();
    });
  }
});
