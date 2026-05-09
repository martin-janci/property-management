import { describe, expect, it } from 'vitest';
import type { ScreenMap, ValidationContext } from '../src/index.js';
import { validateScreenMap } from '../src/validate.js';

const ctx: ValidationContext = {
  knownEndpointIds: new Set(['building_get', 'building_update', 'units_list']),
  knownSitemapIds: new Set([
    'ppt-building-detail',
    'mobile-building-detail-screen',
    'ppt-buildings-list',
  ]),
  knownScreenIds: new Set(['ppt/building-detail', 'ppt/buildings-list', 'ppt/building-edit']),
  resolveDiagramRef: (ref) => ref === 'docs/sequence-diagrams.md#building-detail-load',
};

const baseScreen = (): ScreenMap => ({
  filePath: 'docs/screens/ppt/building-detail.md',
  body: '## Notes\n',
  frontmatter: {
    id: 'ppt/building-detail',
    name: 'Building Detail',
    product: 'ppt',
    sitemapRefs: {
      'ppt-web': 'ppt-building-detail',
      mobile: 'mobile-building-detail-screen',
    },
    implementations: {
      'ppt-web': {
        route: '/buildings/:id',
        buildStatus: 'shipped',
        redesignStatus: 'applied',
        apiStatus: 'complete',
      },
    },
    endpoints: ['building_get', 'units_list'],
    relatedScreens: [{ id: 'ppt/buildings-list', rel: 'parent' }],
    diagrams: [{ ref: 'docs/sequence-diagrams.md#building-detail-load', kind: 'sequence' }],
  },
});

describe('validateScreenMap', () => {
  it('returns no issues for a screen wired up correctly', () => {
    const issues = validateScreenMap(baseScreen(), ctx);
    expect(issues).toEqual([]);
  });

  it('flags an unknown endpoint id', () => {
    const screen = baseScreen();
    screen.frontmatter.endpoints!.push('mystery_endpoint');
    const issues = validateScreenMap(screen, ctx);
    expect(issues).toEqual([
      {
        severity: 'error',
        path: 'endpoints[2]',
        message: 'unknown endpoint id "mystery_endpoint" — not present in @ppt/sitemap',
      },
    ]);
  });

  it('flags an unknown sitemap ref', () => {
    const screen = baseScreen();
    screen.frontmatter.sitemapRefs = { 'ppt-web': 'ppt-bogus' };
    const issues = validateScreenMap(screen, ctx);
    expect(issues).toContainEqual({
      severity: 'error',
      path: 'sitemapRefs.ppt-web',
      message: 'unknown sitemap id "ppt-bogus" — not present in @ppt/sitemap',
    });
  });

  it('flags a related screen that does not exist', () => {
    const screen = baseScreen();
    screen.frontmatter.relatedScreens = [{ id: 'ppt/missing', rel: 'child' }];
    const issues = validateScreenMap(screen, ctx);
    expect(issues).toContainEqual({
      severity: 'error',
      path: 'relatedScreens[0].id',
      message: 'related screen "ppt/missing" does not exist',
    });
  });

  it('flags a diagram ref that does not resolve', () => {
    const screen = baseScreen();
    screen.frontmatter.diagrams = [{ ref: 'docs/no-such.md#anchor', kind: 'sequence' }];
    const issues = validateScreenMap(screen, ctx);
    expect(issues).toContainEqual({
      severity: 'error',
      path: 'diagrams[0].ref',
      message: 'diagram ref "docs/no-such.md#anchor" does not resolve',
    });
  });

  it('flags a product/id mismatch (covered by schema, but enforced again here as a guard)', () => {
    const screen = baseScreen();
    screen.frontmatter.id = 'reality/building-detail';
    screen.frontmatter.product = 'ppt';
    const issues = validateScreenMap(screen, ctx);
    expect(issues.some((i) => i.path === 'id')).toBe(true);
  });
});
