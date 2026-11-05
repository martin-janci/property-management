/// <reference types="vitest/globals" />
/**
 * Tests for the Community formatting helpers.
 *
 * Locale-aware Intl.* output varies by ICU version, so money assertions pin a
 * locale and match on digit groups / decimals rather than the full string
 * (currency symbol + spacing differ across ICU builds).
 */

import type { TFunction } from 'i18next';
import { formatMoney, personLabel } from './format';

// A minimal TFunction stand-in: echoes the i18n key so assertions can check
// which branch personLabel took without pulling in i18next.
const t = ((key: string) => key) as unknown as TFunction;

describe('formatMoney', () => {
  it('returns null for empty / null input', () => {
    expect(formatMoney(null, 'EUR', 'en-US')).toBeNull();
    expect(formatMoney('', 'EUR', 'en-US')).toBeNull();
  });

  it('returns null for a non-numeric string', () => {
    expect(formatMoney('not-a-number', 'EUR', 'en-US')).toBeNull();
  });

  it('formats an ordinary decimal string', () => {
    const out = formatMoney('25.00', 'EUR', 'en-US');
    expect(out).toContain('25');
    expect(out).toContain('.00');
  });

  it('defaults to EUR when no currency is supplied', () => {
    const out = formatMoney('10.00', null, 'en-US');
    // en-US renders EUR with the € symbol.
    expect(out).toContain('€');
  });

  // Regression (issue #3310): a server Decimal string larger than 2^53 must not
  // be round-tripped through an IEEE-754 double. Number('12345678901234567.89')
  // collapses to 12345678901234568, so coercing before Intl.NumberFormat loses
  // the exact value. Passing the decimal string straight to Intl preserves it.
  it('preserves precision for values beyond Number.MAX_SAFE_INTEGER', () => {
    const out = formatMoney('12345678901234567.89', 'USD', 'en-US');
    expect(out).toContain('12,345,678,901,234,567');
    expect(out).toContain('.89');
    // The lossy coercion would have produced ...568.00.
    expect(out).not.toContain('568');
  });
});

describe('personLabel', () => {
  it('shows the anonymous label when the author posted anonymously', () => {
    expect(personLabel('u1', 'u1', t, true)).toBe('community.hub.anonymous');
  });

  it('shows "you" for the current viewer', () => {
    expect(personLabel('u1', 'u1', t, false)).toBe('community.hub.you');
  });

  it('falls back to the neighbor label when no name is available', () => {
    expect(personLabel('u2', 'u1', t, false)).toBe('community.hub.neighbor');
  });
});
