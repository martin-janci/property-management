import { describe, expect, it } from 'vitest';
import { parseFilter } from '../src/cli.js';

const sample = {
  id: 'ppt/foo',
  product: 'ppt',
  implementations: {
    'ppt-web': { buildStatus: 'shipped', redesignStatus: 'in-progress', apiStatus: 'complete' },
  },
};

describe('parseFilter', () => {
  it('matches a simple top-level key:value', () => {
    expect(parseFilter('product:ppt')(sample)).toBe(true);
    expect(parseFilter('product:reality')(sample)).toBe(false);
  });

  it('matches a nested dotted path', () => {
    expect(parseFilter('implementations.ppt-web.buildStatus:shipped')(sample)).toBe(true);
    expect(parseFilter('implementations.ppt-web.redesignStatus:in-progress')(sample)).toBe(true);
  });

  it('ANDs comma-separated terms', () => {
    expect(parseFilter('product:ppt,implementations.ppt-web.buildStatus:shipped')(sample)).toBe(
      true
    );
    expect(parseFilter('product:ppt,implementations.ppt-web.buildStatus:planned')(sample)).toBe(
      false
    );
  });

  it('returns false for missing nested path segments', () => {
    expect(parseFilter('implementations.mobile.buildStatus:shipped')(sample)).toBe(false);
  });
});
