import { describe, expect, it } from 'vitest';
import cs from '../../messages/cs.json';
import de from '../../messages/de.json';
import en from '../../messages/en.json';
import hu from '../../messages/hu.json';
import pl from '../../messages/pl.json';
import sk from '../../messages/sk.json';
import { locales } from './config';

// Regression guard for the Inquiries page i18n extraction: the public
// /inquiries page renders its status labels and chrome via next-intl
// (`pages.inquiries.*`). If any supported locale is missing one of these
// keys, next-intl leaks the raw key (or English) to that locale's users.
// This test fails on `main` (pre-extraction the keys did not exist).

const messages: Record<string, Record<string, unknown>> = { cs, de, en, hu, pl, sk };

// Flat keys expected directly under pages.inquiries.
const CHROME_KEYS = [
  'title',
  'subtitle',
  'filterAll',
  'viewingScheduled',
  'agentResponse',
  'cancelConfirm',
  'cancelYes',
  'cancelNo',
  'cancelInquiry',
  'previous',
  'next',
  'pageInfo',
] as const;

// Keys under pages.inquiries.status — one per InquiryStatus variant.
const STATUS_KEYS = ['pending', 'responded', 'scheduled', 'completed', 'cancelled'] as const;

function inquiriesNode(locale: string): Record<string, unknown> {
  const pages = messages[locale].pages as Record<string, unknown> | undefined;
  return (pages?.inquiries ?? {}) as Record<string, unknown>;
}

describe('pages.inquiries i18n coverage', () => {
  it('every supported locale has a message file loaded', () => {
    for (const locale of locales) {
      expect(messages[locale], `missing messages for locale "${locale}"`).toBeDefined();
    }
  });

  for (const locale of locales) {
    describe(`locale: ${locale}`, () => {
      it('defines all Inquiries chrome keys as non-empty strings', () => {
        const node = inquiriesNode(locale);
        for (const key of CHROME_KEYS) {
          expect(typeof node[key], `pages.inquiries.${key} missing in ${locale}`).toBe('string');
          expect(
            (node[key] as string).length,
            `pages.inquiries.${key} empty in ${locale}`
          ).toBeGreaterThan(0);
        }
      });

      it('defines a status label for every InquiryStatus variant', () => {
        const status = (inquiriesNode(locale).status ?? {}) as Record<string, unknown>;
        for (const key of STATUS_KEYS) {
          expect(typeof status[key], `pages.inquiries.status.${key} missing in ${locale}`).toBe(
            'string'
          );
          expect(
            (status[key] as string).length,
            `pages.inquiries.status.${key} empty in ${locale}`
          ).toBeGreaterThan(0);
        }
      });
    });
  }

  it('pageInfo and viewingScheduled keep their interpolation placeholders in every locale', () => {
    for (const locale of locales) {
      const node = inquiriesNode(locale);
      expect(node.pageInfo as string, `pageInfo placeholders in ${locale}`).toMatch(/\{page\}/);
      expect(node.pageInfo as string, `pageInfo placeholders in ${locale}`).toMatch(/\{total\}/);
      expect(node.viewingScheduled as string, `viewingScheduled placeholders in ${locale}`).toMatch(
        /\{date\}/
      );
      expect(node.viewingScheduled as string, `viewingScheduled placeholders in ${locale}`).toMatch(
        /\{time\}/
      );
    }
  });
});
