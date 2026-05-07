import { describe, expect, it } from 'vitest';
import { type GroupingDecision, mergeCandidates } from '../src/grouping.js';
import type { CandidateScreen } from '../src/scan.js';

const candidates: CandidateScreen[] = [
  { id: 'ppt/foo', name: 'Foo', product: 'ppt', source: 'sitemap' },
  { id: 'ppt/bar', name: 'Bar', product: 'ppt', source: 'sitemap' },
  { id: 'ppt/baz', name: 'Baz', product: 'ppt', source: 'use-cases', useCases: ['UC-12'] },
];

describe('mergeCandidates', () => {
  it('passes candidates through unchanged when there are no decisions', () => {
    const result = mergeCandidates(candidates, []);
    expect(result).toHaveLength(3);
  });

  it('merges multiple candidates into one concept by id', () => {
    const decisions: GroupingDecision[] = [
      { type: 'merge', from: ['ppt/foo', 'ppt/bar'], into: 'ppt/foo-bar', name: 'Foo Bar' },
    ];
    const result = mergeCandidates(candidates, decisions);
    expect(result).toHaveLength(2);
    const merged = result.find((c) => c.id === 'ppt/foo-bar');
    expect(merged?.name).toBe('Foo Bar');
  });

  it('skips candidates listed in skip decisions', () => {
    const decisions: GroupingDecision[] = [{ type: 'skip', ids: ['ppt/baz'] }];
    const result = mergeCandidates(candidates, decisions);
    expect(result).toHaveLength(2);
    expect(result.find((c) => c.id === 'ppt/baz')).toBeUndefined();
  });

  it('renames a candidate via decision', () => {
    const decisions: GroupingDecision[] = [
      {
        type: 'rename',
        from: 'ppt/baz',
        to: 'ppt/building-management',
        name: 'Building Management',
      },
    ];
    const result = mergeCandidates(candidates, decisions);
    const renamed = result.find((c) => c.id === 'ppt/building-management');
    expect(renamed?.name).toBe('Building Management');
    expect(renamed?.useCases).toEqual(['UC-12']); // preserved from baz
  });
});
