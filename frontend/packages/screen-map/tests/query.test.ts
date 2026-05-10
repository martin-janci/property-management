import { describe, expect, it } from 'vitest';
import { formatQueryResult, queryScreens } from '../src/query.js';
import type { ScreenMap } from '../src/types.js';

const screens: ScreenMap[] = [
  {
    filePath: 'docs/screens/ppt/foo.md',
    body: '',
    frontmatter: {
      id: 'ppt/foo',
      name: 'Foo',
      product: 'ppt',
      implementations: {
        'ppt-web': { buildStatus: 'shipped', redesignStatus: 'in-progress', apiStatus: 'complete' },
      },
    },
  },
  {
    filePath: 'docs/screens/ppt/bar.md',
    body: '',
    frontmatter: {
      id: 'ppt/bar',
      name: 'Bar',
      product: 'ppt',
      implementations: {
        'ppt-web': { buildStatus: 'planned', redesignStatus: 'not-started', apiStatus: 'stub' },
      },
    },
  },
  {
    filePath: 'docs/screens/reality/baz.md',
    body: '',
    frontmatter: {
      id: 'reality/baz',
      name: 'Baz',
      product: 'reality',
      implementations: {
        'reality-web': { buildStatus: 'shipped', redesignStatus: 'applied', apiStatus: 'complete' },
      },
    },
  },
];

describe('queryScreens', () => {
  it('returns all screens when filter is empty', () => {
    const out = queryScreens(screens, '');
    expect(out).toHaveLength(3);
  });

  it('filters by top-level product', () => {
    const out = queryScreens(screens, 'product:ppt');
    expect(out.map((s) => s.frontmatter.id).sort()).toEqual(['ppt/bar', 'ppt/foo']);
  });

  it('filters by nested implementation status', () => {
    const out = queryScreens(screens, 'implementations.ppt-web.buildStatus:shipped');
    expect(out).toHaveLength(1);
    expect(out[0].frontmatter.id).toBe('ppt/foo');
  });
});

describe('formatQueryResult', () => {
  it('formats as a markdown table', () => {
    const out = formatQueryResult(screens.slice(0, 2), 'md');
    expect(out).toMatch(/^\| id \| name \| product \|/m);
    expect(out).toMatch(/\| ppt\/foo \| Foo \| ppt \|/);
    expect(out).toMatch(/\| ppt\/bar \| Bar \| ppt \|/);
  });

  it('formats as JSON', () => {
    const out = formatQueryResult(screens.slice(0, 1), 'json');
    const parsed = JSON.parse(out);
    expect(parsed).toHaveLength(1);
    expect(parsed[0].id).toBe('ppt/foo');
  });

  it('formats as plain table (terminal)', () => {
    const out = formatQueryResult(screens.slice(0, 2), 'table');
    expect(out).toContain('ppt/foo');
    expect(out).toContain('Foo');
  });
});
