/**
 * Regression test for the untranslated inquiry-type bug.
 *
 * The inquiries page used to print `inquiry.type.replace('_', ' ')`, so a
 * Slovak/Czech/German user saw the raw English enum ("viewing request") on
 * their inquiry cards. These tests pin that the type now resolves through the
 * next-intl translation function (a stable `type.<value>` key) rather than the
 * raw enum, and that every message catalog carries those keys.
 */
/// <reference types="vitest/globals" />

import { readFileSync } from 'node:fs';
import path from 'node:path';
import type { InquiryType } from '@ppt/reality-api-client';
import { describe, expect, it, vi } from 'vitest';
import { humanizeInquiryType, inquiryTypeLabel, KNOWN_INQUIRY_TYPES } from './inquiry-type';

describe('inquiryTypeLabel', () => {
  it.each(KNOWN_INQUIRY_TYPES)(
    'resolves the %s enum via the translation function, not the raw enum',
    (type) => {
      // A spy `t` that marks whatever key it was asked for, so we can prove the
      // label came from the translation layer rather than from the raw enum.
      const t = vi.fn((key: string) => `translated:${key}`);

      const label = inquiryTypeLabel(t, type);

      // Routed through next-intl with the stable per-enum key…
      expect(t).toHaveBeenCalledWith(`type.${type}`);
      expect(label).toBe(`translated:type.${type}`);
      // …and never the raw enum or its naive humanization.
      expect(label).not.toBe(type);
      expect(label).not.toBe(type.replace('_', ' '));
    }
  );

  it('falls back to a humanized default for unknown values without calling t', () => {
    const t = vi.fn((key: string) => `translated:${key}`);

    const label = inquiryTypeLabel(t, 'some_future_type');

    expect(t).not.toHaveBeenCalled();
    expect(label).toBe('Some future type');
  });

  it('humanizes multi-underscore and empty values safely', () => {
    expect(humanizeInquiryType('a_b_c')).toBe('A b c');
    expect(humanizeInquiryType('')).toBe('');
  });
});

describe('inquiry-type message catalogs', () => {
  const LOCALES = ['en', 'sk', 'cs', 'de', 'hu', 'pl'] as const;
  const messagesDir = path.resolve(__dirname, '../../messages');

  it.each(LOCALES)('%s defines pages.inquiries.type for every known enum value', (locale) => {
    const catalog = JSON.parse(readFileSync(path.join(messagesDir, `${locale}.json`), 'utf8')) as {
      pages?: { inquiries?: { type?: Record<string, string> } };
    };
    const typeBlock = catalog.pages?.inquiries?.type;

    expect(typeBlock, `${locale}.json is missing pages.inquiries.type`).toBeDefined();
    for (const value of KNOWN_INQUIRY_TYPES as readonly InquiryType[]) {
      const label = typeBlock?.[value];
      expect(label, `${locale}.json missing pages.inquiries.type.${value}`).toBeTruthy();
    }
  });
});
