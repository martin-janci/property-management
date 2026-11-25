import { describe, expect, it } from 'vitest';
import { resolveListingTitle } from './_title';

const form = { description: 'Sunny flat near the park', propertyType: 'apartment', city: 'Nitra' };

describe('resolveListingTitle', () => {
  it('keeps the title the listing already has', () => {
    expect(resolveListingTitle('3-room flat, Nitra centre', form)).toBe(
      '3-room flat, Nitra centre'
    );
  });

  it('derives a title from the description only when there is none', () => {
    expect(resolveListingTitle('', form)).toBe('Sunny flat near the park');
    expect(resolveListingTitle('   ', form)).toBe('Sunny flat near the park');
    expect(resolveListingTitle(undefined, { ...form, description: 'x'.repeat(120) })).toHaveLength(
      80
    );
  });

  it('falls back to type and city without a description', () => {
    expect(resolveListingTitle(null, { ...form, description: '' })).toBe('apartment Nitra');
  });
});
