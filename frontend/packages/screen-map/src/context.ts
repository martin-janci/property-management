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
import { parseScreenMap } from './parse.js';
import type { ValidationContext } from './validate.js';

export interface BuildContextOptions {
  repoRoot: string;
  /** Defaults to `<repoRoot>/docs/screens`. */
  screensDir?: string;
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

  const screenFiles = await discoverScreenMaps(screensDir);
  const knownScreenIds = new Set<string>();
  for (const file of screenFiles) {
    try {
      const screen = await parseScreenMap(file);
      knownScreenIds.add(screen.frontmatter.id);
    } catch {
      // ignore here — the CLI itself reports per-file errors below
    }
  }

  return {
    knownEndpointIds,
    knownSitemapIds,
    knownScreenIds,
    resolveDiagramRef: (ref) => resolveDiagramRef(ref, options.repoRoot),
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
  let m: RegExpExecArray | null = headingRe.exec(markdown);
  while (m !== null) {
    slugs.add(slugify(m[1]));
    m = headingRe.exec(markdown);
  }
  return slugs;
}

function slugify(s: string): string {
  return s
    .toLowerCase()
    .replace(/[^a-z0-9\s-]/g, '')
    .trim()
    .replace(/\s+/g, '-');
}
