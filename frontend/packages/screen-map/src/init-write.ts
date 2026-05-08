import { existsSync } from 'node:fs';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import type { CandidateScreen } from './scan.js';
import type { Platform, Product, ScreenMapFrontmatter } from './types.js';
import { writeScreenMapString } from './write.js';

export interface BulkWriteOptions {
  force?: boolean;
}

export interface BulkWriteResult {
  written: string[];
  /** Files that already existed and contained user edits — preserved (not overwritten). */
  skipped: string[];
}

export async function bulkWriteScreenMaps(
  concepts: CandidateScreen[],
  screensDir: string,
  options: BulkWriteOptions = {}
): Promise<BulkWriteResult> {
  const written: string[] = [];
  const skipped: string[] = [];
  for (const concept of concepts) {
    const slug = concept.id.split('/')[1];
    if (!slug) {
      throw new Error(`invalid concept id "${concept.id}" (no slug)`);
    }
    const dir = path.join(screensDir, concept.product);
    await mkdir(dir, { recursive: true });
    const file = path.join(dir, `${slug}.md`);
    if (existsSync(file) && !options.force) {
      throw new Error(`${file} already exists; pass force=true to overwrite`);
    }
    if (existsSync(file) && options.force) {
      // Re-read the existing file. If it still has the "init: created from
      // scan" marker, we treat it as template-shaped and overwrite. Otherwise
      // a human has edited it and we preserve their work.
      const existing = await readFile(file, 'utf8');
      if (!existing.includes('— init: created from scan')) {
        skipped.push(file);
        continue;
      }
    }
    const screen = {
      filePath: file,
      frontmatter: buildFrontmatter(concept),
      body: buildBody(concept),
    };
    const serialized = writeScreenMapString(screen);
    await writeFile(file, serialized, 'utf8');
    written.push(file);
  }
  return { written, skipped };
}

function buildFrontmatter(c: CandidateScreen): ScreenMapFrontmatter {
  const isDesigned = c.source === 'design';
  // A design-sourced concept that ALSO has sitemapRefs means a shipped
  // screen with a redesign queued. A pure-design concept (no sitemapRefs)
  // is a new screen not yet built.
  const hasSitemapRefs = c.sitemapRefs && Object.keys(c.sitemapRefs).length > 0;
  const buildStatus = isDesigned && !hasSitemapRefs ? 'planned' : 'shipped';
  const apiStatus = isDesigned && !hasSitemapRefs ? 'stub' : 'partial';
  const platforms = platformsForProduct(c.product);
  const implementations: ScreenMapFrontmatter['implementations'] = {};
  for (const p of platforms) {
    implementations[p] = {
      buildStatus,
      redesignStatus: isDesigned ? 'in-progress' : 'not-started',
      apiStatus,
    };
  }
  const frontmatter: ScreenMapFrontmatter = {
    id: c.id,
    name: c.name,
    product: c.product,
    implementations,
  };
  if (c.sitemapRefs) frontmatter.sitemapRefs = c.sitemapRefs;
  if (c.useCases) frontmatter.useCases = c.useCases;
  if (c.epics) frontmatter.epics = c.epics;
  if (c.frameId) {
    frontmatter.designSources = [{ adapter: 'zip', frame: c.frameId }];
  }
  return frontmatter;
}

function buildBody(c: CandidateScreen): string {
  const today = new Date().toISOString().slice(0, 10);
  return [
    '## Functionality Checklist',
    '',
    '<!-- tag with [w] / [m] / [w,m] / [-] -->',
    '- [ ] [w,m] (none yet)',
    '',
    '## States',
    '',
    '- **Empty**:',
    '- **Loading**:',
    '- **Error**:',
    '',
    '## Notes',
    '',
    '### Broader context',
    '',
    '### Specific (recent)',
    '',
    '## Agent Log',
    '',
    '<!-- newest entries on top -->',
    '',
    `- ${today} — init: created from scan (source: ${c.source})`,
    '',
  ].join('\n');
}

function platformsForProduct(product: Product): Platform[] {
  return product === 'ppt' ? ['ppt-web', 'mobile'] : ['reality-web', 'mobile-native'];
}
