#!/usr/bin/env -S npx tsx
import path from 'node:path';
import { Command } from 'commander';
import { buildValidationContext } from './context.js';
import { discoverScreenMaps } from './discover.js';
import { parseScreenMap, ScreenMapParseError } from './parse.js';
import { validateScreenMap } from './validate.js';

const program = new Command();
program
  .name('screen-map')
  .description('CLI for the @ppt/screen-map system')
  .version('0.1.0');

program
  .command('validate')
  .description('validate every screen-map under <root>/docs/screens')
  .option('--root <path>', 'repo root', process.cwd())
  .option('--strict', 'exit non-zero on any error', false)
  .action(async (opts: { root: string; strict: boolean }) => {
    const repoRoot = path.resolve(opts.root);
    const screensDir = path.join(repoRoot, 'docs/screens');
    const files = await discoverScreenMaps(screensDir);
    const ctx = await buildValidationContext({ repoRoot });

    let totalErrors = 0;
    let totalWarnings = 0;
    for (const file of files) {
      try {
        const screen = await parseScreenMap(file);
        const issues = validateScreenMap(screen, ctx);
        if (issues.length === 0) {
          process.stdout.write(`  ok  ${path.relative(repoRoot, file)}\n`);
          continue;
        }
        for (const issue of issues) {
          const tag = issue.severity === 'error' ? 'error' : 'warn ';
          process.stdout.write(
            `  ${tag} ${path.relative(repoRoot, file)} :: ${issue.path} :: ${issue.message}\n`,
          );
          if (issue.severity === 'error') totalErrors += 1;
          else totalWarnings += 1;
        }
      } catch (err) {
        if (err instanceof ScreenMapParseError) {
          for (const issue of err.issues) {
            process.stderr.write(
              `  parse ${path.relative(repoRoot, file)} :: ${issue}\n`,
            );
          }
          totalErrors += 1;
        } else {
          throw err;
        }
      }
    }
    process.stdout.write(
      `Validated ${files.length} screen-maps: ${totalErrors} errors, ${totalWarnings} warnings.\n`,
    );
    if (opts.strict && totalErrors > 0) process.exit(1);
  });

program.parseAsync().catch((err) => {
  process.stderr.write(`Unexpected error: ${(err as Error).message}\n`);
  process.exit(2);
});
