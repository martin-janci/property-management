/// <reference types="vitest/globals" />
/**
 * Unwired-feature guard (PAP-55 / WS-C — Stable Beta T2).
 *
 * Enforces the board decision recorded on PAP-28: the built-but-unwired
 * ppt-web features stay OUT of the routing / nav surface (no dead-end links to
 * half-built screens for paying customers) while their code is RETAINED for a
 * later, deliberate wiring phase.
 *
 * The guard reads the real routing-surface source files and asserts that none
 * of them import or reference any unwired feature directory. If a future change
 * wires one of these features back in, this test fails until the slug is removed
 * from `UNWIRED_FEATURES` (which is the explicit, reviewed act of un-hiding it).
 */
import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { UNWIRED_FEATURES } from '../features/unwired-features';

// Vitest runs with cwd set to this package root (apps/ppt-web).
const srcDir = `${join(process.cwd(), 'src')}/`;

/** Read a source file under src/, returning '' if it does not exist. */
function readSrc(relPath: string): string {
  try {
    return readFileSync(`${srcDir}${relPath}`, 'utf8');
  } catch {
    return '';
  }
}

/** Every file that contributes to the router / navigation surface. */
function routingSurfaceSources(): { file: string; content: string }[] {
  const files: string[] = [
    'App.tsx',
    'routes/AppRoutes.tsx',
    'routes/lazyRoutes.tsx',
    'features/command-palette/hooks/useNavigationCommands.tsx',
  ];
  // All per-feature route-group fragments.
  const groupsDir = `${srcDir}routes/groups`;
  for (const entry of readdirSync(groupsDir)) {
    if (entry.endsWith('.tsx') || entry.endsWith('.ts')) {
      files.push(`routes/groups/${entry}`);
    }
  }
  return files.map((file) => ({ file, content: readSrc(file) }));
}

describe('Unwired feature registry (PAP-55)', () => {
  it('lists 10 features', () => {
    expect(UNWIRED_FEATURES.length).toBe(10);
  });

  it('retains the code for every unwired feature (dir exists)', () => {
    const featureDirs = new Set(
      readdirSync(`${srcDir}features`, { withFileTypes: true })
        .filter((d) => d.isDirectory())
        .map((d) => d.name)
    );
    for (const slug of UNWIRED_FEATURES) {
      expect(featureDirs.has(slug), `features/${slug} should be retained (not deleted)`).toBe(true);
    }
  });
});

describe('Unwired features stay out of the routing/nav surface (PAP-55)', () => {
  const sources = routingSurfaceSources();

  it.each(UNWIRED_FEATURES.map((slug) => [slug]))(
    'feature "%s" is not wired into any router/nav file',
    (slug) => {
      // Match an import/dynamic-import that ends at the feature dir, e.g.
      //   import('../features/insurance')  or  from '../features/insurance'
      // regardless of quote style. A trailing quote/backtick after the slug
      // means the import targets the dir itself (not a deeper, unrelated path).
      const importRe = new RegExp(`features/${slug}["'\`]`);
      const offenders = sources
        .filter(({ content }) => importRe.test(content))
        .map(({ file }) => file);
      expect(
        offenders,
        `Unwired feature "${slug}" is referenced by routing surface file(s): ${offenders.join(
          ', '
        )}. Per PAP-55 it must stay hidden from nav; if you intentionally wired it, remove "${slug}" from UNWIRED_FEATURES.`
      ).toEqual([]);
    }
  );
});
