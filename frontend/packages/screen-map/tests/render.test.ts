import { describe, expect, it } from 'vitest';
import { renderEndpointMatrix, renderSiteGraph, renderStatusDashboard } from '../src/render.js';
import type { ScreenMap } from '../src/types.js';

const screens: ScreenMap[] = [
  {
    filePath: 'docs/screens/ppt/buildings-list.md',
    body: '',
    frontmatter: {
      id: 'ppt/buildings-list',
      name: 'Buildings List',
      product: 'ppt',
      implementations: {
        'ppt-web': { buildStatus: 'shipped', redesignStatus: 'applied', apiStatus: 'complete' },
      },
      endpoints: ['buildings_list'],
      relatedScreens: [{ id: 'ppt/building-detail', rel: 'child' }],
    },
  },
  {
    filePath: 'docs/screens/ppt/building-detail.md',
    body: '',
    frontmatter: {
      id: 'ppt/building-detail',
      name: 'Building Detail',
      product: 'ppt',
      implementations: {
        'ppt-web': {
          buildStatus: 'in-progress',
          redesignStatus: 'in-progress',
          apiStatus: 'partial',
        },
      },
      endpoints: ['building_get', 'units_list'],
      relatedScreens: [{ id: 'ppt/buildings-list', rel: 'parent' }],
    },
  },
];

describe('renderSiteGraph', () => {
  it('emits a Mermaid graph TD with screens as nodes and rel edges', () => {
    const out = renderSiteGraph(screens);
    expect(out).toMatch(/^graph TD/);
    expect(out).toMatch(/ppt\/buildings-list/);
    expect(out).toMatch(/ppt\/building-detail/);
    // Edge should appear once (the symmetrical parent/child generates one edge each direction; dedupe is fine).
    expect(out).toMatch(/ppt\/buildings-list .* ppt\/building-detail/);
  });
});

describe('renderEndpointMatrix', () => {
  it('emits a markdown table of screens × endpoints with check marks', () => {
    const out = renderEndpointMatrix(screens);
    expect(out).toMatch(/^\| Screen \| building_get \| buildings_list \| units_list \|/m);
    expect(out).toMatch(/\| ppt\/building-detail \| ✓ \| {2}\| ✓ \|/);
    expect(out).toMatch(/\| ppt\/buildings-list \| {2}\| ✓ \| {2}\|/);
  });
});

describe('renderStatusDashboard', () => {
  it('emits Mermaid pie charts per platform per axis', () => {
    const out = renderStatusDashboard(screens);
    // Three axes (build, redesign, api) per platform; one platform here.
    expect(out).toMatch(/pie .*ppt-web build/i);
    expect(out).toMatch(/pie .*ppt-web redesign/i);
    expect(out).toMatch(/pie .*ppt-web api/i);
    // Counts: shipped=1, in-progress=1.
    expect(out).toMatch(/"shipped" : 1/);
    expect(out).toMatch(/"in-progress" : 1/);
  });
});
