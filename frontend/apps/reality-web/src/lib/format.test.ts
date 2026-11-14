/**
 * Unit tests for the locale-aware date helpers in `@/lib/format`.
 *
 * These guard the i18n contract the component-level regression tests
 * (ListingHeader / AdditionalInfo / agency price) rely on: the active
 * next-intl locale must drive date ordering/separators rather than the
 * runtime's ambient locale.
 */

import { describe, expect, it } from 'vitest';
import { formatDate, formatDateTime, formatPrice } from './format';

// Local-time constructor keeps the date portion stable regardless of the
// test runner's TZ (31 Dec 2025, 14:05 local).
const d = new Date(2025, 11, 31, 14, 5, 0);

describe('formatDate', () => {
  it('uses the active locale for ordering and separators', () => {
    // de-DE: day-first, dot separators.
    expect(formatDate(d, 'de')).toBe('31.12.2025');
    // en maps to en-GB: day-first, slash separators.
    expect(formatDate(d, 'en')).toBe('31/12/2025');
    // The two locales must not collapse to the same rendering.
    expect(formatDate(d, 'de')).not.toBe(formatDate(d, 'en'));
  });

  it('accepts ISO strings and epoch millis, not just Date objects', () => {
    expect(formatDate('2025-12-31T00:00:00', 'de')).toBe('31.12.2025');
    expect(formatDate(d.getTime(), 'de')).toBe('31.12.2025');
  });

  it('honours explicit DateTimeFormat options', () => {
    expect(formatDate(d, 'de', { year: 'numeric', month: 'long' })).toBe('Dezember 2025');
  });

  it('returns an empty string for an unparseable date instead of "Invalid Date"', () => {
    expect(formatDate('not-a-date', 'de')).toBe('');
    expect(formatDate(Number.NaN, 'sk')).toBe('');
  });
});

describe('formatPrice', () => {
  it('formats a valid currency code', () => {
    // Narrow non-breaking spaces vary by ICU build; assert on the pieces.
    const out = formatPrice(1234, 'sk', { currency: 'EUR' });
    expect(out).toMatch(/1.?234/);
    expect(out).toContain('€');
  });

  it('defaults to EUR when no currency is supplied', () => {
    expect(formatPrice(1000, 'de')).toContain('€');
  });

  // Regression: listing data flows in as `{ currency: listing.currency }`
  // straight from the API / mocks, and that value can be empty, a symbol,
  // or an otherwise invalid code. `Intl.NumberFormat` throws
  // `RangeError: Invalid currency code` for those, which crashed the
  // listing pages / SSR. These must not throw.
  it.each([
    ['empty string', ''],
    ['a currency symbol', '€'],
    ['wrong length', 'EU'],
    ['too long', 'EURO'],
    ['digits', '123'],
    ['unknown-but-well-formed code', 'ZZZ'],
  ])('does not throw for an invalid currency code (%s)', (_label, bad) => {
    expect(() => formatPrice(1234, 'sk', { currency: bad as string })).not.toThrow();
    // Output still renders the number rather than blowing up the page.
    expect(formatPrice(1234, 'sk', { currency: bad as string })).toMatch(/1.?234/);
  });

  it('falls back to EUR formatting for an invalid code', () => {
    // '€' is not a valid ISO 4217 code → guard swaps in EUR and still
    // renders a euro-denominated price.
    expect(formatPrice(1234, 'de', { currency: '€' })).toContain('€');
  });
});

describe('formatDateTime', () => {
  it('renders a locale-correct date plus a time component', () => {
    const out = formatDateTime(d, 'de');
    // German date grouping (dots) …
    expect(out).toMatch(/31\.12\.(20)?25/);
    // … and a time portion (colon-separated HH:MM).
    expect(out).toMatch(/\d{1,2}:\d{2}/);
    // Differs from the English rendering.
    expect(out).not.toBe(formatDateTime(d, 'en'));
  });
});
