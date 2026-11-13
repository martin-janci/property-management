/// <reference types="vitest/globals" />

import { describe, expect, it } from 'vitest';
import { parseFiniteNumber, parsePageParam } from './_params';

/**
 * Regression for the listings-page URL param coercion.
 *
 * The old inline `Number(searchParams.get('page') || 1)` and
 * `if (priceMin) filters.priceMin = Number(priceMin)` let an attacker- or
 * user-supplied malformed param (`?page=abc`, `?priceMin=foo`) collapse to
 * `NaN` and flow straight into `useListings(filters, currentPage)`, producing a
 * `page=NaN` request and NaN-valued filters. These helpers must guard every
 * parse at the edge so a bad param degrades gracefully.
 */
describe('parseFiniteNumber (filter params)', () => {
  it('parses a valid numeric string', () => {
    expect(parseFiniteNumber('1500')).toBe(1500);
  });

  it('parses a decimal value', () => {
    expect(parseFiniteNumber('85.5')).toBe(85.5);
  });

  it('preserves a genuinely-supplied zero', () => {
    expect(parseFiniteNumber('0')).toBe(0);
  });

  it('returns undefined for a non-numeric param instead of NaN', () => {
    // Fails on the old `Number(priceMin)` coercion, which returned NaN and let
    // it flow into the filter set / listings query.
    expect(parseFiniteNumber('foo')).toBeUndefined();
  });

  it('returns undefined for Infinity', () => {
    expect(parseFiniteNumber('Infinity')).toBeUndefined();
  });

  it('returns undefined for a missing param', () => {
    expect(parseFiniteNumber(null)).toBeUndefined();
  });

  it('returns undefined for an empty / whitespace param', () => {
    expect(parseFiniteNumber('')).toBeUndefined();
    expect(parseFiniteNumber('   ')).toBeUndefined();
  });
});

describe('parsePageParam', () => {
  it('parses a valid page number', () => {
    expect(parsePageParam('3')).toBe(3);
  });

  it('defaults to 1 for a missing param', () => {
    expect(parsePageParam(null)).toBe(1);
  });

  it('defaults to 1 for a malformed param instead of yielding NaN', () => {
    // Fails on the old `Number(searchParams.get('page') || 1)`, which turned
    // `?page=abc` into NaN and requested page=NaN.
    expect(parsePageParam('abc')).toBe(1);
  });

  it('defaults to 1 for a fractional page', () => {
    expect(parsePageParam('2.5')).toBe(1);
  });

  it('defaults to 1 for a zero or negative page', () => {
    expect(parsePageParam('0')).toBe(1);
    expect(parsePageParam('-4')).toBe(1);
  });
});
