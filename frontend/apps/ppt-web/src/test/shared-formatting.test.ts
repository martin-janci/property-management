/// <reference types="vitest/globals" />
/**
 * Tests for @ppt/shared formatting utilities.
 *
 * Locale-aware Intl.* outputs vary by ICU version, so most assertions either
 * pin a specific locale or use loose matchers.
 */

import {
  formatAddress,
  formatCompactCurrency,
  formatCompactNumber,
  formatCurrency,
  formatDate,
  formatDateTime,
  formatDetailedDate,
  formatFileSize,
  formatISODate,
  formatNumber,
  formatPercent,
  formatPhone,
  formatRelativeTime,
  formatTime,
  getUserLocale,
} from '@ppt/shared';

describe('@ppt/shared - formatting', () => {
  describe('getUserLocale', () => {
    it('returns a non-empty string', () => {
      const locale = getUserLocale();
      expect(typeof locale).toBe('string');
      expect(locale.length).toBeGreaterThan(0);
    });
  });

  describe('formatCurrency', () => {
    it('formats EUR with the en-US locale', () => {
      expect(formatCurrency(1234.56, { locale: 'en-US', currency: 'EUR' })).toBe('€1,234.56');
    });

    it('formats USD', () => {
      expect(formatCurrency(1234.56, { locale: 'en-US', currency: 'USD' })).toBe('$1,234.56');
    });

    it('honors fromCents flag', () => {
      expect(formatCurrency(123456, { locale: 'en-US', currency: 'EUR', fromCents: true })).toBe(
        '€1,234.56',
      );
    });
  });

  describe('formatCompactCurrency', () => {
    it('produces a compact representation', () => {
      const result = formatCompactCurrency(1_500_000, { locale: 'en-US', currency: 'USD' });
      // Could be "$1.5M" depending on ICU
      expect(result).toMatch(/\$1\.5M|US\$1\.5M/);
    });
  });

  describe('date formatting', () => {
    const date = new Date('2026-03-20T15:45:00Z');

    it('formatDate returns a short date', () => {
      const r = formatDate(date, { locale: 'en-US' });
      expect(r).toMatch(/Mar/);
      expect(r).toMatch(/2026/);
    });

    it('formatDetailedDate uses full month name', () => {
      const r = formatDetailedDate(date, { locale: 'en-US' });
      expect(r).toMatch(/March/);
      expect(r).toMatch(/2026/);
    });

    it('formatDateTime includes hour and minute', () => {
      const r = formatDateTime(date, { locale: 'en-US' });
      expect(r).toMatch(/2026/);
      expect(r).toMatch(/:/);
    });

    it('formatTime returns hour:minute', () => {
      const r = formatTime(date, { locale: 'en-US' });
      expect(r).toMatch(/:/);
    });

    it('accepts Date, string, and number inputs', () => {
      expect(formatDate(date, { locale: 'en-US' })).toMatch(/Mar/);
      expect(formatDate('2026-03-20T15:45:00Z', { locale: 'en-US' })).toMatch(/Mar/);
      expect(formatDate(date.getTime(), { locale: 'en-US' })).toMatch(/Mar/);
    });
  });

  describe('formatISODate', () => {
    it('returns YYYY-MM-DD slice of toISOString', () => {
      expect(formatISODate(new Date('2026-03-20T15:45:00Z'))).toBe('2026-03-20');
    });
  });

  describe('formatRelativeTime', () => {
    it('returns "ago" for past dates', () => {
      const past = new Date(Date.now() - 60 * 60 * 1000);
      const r = formatRelativeTime(past, { locale: 'en-US' });
      expect(r).toMatch(/ago/);
    });

    it('returns future phrasing for upcoming dates', () => {
      const future = new Date(Date.now() + 60 * 60 * 1000);
      const r = formatRelativeTime(future, { locale: 'en-US' });
      expect(r).toMatch(/in/);
    });

    it('uses second granularity for sub-minute deltas', () => {
      const veryRecent = new Date(Date.now() - 5_000);
      const r = formatRelativeTime(veryRecent, { locale: 'en-US' });
      expect(r).toMatch(/second|now/);
    });

    it('uses year granularity for >1y deltas', () => {
      const longAgo = new Date(Date.now() - 400 * 24 * 60 * 60 * 1000);
      const r = formatRelativeTime(longAgo, { locale: 'en-US' });
      expect(r).toMatch(/year/);
    });
  });

  describe('number formatting', () => {
    it('formatNumber adds thousands separators', () => {
      expect(formatNumber(1234567.89, { locale: 'en-US' })).toBe('1,234,567.89');
    });

    it('formatNumber respects minimumFractionDigits', () => {
      expect(
        formatNumber(5, {
          locale: 'en-US',
          minimumFractionDigits: 2,
          maximumFractionDigits: 2,
        }),
      ).toBe('5.00');
    });

    it('formatCompactNumber is compact', () => {
      expect(formatCompactNumber(1500, { locale: 'en-US' })).toMatch(/1\.5K/);
      expect(formatCompactNumber(2_500_000, { locale: 'en-US' })).toMatch(/2\.5M/);
    });

    it('formatPercent uses the percent style', () => {
      expect(formatPercent(0.1234, { locale: 'en-US' })).toBe('12.34%');
      expect(formatPercent(0.5, { locale: 'en-US' })).toBe('50%');
    });
  });

  describe('formatPhone', () => {
    it('formats international numbers with three-digit country code', () => {
      expect(formatPhone('+421123456789')).toBe('+421 123 456 789');
    });

    it('prepends country code if requested and not present', () => {
      expect(formatPhone('123456789', { countryCode: '+421' })).toBe('+421 123 456 789');
    });

    it('strips leading zero before applying country code', () => {
      expect(formatPhone('0123456789', { countryCode: '+421' })).toBe('+421 123 456 789');
    });

    it('groups short numbers in threes', () => {
      // No country code path: just spaces every 3
      expect(formatPhone('123456')).toBe('123 456');
    });
  });

  describe('formatAddress', () => {
    it('joins street + number, postal + city, country', () => {
      expect(
        formatAddress({
          street: 'Main St',
          streetNumber: '123',
          postalCode: '811 01',
          city: 'Bratislava',
          country: 'Slovakia',
        }),
      ).toBe('Main St 123, 811 01 Bratislava, Slovakia');
    });

    it('omits empty parts', () => {
      expect(formatAddress({ city: 'Prague' })).toBe('Prague');
      expect(formatAddress({})).toBe('');
    });

    it('handles street without number', () => {
      expect(formatAddress({ street: 'Main St', city: 'Prague' })).toBe('Main St, Prague');
    });

    it('handles city without postal code', () => {
      expect(formatAddress({ street: 'X', city: 'Y' })).toBe('X, Y');
    });
  });

  describe('formatFileSize', () => {
    it('formats bytes', () => {
      expect(formatFileSize(0)).toBe('0 B');
      expect(formatFileSize(512)).toBe('512 B');
      expect(formatFileSize(1023)).toBe('1,023 B');
    });

    it('formats KB', () => {
      expect(formatFileSize(1024)).toBe('1 KB');
      expect(formatFileSize(1536)).toBe('1.5 KB');
    });

    it('formats MB', () => {
      expect(formatFileSize(1_048_576)).toBe('1 MB');
      expect(formatFileSize(5 * 1_048_576)).toBe('5 MB');
    });

    it('caps at TB for very large values', () => {
      const huge = 1024 ** 5; // 1 PB worth
      expect(formatFileSize(huge)).toMatch(/TB$/);
    });
  });
});
