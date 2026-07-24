import { readFile } from 'node:fs/promises';
import path from 'node:path';

export interface KnownContexts {
  knownComponents: Set<string>;
  knownUseCases: Set<string>;
  knownEpics: Set<string>;
}

/**
 * Extract the "known IDs" sets that drive the drift-context optional checks
 * in `scanDrift`. Sources:
 * - `docs/use-cases.md` — regex `\bUC-(\d+(?:\.\d+)?)\b`.
 * - `_bmad-output/epics.md` — the authoritative epic catalog (the BMAD "Epic
 *   Breakdown" doc). Epic ids are read from markdown headings of the form
 *   `#### Epic 10A: …` / `## Epic 2B-Complete: …` via
 *   `^#{1,6}\s+Epic\s+(\d+[A-Z]?)\b`. The optional single letter suffix
 *   preserves the established `10A`/`10B`/`7B` epic convention and is
 *   upper-cased so the extracted id matches the frontmatter refs (`Epic-10A`);
 *   any further `-SSO`/`-Complete` heading qualifier is dropped so the id maps
 *   onto the canonical `Epic-N`. Only heading lines are scanned so prose
 *   cross-references don't get mis-binned as catalog entries.
 *   (The previous filename-based `docs/epics/EPIC-*.md` reader always resolved
 *   to an empty set because that directory does not exist — see #2536.)
 * - `frontend/packages/ui-kit/src/index.ts` — `export { Foo, Bar }` and `export const Baz`
 *   (excludes `export type` lines so type-only exports aren't treated as components).
 *
 * Missing source files yield empty sets — caller decides whether that's a problem.
 */
export async function extractKnownContexts(repoRoot: string): Promise<KnownContexts> {
  return {
    knownUseCases: await extractUseCases(repoRoot),
    knownEpics: await extractEpics(repoRoot),
    knownComponents: await extractComponents(repoRoot),
  };
}

async function extractUseCases(repoRoot: string): Promise<Set<string>> {
  try {
    const content = await readFile(path.join(repoRoot, 'docs/use-cases.md'), 'utf8');
    const out = new Set<string>();
    for (const m of content.matchAll(/\bUC-(\d+(?:\.\d+)?)\b/g)) {
      out.add(`UC-${m[1]}`);
    }
    return out;
  } catch (err) {
    if ((err as NodeJS.ErrnoException).code === 'ENOENT') return new Set();
    throw err;
  }
}

async function extractEpics(repoRoot: string): Promise<Set<string>> {
  try {
    const content = await readFile(path.join(repoRoot, '_bmad-output/epics.md'), 'utf8');
    const out = new Set<string>();
    for (const line of content.split(/\r?\n/)) {
      // Only markdown headings introduce a catalog epic. `\d+[A-Z]?` captures the
      // number plus an optional single-letter phase suffix (`10A`); the `\b`
      // boundary stops before any `-SSO`/`-Complete` qualifier and rejects
      // malformed ids (`Epic 10beta`) instead of mis-binning them.
      const m = line.match(/^#{1,6}\s+Epic\s+(\d+[A-Z]?)\b/i);
      if (m) out.add(`Epic-${m[1].toUpperCase()}`);
    }
    return out;
  } catch (err) {
    if ((err as NodeJS.ErrnoException).code === 'ENOENT') return new Set();
    throw err;
  }
}

async function extractComponents(repoRoot: string): Promise<Set<string>> {
  try {
    const content = await readFile(
      path.join(repoRoot, 'frontend/packages/ui-kit/src/index.ts'),
      'utf8'
    );
    const out = new Set<string>();
    // Scan line-by-line so we can reliably skip `export type { ... }` lines.
    const lines = content.split(/\r?\n/);
    for (const rawLine of lines) {
      const line = rawLine.trim();
      if (line.startsWith('export type ') || line.startsWith('export type{')) continue;
      // `export { Foo, Bar as Baz }` — value-side named re-exports.
      const exportMatch = line.match(/^export\s+\{([^}]+)\}/);
      if (exportMatch) {
        const list = exportMatch[1]
          .split(',')
          .map((s) => s.trim())
          .filter(Boolean);
        for (const item of list) {
          // Skip per-item `type Foo` (mixed type/value re-export form).
          if (/^type\s+/.test(item)) continue;
          const asMatch = item.match(/\sas\s+(\w+)/);
          const name = asMatch ? asMatch[1] : item.replace(/\s.*$/, '');
          if (/^[A-Z]/.test(name)) out.add(name);
        }
        continue;
      }
      // `export const FooBar = ...` — PascalCase const exports (component factories).
      const constMatch = line.match(/^export\s+const\s+([A-Z]\w*)\s*=/);
      if (constMatch) {
        out.add(constMatch[1]);
      }
    }
    return out;
  } catch (err) {
    if ((err as NodeJS.ErrnoException).code === 'ENOENT') return new Set();
    throw err;
  }
}
