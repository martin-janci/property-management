import { createHash } from 'node:crypto';
import { readdir, readFile } from 'node:fs/promises';
import path from 'node:path';
import { mobileScreens, pptWebRoutes, realityWebRoutes } from '@ppt/sitemap';
import type { DesignSource } from './design-source/index.js';
import { IdSchema } from './schema.js';
import type { Platform, Product } from './types.js';

export interface CandidateScreen {
  /** Heuristic id; user can override during interactive grouping. */
  id: string;
  name: string;
  product: Product;
  source: 'sitemap' | 'use-cases' | 'epics' | 'design' | 'user';
  /** Sitemap-side IDs that this candidate ties to (for sitemap sources). */
  sitemapRefs?: Partial<Record<Platform, string>>;
  useCases?: string[];
  epics?: string[];
  /** DesignSource frame id, if source === 'design'. */
  frameId?: string;
}

export interface ScanOptions {
  product: Product;
  repoRoot: string;
  /** Path to docs/use-cases.md; defaults to `<repoRoot>/docs/use-cases.md`. */
  useCasesFile?: string;
  /** Path to epic dir; defaults to `<repoRoot>/docs/epics`. */
  epicsDir?: string;
  sources: {
    sitemap: boolean;
    useCases: boolean;
    epics: boolean;
    designSource: DesignSource | undefined;
    userAdd: string[];
  };
}

export async function scanCandidates(opts: ScanOptions): Promise<CandidateScreen[]> {
  const out: CandidateScreen[] = [];
  const product = opts.product;

  if (opts.sources.sitemap) {
    out.push(...scanSitemap(product));
  }
  if (opts.sources.useCases) {
    const file = opts.useCasesFile ?? path.join(opts.repoRoot, 'docs/use-cases.md');
    out.push(...(await scanUseCases(file, product)));
  }
  if (opts.sources.epics) {
    const dir = opts.epicsDir ?? path.join(opts.repoRoot, 'docs/epics');
    out.push(...(await scanEpics(dir, product)));
  }
  if (opts.sources.designSource) {
    out.push(...(await scanDesignSource(opts.sources.designSource, product)));
  }
  for (const name of opts.sources.userAdd) {
    out.push({
      id: `${product}/${slugifyName(name)}`,
      name,
      product,
      source: 'user',
    });
  }

  // Boundary guard: every synthesized id is written verbatim by `screens init`
  // (and the filename is slugged from it), so an id that fails `IdSchema` here
  // produces a screen-map that immediately fails `screens validate` (see #2367,
  // follow-up to #2344). Per-source slugging can still emit an invalid id — a
  // name with no Latin/alphanumeric characters (e.g. a non-Latin Figma frame
  // name, or `--add ""`) slugifies to empty, yielding a bare `ppt/`. Validate
  // each candidate's id at the boundary and fall back to a stable, non-empty,
  // schema-valid slug rather than letting the invalid id escape.
  for (const c of out) {
    if (!IdSchema.safeParse(c.id).success) {
      c.id = `${c.product}/${slugifyName(c.name)}`;
    }
  }
  return out;
}

function scanSitemap(product: Product): CandidateScreen[] {
  const out: CandidateScreen[] = [];
  if (product === 'ppt') {
    for (const r of pptWebRoutes) {
      const display = r.name ?? r.id;
      out.push({
        id: `ppt/${slugifyName(display)}`,
        name: display,
        product: 'ppt',
        source: 'sitemap',
        sitemapRefs: { 'ppt-web': r.id },
      });
    }
    for (const s of mobileScreens) {
      const display = s.name ?? s.screenName ?? s.id;
      out.push({
        id: `ppt/${slugifyName(display)}`,
        name: display,
        product: 'ppt',
        source: 'sitemap',
        sitemapRefs: { mobile: s.id },
      });
    }
  } else {
    for (const r of realityWebRoutes) {
      const display = r.name ?? r.id;
      out.push({
        id: `reality/${slugifyName(display)}`,
        name: display,
        product: 'reality',
        source: 'sitemap',
        sitemapRefs: { 'reality-web': r.id },
      });
    }
    // mobile-native (KMP) is not in @ppt/sitemap as routes; defer to user-add.
  }
  return out;
}

async function scanUseCases(file: string, product: Product): Promise<CandidateScreen[]> {
  let content: string;
  try {
    content = await readFile(file, 'utf8');
  } catch {
    return [];
  }
  // Match both `UC-NN` (category) and `UC-NN.M` (story).
  const matches = content.matchAll(/\bUC-(\d+(?:\.\d+)?)\b/g);
  const ucIds = new Set<string>();
  for (const m of matches) {
    ucIds.add(`UC-${m[1]}`);
  }
  // One synthetic candidate per UC id; user merges into concepts during grouping.
  return [...ucIds].map((id) => ({
    id: `${product}/${slugifyName(id)}`,
    name: id,
    product,
    source: 'use-cases' as const,
    useCases: [id],
  }));
}

async function scanEpics(dir: string, product: Product): Promise<CandidateScreen[]> {
  let entries: string[];
  try {
    entries = await readdir(dir);
  } catch {
    return [];
  }
  // Capture an optional single upper-cased letter suffix (10A/10B/7B
  // convention, see docs/EPIC_STORY_STATUS.md) bounded by the segment end, and
  // strip leading zeros so the synthesized candidate matches the unpadded
  // `Epic-10A` frontmatter refs. The `(?=[-.]|$)` boundary rejects malformed
  // ids (`EPIC-10beta`, `EPIC-7A2-*`) instead of silently mis-binning them.
  const epics = entries
    .map((entry) => entry.match(/^EPIC-0*(\d+[A-Z]?)(?=[-.]|$)/i)?.[1]?.toUpperCase())
    .filter((id): id is string => Boolean(id));
  return [...new Set(epics)].map((num) => ({
    // Human-facing fields keep the upper-cased suffix (`Epic-10B`) so they
    // match frontmatter refs; the id slug is lower-cased to satisfy the
    // schema id regex (`/^[a-z...]/`) — see schema.ts IdSchema.
    id: `${product}/epic-${num.toLowerCase()}`,
    name: `Epic-${num}`,
    product,
    source: 'epics' as const,
    epics: [`Epic-${num}`],
  }));
}

async function scanDesignSource(
  source: DesignSource,
  product: Product
): Promise<CandidateScreen[]> {
  const frames = await source.list();
  return frames.map((frame) => ({
    id: `${product}/${slugifyName(frame.name)}`,
    name: frame.name,
    product,
    source: 'design' as const,
    frameId: frame.id,
  }));
}

function slugifyName(s: string): string {
  const slug = s
    .toLowerCase()
    .normalize('NFKD')
    .replace(/\p{Mn}/gu, '')
    .replace(/[^a-z0-9\s-]/g, '')
    .trim()
    .replace(/\s+/g, '-')
    .replace(/-+/g, '-')
    .replace(/^-+|-+$/g, '');
  // Never return an empty (or hyphen-only) slug: a name with no Latin
  // alphanumerics (non-Latin script, pure emoji/symbols) would otherwise
  // collapse to '' and yield a bare `<product>/`, which fails IdSchema. Fall
  // back to a stable content hash so the id stays deterministic and unique.
  return slug || `screen-${hashShort(s)}`;
}

/** Short, stable hex digest used as a deterministic slug fallback. */
function hashShort(s: string): string {
  return createHash('sha1').update(s).digest('hex').slice(0, 8);
}
