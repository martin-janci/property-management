/// <reference types="vitest/globals" />

import { describe, expect, it } from 'vitest';
import { resolveOptionalNumber } from './_number';

/**
 * Regression for the listing-edit `handleSave` numeric-field coercion
 * (sizeSqm / rooms / floor).
 *
 * The old inline `typeof x === 'number' ? x : Number(x) || undefined` coercion
 * was inconsistent: a numeric `0` was preserved, but the SAME logical value
 * arriving as the string `'0'` (the Decimal-serialized shape the backend can
 * send — see `MyListing` in lib/realtor-api.ts) was silently dropped by the
 * `|| undefined` fallback. That is the same silent-zero bug family already
 * fixed for price in #3197. `resolveOptionalNumber` must coerce consistently
 * and preserve a genuinely-entered zero regardless of how it was serialized.
 */
describe('resolveOptionalNumber', () => {
  it('returns the numeric value for a genuinely-entered number', () => {
    expect(resolveOptionalNumber(85)).toBe(85);
  });

  it('coerces a numeric string (Decimal serialized as string) to a number', () => {
    expect(resolveOptionalNumber('85.5')).toBe(85.5);
  });

  it('preserves a genuinely-entered zero that arrives as a string instead of dropping it', () => {
    // Fails on the old `Number(value) || undefined` coercion, which turned
    // the string '0' into undefined and silently omitted the field.
    expect(resolveOptionalNumber('0')).toBe(0);
  });

  it('preserves a numeric zero', () => {
    expect(resolveOptionalNumber(0)).toBe(0);
  });

  it('omits an empty field so the PATCH preserves the stored value', () => {
    expect(resolveOptionalNumber('')).toBeUndefined();
  });

  it('omits null / undefined', () => {
    expect(resolveOptionalNumber(null)).toBeUndefined();
    expect(resolveOptionalNumber(undefined)).toBeUndefined();
  });

  it('omits a non-finite / non-numeric value', () => {
    expect(resolveOptionalNumber(Number.NaN)).toBeUndefined();
    expect(resolveOptionalNumber('abc')).toBeUndefined();
  });
});
