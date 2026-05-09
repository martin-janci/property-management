#!/usr/bin/env -S npx tsx
import { createRequire } from 'node:module';
import path from 'node:path';
import { Command } from 'commander';
import { buildValidationContext } from './context.js';
import { discoverScreenMaps } from './discover.js';
import { parseScreenMap, ScreenMapParseError } from './parse.js';
import type { ScreenMap } from './types.js';
import { validateScreenMap } from './validate.js';

// Read version from package.json so `screen-map --version` always reflects
// the actual published version (the repo treats VERSION/package.json as the
// single source of truth).
const requireFromHere = createRequire(import.meta.url);
const pkg = requireFromHere('../package.json') as { version: string };

const program = new Command();
program.name('screen-map').description('CLI for the @ppt/screen-map system').version(pkg.version);

program
  .command('validate')
  .description('validate every screen-map under <root>/docs/screens')
  .option('--root <path>', 'repo root', process.cwd())
  .option('--strict', 'exit non-zero on any error', false)
  .action(async (opts: { root: string; strict: boolean }) => {
    const repoRoot = path.resolve(opts.root);
    const screensDir = path.join(repoRoot, 'docs/screens');
    const files = await discoverScreenMaps(screensDir);

    // Parse every file once. Successful parses are kept for both validation
    // below and reuse via `buildValidationContext({ parsedScreens })`, so we
    // don't pay the IO+parse cost twice per CLI run.
    type ParseEntry =
      | { kind: 'ok'; file: string; screen: ScreenMap }
      | { kind: 'parse-error'; file: string; issues: string[] }
      | { kind: 'unexpected'; file: string; err: unknown };
    const entries: ParseEntry[] = [];
    for (const file of files) {
      try {
        const screen = await parseScreenMap(file);
        entries.push({ kind: 'ok', file, screen });
      } catch (err) {
        if (err instanceof ScreenMapParseError) {
          entries.push({ kind: 'parse-error', file, issues: err.issues });
        } else {
          entries.push({ kind: 'unexpected', file, err });
        }
      }
    }
    const parsedScreens = entries.flatMap((e) => (e.kind === 'ok' ? [e.screen] : []));
    const ctx = await buildValidationContext({ repoRoot, parsedScreens });

    let totalErrors = 0;
    let totalWarnings = 0;
    for (const entry of entries) {
      if (entry.kind === 'parse-error') {
        for (const issue of entry.issues) {
          process.stderr.write(`  parse ${path.relative(repoRoot, entry.file)} :: ${issue}\n`);
        }
        totalErrors += 1;
        continue;
      }
      if (entry.kind === 'unexpected') {
        throw entry.err;
      }
      const issues = validateScreenMap(entry.screen, ctx);
      if (issues.length === 0) {
        process.stdout.write(`  ok  ${path.relative(repoRoot, entry.file)}\n`);
        continue;
      }
      for (const issue of issues) {
        const tag = issue.severity === 'error' ? 'error' : 'warn ';
        process.stdout.write(
          `  ${tag} ${path.relative(repoRoot, entry.file)} :: ${issue.path} :: ${issue.message}\n`
        );
        if (issue.severity === 'error') totalErrors += 1;
        else totalWarnings += 1;
      }
    }
    process.stdout.write(
      `Validated ${files.length} screen-maps: ${totalErrors} errors, ${totalWarnings} warnings.\n`
    );
    if (opts.strict && totalErrors > 0) process.exit(1);
  });

program.parseAsync().catch((err) => {
  process.stderr.write(`Unexpected error: ${(err as Error).message}\n`);
  process.exit(2);
});
