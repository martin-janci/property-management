import { describe, expect, it } from 'vitest';
import { type DriftIssue, scanDrift } from '../src/scan-drift.js';
import type { ScreenMap } from '../src/types.js';
import type { ValidationContext } from '../src/validate.js';

const ctx: ValidationContext = {
  knownEndpointIds: new Set(['building_get', 'units_list']),
  knownSitemapIds: new Set([
    'ppt-buildings-list',
    'ppt-building-detail',
    'mobile-building-detail-screen',
  ]),
  knownScreenIds: new Set(['ppt/building-detail']),
  resolveDiagramRef: () => true,
};

const baseScreen: ScreenMap = {
  filePath: 'docs/screens/ppt/building-detail.md',
  body: '',
  frontmatter: {
    id: 'ppt/building-detail',
    name: 'Building Detail',
    product: 'ppt',
    sitemapRefs: { 'ppt-web': 'ppt-building-detail' },
    implementations: {
      'ppt-web': { buildStatus: 'shipped', redesignStatus: 'applied', apiStatus: 'complete' },
    },
    endpoints: ['building_get'],
  },
};

describe('scanDrift', () => {
  it('reports sitemap entries with no screen-map referencing them', () => {
    const issues = scanDrift({ screens: [baseScreen], context: ctx });
    const orphans = issues.filter((i) => i.kind === 'unmapped-sitemap');
    expect(orphans.map((i) => i.sitemapId).sort()).toEqual([
      'mobile-building-detail-screen',
      'ppt-buildings-list',
    ]);
  });

  it('reports screen-map endpoints not in sitemap', () => {
    const screen: ScreenMap = {
      ...baseScreen,
      frontmatter: { ...baseScreen.frontmatter, endpoints: ['building_get', 'mystery_endpoint'] },
    };
    const issues = scanDrift({ screens: [screen], context: ctx });
    const bad = issues.find((i) => i.kind === 'unknown-endpoint');
    expect(bad).toBeDefined();
    expect((bad as Extract<DriftIssue, { kind: 'unknown-endpoint' }>).endpointId).toBe(
      'mystery_endpoint'
    );
  });

  it('reports sharedComponents not in the export list', () => {
    const screen: ScreenMap = {
      ...baseScreen,
      frontmatter: {
        ...baseScreen.frontmatter,
        sharedComponents: ['BuildingHeader', 'NotARealComponent'],
      },
    };
    const issues = scanDrift({
      screens: [screen],
      context: ctx,
      knownComponents: new Set(['BuildingHeader']),
    });
    const bad = issues.find((i) => i.kind === 'unknown-component');
    expect(bad).toBeDefined();
    expect((bad as Extract<DriftIssue, { kind: 'unknown-component' }>).component).toBe(
      'NotARealComponent'
    );
  });

  it('reports useCases not in known set', () => {
    const screen: ScreenMap = {
      ...baseScreen,
      frontmatter: { ...baseScreen.frontmatter, useCases: ['UC-12', 'UC-99'] },
    };
    const issues = scanDrift({
      screens: [screen],
      context: ctx,
      knownUseCases: new Set(['UC-12']),
    });
    const bad = issues.find((i) => i.kind === 'unknown-use-case');
    expect(bad).toBeDefined();
    expect((bad as Extract<DriftIssue, { kind: 'unknown-use-case' }>).useCaseId).toBe('UC-99');
  });

  it('reports orphan screens whose sitemapRefs do not point to anything in sitemap', () => {
    const screen: ScreenMap = {
      ...baseScreen,
      frontmatter: {
        ...baseScreen.frontmatter,
        id: 'ppt/orphan',
        sitemapRefs: { 'ppt-web': 'ppt-orphan-route-deleted' },
      },
    };
    const issues = scanDrift({ screens: [screen], context: ctx });
    const orphan = issues.find((i) => i.kind === 'orphan-screen');
    expect(orphan).toBeDefined();
    expect((orphan as Extract<DriftIssue, { kind: 'orphan-screen' }>).screenId).toBe('ppt/orphan');
  });
});
