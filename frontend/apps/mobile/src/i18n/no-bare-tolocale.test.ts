/**
 * Guard against bare `toLocale*` date rendering (GitHub #2759, follow-up to
 * #2755 / #2752 / #2282).
 *
 * Screens must format dates/times through the shared locale-aware helpers in
 * `src/i18n/format.ts` (`formatDate` / `formatTime` / `formatDateTime`), which
 * derive the formatting locale from the app's *active* i18next language. A bare
 * `new Date(x).toLocaleDateString()` (or `toLocaleTimeString` / `toLocaleString`)
 * renders in the **device** locale instead — so a Slovak/Czech/German/Polish/
 * Hungarian user who picked their language in-app still sees dates formatted for
 * whatever locale the phone happens to be set to. This drift has been fixed
 * screen-by-screen more than once; this test locks the contract tree-wide so it
 * cannot silently come back.
 *
 * The only sanctioned home for `toLocale*` is `src/i18n/format.ts` itself (the
 * helper implementation). Test files are exempt — they legitimately compute
 * expected values with an explicit locale.
 *
 * Reads the source tree directly (no build step) so the guard tracks the actual
 * code, not a snapshot.
 */

// This suite runs under Node (jest), but the React Native `tsconfig` doesn't
// pull in `@types/node`, so declare the tiny slice of the Node API we use here
// rather than adding a project-wide dependency just for one test. `require`
// resolves at runtime because babel-jest compiles this module to CommonJS.
declare const __dirname: string;
declare function require(id: 'node:fs'): {
  readFileSync(path: string, encoding: 'utf8'): string;
  readdirSync(
    path: string,
    opts: { withFileTypes: true }
  ): Array<{
    name: string;
    isDirectory(): boolean;
    isFile(): boolean;
  }>;
};
declare function require(id: 'node:path'): {
  join(...parts: string[]): string;
  relative(from: string, to: string): string;
};

const { readFileSync, readdirSync } = require('node:fs');
const { join, relative } = require('node:path');

// i18n/ → src
const SRC_ROOT = join(__dirname, '..');

// The one sanctioned home for `toLocale*`: the helper implementation.
const ALLOWLIST = new Set<string>(['i18n/format.ts']);

// `.toLocaleDateString(` / `.toLocaleTimeString(` / `.toLocaleString(`.
const BARE_TOLOCALE = /\.toLocale(?:Date|Time)?String\s*\(/;

function collectSourceFiles(dir: string, acc: string[] = []): string[] {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const full = join(dir, entry.name);
    if (entry.isDirectory()) {
      if (entry.name === 'node_modules' || entry.name === '__snapshots__') continue;
      collectSourceFiles(full, acc);
    } else if (entry.isFile()) {
      if (!/\.(ts|tsx)$/.test(entry.name)) continue;
      // Exempt every test / spec file — those may build expected values with an
      // explicit locale.
      if (/\.(test|spec)\.(ts|tsx)$/.test(entry.name)) continue;
      const rel = relative(SRC_ROOT, full).split('\\').join('/');
      if (ALLOWLIST.has(rel)) continue;
      acc.push(full);
    }
  }
  return acc;
}

describe('no bare toLocale* date formatting', () => {
  it('every screen formats dates through src/i18n/format.ts helpers', () => {
    const offenders: string[] = [];

    for (const file of collectSourceFiles(SRC_ROOT)) {
      const lines = readFileSync(file, 'utf8').split('\n');
      lines.forEach((line, i) => {
        if (BARE_TOLOCALE.test(line)) {
          const rel = relative(SRC_ROOT, file).split('\\').join('/');
          offenders.push(`${rel}:${i + 1}  ${line.trim()}`);
        }
      });
    }

    if (offenders.length > 0) {
      throw new Error(
        [
          'Bare `toLocale*` date formatting found. Use the locale-aware helpers',
          "from 'src/i18n/format.ts' (formatDate / formatTime / formatDateTime)",
          'so dates match the active in-app language instead of the device locale:',
          '',
          ...offenders,
        ].join('\n')
      );
    }

    expect(offenders).toEqual([]);
  });
});
