import { existsSync, readFileSync } from 'node:fs';
import path from 'node:path';
import {
  apiServerEndpoints,
  mobileScreens,
  pptWebRoutes,
  realityServerEndpoints,
  realityWebRoutes,
} from '@ppt/sitemap';
import { discoverScreenMaps } from './discover.js';
import { extractKnownContexts } from './extract-known.js';
import { parseScreenMap } from './parse.js';
import type { ScreenMap } from './types.js';
import type { ValidationContext } from './validate.js';

export interface BuildContextOptions {
  repoRoot: string;
  /** Defaults to `<repoRoot>/docs/screens`. */
  screensDir?: string;
  /**
   * Pre-parsed screens (e.g. produced by the CLI's main pass). When supplied,
   * `buildValidationContext` skips its own discover+parse to avoid doubling
   * disk IO + parsing cost on larger trees.
   */
  parsedScreens?: ScreenMap[];
}

/**
 * Build a ValidationContext by collecting sitemap IDs, screen-map IDs, and a
 * filesystem-aware diagram-ref resolver.
 */
export async function buildValidationContext(
  options: BuildContextOptions
): Promise<ValidationContext> {
  const screensDir = options.screensDir ?? path.join(options.repoRoot, 'docs/screens');

  // Note: `ApiEndpoint` in @ppt/sitemap uses `operationId` as its identifier
  // (no `id` field). Screen-maps reference endpoints by these operationId values.
  const knownEndpointIds = new Set([
    ...apiServerEndpoints.map((e) => e.operationId),
    ...realityServerEndpoints.map((e) => e.operationId),
  ]);
  const knownSitemapIds = new Set([
    ...pptWebRoutes.map((r) => r.id),
    ...realityWebRoutes.map((r) => r.id),
    ...mobileScreens.map((s) => s.id),
  ]);

  const knownScreenIds = new Set<string>();
  if (options.parsedScreens) {
    for (const screen of options.parsedScreens) {
      knownScreenIds.add(screen.frontmatter.id);
    }
  } else {
    const screenFiles = await discoverScreenMaps(screensDir);
    for (const file of screenFiles) {
      try {
        const screen = await parseScreenMap(file);
        knownScreenIds.add(screen.frontmatter.id);
      } catch {
        // ignore here — the CLI itself reports per-file errors below
      }
    }
  }

  const known = await extractKnownContexts(options.repoRoot);

  return {
    knownEndpointIds,
    knownSitemapIds,
    knownScreenIds,
    resolveDiagramRef: (ref) => resolveDiagramRef(ref, options.repoRoot),
    knownUseCases: known.knownUseCases,
    knownEpics: known.knownEpics,
    knownComponents: known.knownComponents,
  };
}

function resolveDiagramRef(ref: string, repoRoot: string): boolean {
  const [filePart, anchor] = ref.split('#');
  if (!filePart) return false;
  const abs = path.isAbsolute(filePart) ? filePart : path.join(repoRoot, filePart);
  if (!existsSync(abs)) return false;
  if (!anchor) return true;
  // Best-effort: check the anchor appears as a `#`/`##`/... heading slug.
  try {
    const content = readFileSync(abs, 'utf8');
    const slugs = extractHeadingSlugs(content);
    return slugs.has(anchor);
  } catch {
    return false;
  }
}

function extractHeadingSlugs(markdown: string): Set<string> {
  const slugs = new Set<string>();
  const headingRe = /^#{1,6}\s+(.+?)\s*$/gm;
  for (let m = headingRe.exec(markdown); m !== null; m = headingRe.exec(markdown)) {
    const text = m[1];
    // ASCII slug (current behavior) AND a Unicode-preserving slug (lowercased,
    // with spaces hyphenated but diacritics intact). Both forms resolve.
    slugs.add(slugify(text));
    slugs.add(text.toLowerCase().trim().replace(/\s+/g, '-'));
  }
  return slugs;
}

function slugify(s: string): string {
  return (
    s
      .toLowerCase()
      .normalize('NFKD')
      // Strip combining marks (Unicode "Mark, Nonspacing") — preserves base letter.
      .replace(/\p{Mn}/gu, '')
      .replace(/[^a-z0-9\s-]/g, '')
      .trim()
      .replace(/\s+/g, '-')
  );
}
