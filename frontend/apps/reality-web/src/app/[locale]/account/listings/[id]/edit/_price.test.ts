/// <reference types="vitest/globals" />

import { describe, expect, it } from 'vitest';
import { resolveListingPrice } from './_price';

/**
 * Regression for the listing-edit `handleSave` price coercion.
 *
 * The old `Number(form.price) || 0` coercion silently persisted a €0 price when
 * the field was empty / invalid. `resolveListingPrice` must instead return
 * `null` for anything that is not a genuinely-entered positive number, so the
 * caller can reject it with validation rather than quietly saving 0.
 */
describe('resolveListingPrice', () => {
  it('returns the numeric value for a genuinely-entered positive price', () => {
    expect(resolveListingPrice(245000)).toBe(245000);
  });

  it('rejects an empty field instead of silently persisting 0', () => {
    expect(resolveListingPrice('')).toBeNull();
  });

  it('rejects a zero price', () => {
    expect(resolveListingPrice(0)).toBeNull();
  });

  it('rejects a negative price', () => {
    expect(resolveListingPrice(-5)).toBeNull();
  });

  it('rejects a non-finite price', () => {
    expect(resolveListingPrice(Number.NaN)).toBeNull();
  });
});
