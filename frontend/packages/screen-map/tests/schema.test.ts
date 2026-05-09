import { describe, expect, it } from 'vitest';
import { ScreenMapFrontmatterSchema } from '../src/schema.js';

describe('ScreenMapFrontmatterSchema', () => {
  it('accepts a minimal valid frontmatter', () => {
    const valid = {
      id: 'ppt/building-detail',
      name: 'Building Detail',
      product: 'ppt',
      implementations: {
        'ppt-web': {
          route: '/buildings/:id',
          buildStatus: 'shipped',
          redesignStatus: 'applied',
          apiStatus: 'complete',
        },
      },
    };
    const result = ScreenMapFrontmatterSchema.safeParse(valid);
    expect(result.success).toBe(true);
  });

  it('rejects an unknown product', () => {
    const result = ScreenMapFrontmatterSchema.safeParse({
      id: 'foo/bar',
      name: 'Foo',
      product: 'foo',
      implementations: {},
    });
    expect(result.success).toBe(false);
  });

  it('rejects an unknown buildStatus', () => {
    const result = ScreenMapFrontmatterSchema.safeParse({
      id: 'ppt/x',
      name: 'X',
      product: 'ppt',
      implementations: {
        'ppt-web': {
          buildStatus: 'launched',
          redesignStatus: 'applied',
          apiStatus: 'complete',
        },
      },
    });
    expect(result.success).toBe(false);
  });

  it('requires id to match <product>/<slug> pattern', () => {
    const result = ScreenMapFrontmatterSchema.safeParse({
      id: 'building-detail',
      name: 'Building Detail',
      product: 'ppt',
      implementations: {},
    });
    expect(result.success).toBe(false);
  });

  it('requires lastReview to be ISO date if present', () => {
    const result = ScreenMapFrontmatterSchema.safeParse({
      id: 'ppt/x',
      name: 'X',
      product: 'ppt',
      implementations: {},
      lastReview: '01/05/2026',
    });
    expect(result.success).toBe(false);
  });

  it('accepts lastReview as a JS Date object (gray-matter coerces unquoted ISO dates)', () => {
    const result = ScreenMapFrontmatterSchema.safeParse({
      id: 'ppt/x',
      name: 'X',
      product: 'ppt',
      implementations: {
        'ppt-web': {
          buildStatus: 'shipped',
          redesignStatus: 'not-started',
          apiStatus: 'complete',
        },
      },
      lastReview: new Date('2026-05-07T00:00:00.000Z'),
    });
    expect(result.success).toBe(true);
    expect(result.data?.lastReview).toBe('2026-05-07');
  });
});
