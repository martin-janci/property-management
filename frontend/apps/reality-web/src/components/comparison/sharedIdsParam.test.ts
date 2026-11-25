/**
 * Regression tests for `/compare?ids=` query-parameter narrowing.
 *
 * Bug (`code-review-reality-web-comparison-unvalidated-cast`): the compare
 * server component typed `searchParams` as `{ ids?: string }` and called
 * `params.ids?.split(',')` on it. Next.js App Router actually types each
 * `searchParams` value as `string | string[] | undefined` — a repeated query
 * parameter (`?ids=a&ids=b`) arrives as a `string[]`. The `{ ids?: string }`
 * annotation was therefore an unvalidated cast, and `.split` does not exist on
 * an array, so a crafted/duplicated `ids` param threw
 * `TypeError: params.ids.split is not a function` and crashed the whole
 * `/compare` route with a 500.
 *
 * `parseSharedIdsParam` narrows the raw value at the boundary instead of
 * trusting the cast.
 */
/// <reference types="vitest/globals" />

import { describe, expect, it } from 'vitest';

import { parseSharedIdsParam } from './sharedIdsParam';

describe('parseSharedIdsParam', () => {
  it('splits a single comma-joined string value', () => {
    expect(parseSharedIdsParam('a,b,c')).toEqual(['a', 'b', 'c']);
  });

  it('returns a single id for a bare string', () => {
    expect(parseSharedIdsParam('abc-123')).toEqual(['abc-123']);
  });

  it('returns [] for an undefined param', () => {
    expect(parseSharedIdsParam(undefined)).toEqual([]);
  });

  it('returns [] for an empty string', () => {
    expect(parseSharedIdsParam('')).toEqual([]);
  });

  it('drops empty segments from trailing/double commas', () => {
    expect(parseSharedIdsParam('a,,b,')).toEqual(['a', 'b']);
  });

  // The regression bar: a repeated `?ids=a&ids=b` query param reaches the
  // server component as a `string[]`. The old unchecked cast called
  // `string[].split`, which throws. The narrowed helper must fold the array
  // into the same id list instead of throwing.
  it('normalizes a repeated (array) param without throwing', () => {
    expect(() => parseSharedIdsParam(['a', 'b'])).not.toThrow();
    expect(parseSharedIdsParam(['a', 'b'])).toEqual(['a', 'b']);
  });

  it('flattens comma-joined segments inside an array param', () => {
    expect(parseSharedIdsParam(['a,b', 'c'])).toEqual(['a', 'b', 'c']);
  });

  it('drops empty segments from an array param', () => {
    expect(parseSharedIdsParam(['a', '', 'b'])).toEqual(['a', 'b']);
  });
});
