import type { ScreenMap } from './types.js';
import type { ValidationContext } from './validate.js';

export type DriftIssue =
  | { kind: 'unmapped-sitemap'; sitemapId: string }
  | { kind: 'unknown-endpoint'; screenId: string; endpointId: string }
  | { kind: 'unknown-component'; screenId: string; component: string }
  | { kind: 'unknown-use-case'; screenId: string; useCaseId: string }
  | { kind: 'unknown-epic'; screenId: string; epicId: string }
  | { kind: 'orphan-screen'; screenId: string; sitemapId: string };

export interface ScanDriftOptions {
  screens: ScreenMap[];
  context: ValidationContext;
  /** Optional: known component export list. If omitted, no component check runs. */
  knownComponents?: Set<string>;
  /** Optional: known UC IDs (from docs/use-cases.md). If omitted, no UC check runs. */
  knownUseCases?: Set<string>;
  /** Optional: known epic IDs. If omitted, no epic check runs. */
  knownEpics?: Set<string>;
}

export function scanDrift(opts: ScanDriftOptions): DriftIssue[] {
  const issues: DriftIssue[] = [];
  const { screens, context } = opts;

  // 1. Unmapped sitemap entries.
  const referencedSitemap = new Set<string>();
  for (const s of screens) {
    if (!s.frontmatter.sitemapRefs) continue;
    for (const id of Object.values(s.frontmatter.sitemapRefs)) {
      if (id) referencedSitemap.add(id);
    }
  }
  for (const sitemapId of context.knownSitemapIds) {
    if (!referencedSitemap.has(sitemapId)) {
      issues.push({ kind: 'unmapped-sitemap', sitemapId });
    }
  }

  // 2-5. Per-screen checks.
  for (const s of screens) {
    const screenId = s.frontmatter.id;

    // Unknown endpoints.
    for (const ep of s.frontmatter.endpoints ?? []) {
      if (!context.knownEndpointIds.has(ep)) {
        issues.push({ kind: 'unknown-endpoint', screenId, endpointId: ep });
      }
    }

    // Unknown components.
    if (opts.knownComponents) {
      for (const c of s.frontmatter.sharedComponents ?? []) {
        if (!opts.knownComponents.has(c)) {
          issues.push({ kind: 'unknown-component', screenId, component: c });
        }
      }
    }

    // Unknown use cases.
    if (opts.knownUseCases) {
      for (const uc of s.frontmatter.useCases ?? []) {
        if (!opts.knownUseCases.has(uc)) {
          issues.push({ kind: 'unknown-use-case', screenId, useCaseId: uc });
        }
      }
    }

    // Unknown epics.
    if (opts.knownEpics) {
      for (const epic of s.frontmatter.epics ?? []) {
        if (!opts.knownEpics.has(epic)) {
          issues.push({ kind: 'unknown-epic', screenId, epicId: epic });
        }
      }
    }

    // Orphan: sitemapRefs point at IDs that aren't in known sitemap.
    if (s.frontmatter.sitemapRefs) {
      for (const sid of Object.values(s.frontmatter.sitemapRefs)) {
        if (sid && !context.knownSitemapIds.has(sid)) {
          issues.push({ kind: 'orphan-screen', screenId, sitemapId: sid });
        }
      }
    }
  }

  return issues;
}
