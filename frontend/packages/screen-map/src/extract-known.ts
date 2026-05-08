import { readFile, readdir } from 'node:fs/promises';
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
 * - `docs/epics/EPIC-NNN-*.md` — regex `^EPIC-(\d+)` on filenames.
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
    const entries = await readdir(path.join(repoRoot, 'docs/epics'));
    const out = new Set<string>();
    for (const entry of entries) {
      const m = entry.match(/^EPIC-(\d+)/i);
      if (m) out.add(`Epic-${m[1]}`);
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
