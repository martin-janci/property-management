import type { CandidateScreen } from './scan.js';

export type GroupingDecision =
  | { type: 'merge'; from: string[]; into: string; name?: string }
  | { type: 'skip'; ids: string[] }
  | { type: 'rename'; from: string; to: string; name?: string };

export function mergeCandidates(
  candidates: CandidateScreen[],
  decisions: GroupingDecision[]
): CandidateScreen[] {
  let working = [...candidates];

  for (const decision of decisions) {
    if (decision.type === 'skip') {
      const skipSet = new Set(decision.ids);
      working = working.filter((c) => !skipSet.has(c.id));
      continue;
    }
    if (decision.type === 'rename') {
      const target = working.find((c) => c.id === decision.from);
      if (!target) continue;
      target.id = decision.to;
      if (decision.name) target.name = decision.name;
      continue;
    }
    if (decision.type === 'merge') {
      const fromSet = new Set(decision.from);
      const merged = working.filter((c) => fromSet.has(c.id));
      if (merged.length === 0) continue;
      const hasFrame = merged.some((m) => m.frameId);
      const frameId = merged.find((m) => m.frameId)?.frameId;
      const combined: CandidateScreen = {
        id: decision.into,
        name: decision.name ?? merged[0].name,
        product: merged[0].product,
        // If any merged candidate brings a design frame, the merged concept
        // is design-driven — buildStatus stays 'shipped' from sitemap parents,
        // but redesignStatus becomes 'in-progress' via init-write's source check.
        source: hasFrame ? 'design' : merged[0].source,
        sitemapRefs: mergeSitemapRefs(merged),
        useCases: dedupe(merged.flatMap((m) => m.useCases ?? [])),
        epics: dedupe(merged.flatMap((m) => m.epics ?? [])),
        frameId,
      };
      working = working.filter((c) => !fromSet.has(c.id));
      working.push(combined);
    }
  }

  return working;
}

function mergeSitemapRefs(merged: CandidateScreen[]): CandidateScreen['sitemapRefs'] {
  const result: NonNullable<CandidateScreen['sitemapRefs']> = {};
  for (const c of merged) {
    if (!c.sitemapRefs) continue;
    for (const [platform, id] of Object.entries(c.sitemapRefs)) {
      if (id) result[platform as keyof typeof result] = id;
    }
  }
  return Object.keys(result).length > 0 ? result : undefined;
}

function dedupe<T>(arr: T[]): T[] | undefined {
  if (arr.length === 0) return undefined;
  return [...new Set(arr)];
}
