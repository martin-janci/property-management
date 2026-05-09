#!/usr/bin/env -S npx tsx
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { Command } from 'commander';
import { buildValidationContext } from './context.js';
import { createDesignSource } from './design-source/index.js';
import { discoverScreenMaps } from './discover.js';
import { loadScreenContext } from './edit-context.js';
import { parseFilter } from './filter.js';
import { type GroupingDecision, mergeCandidates } from './grouping.js';
import { bulkWriteScreenMaps } from './init-write.js';
import { parseScreenMap, ScreenMapParseError } from './parse.js';
import { startReviewServer } from './review-server/start.js';
import { scanCandidates } from './scan.js';
import { validateScreenMap } from './validate.js';

// Re-export parseFilter to preserve backward compatibility for callers
// (including existing unit tests) that import it from cli.ts.
export { parseFilter };

const program = new Command();
program.name('screen-map').description('CLI for the @ppt/screen-map system').version('0.1.0');

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
            `  ${tag} ${path.relative(repoRoot, file)} :: ${issue.path} :: ${issue.message}\n`
          );
          if (issue.severity === 'error') totalErrors += 1;
          else totalWarnings += 1;
        }
      } catch (err) {
        if (err instanceof ScreenMapParseError) {
          for (const issue of err.issues) {
            process.stderr.write(`  parse ${path.relative(repoRoot, file)} :: ${issue}\n`);
          }
          totalErrors += 1;
        } else {
          throw err;
        }
      }
    }
    process.stdout.write(
      `Validated ${files.length} screen-maps: ${totalErrors} errors, ${totalWarnings} warnings.\n`
    );
    if (opts.strict && totalErrors > 0) process.exit(1);
  });

program
  .command('init')
  .description('scan + interactive grouping + bulk-write screen-maps for a product')
  .requiredOption('--product <name>', 'ppt | reality')
  .option('--root <path>', 'repo root', process.cwd())
  .option('--designs <zipPath>', 'DesignSource ZIP file')
  .option('--add <names...>', 'user-added candidate names')
  .option('--decisions <jsonPath>', 'JSON file with grouping decisions (skip interactive prompt)')
  .option('--force', 'overwrite existing screen-maps', false)
  .action(
    async (opts: {
      product: 'ppt' | 'reality';
      root: string;
      designs?: string;
      add?: string[];
      decisions?: string;
      force: boolean;
    }) => {
      const repoRoot = path.resolve(opts.root);
      const designSource = opts.designs
        ? await createDesignSource({ adapter: 'zip', file: opts.designs }, { repoRoot })
        : undefined;
      const candidates = await scanCandidates({
        product: opts.product,
        repoRoot,
        sources: {
          sitemap: true,
          useCases: true,
          epics: true,
          designSource,
          userAdd: opts.add ?? [],
        },
      });
      let decisions: GroupingDecision[] = [];
      if (opts.decisions) {
        const fs = await import('node:fs/promises');
        decisions = JSON.parse(await fs.readFile(opts.decisions, 'utf8'));
      }
      const concepts = mergeCandidates(candidates, decisions);
      const screensDir = path.join(repoRoot, 'docs/screens');
      const result = await bulkWriteScreenMaps(concepts, screensDir, { force: opts.force });
      process.stdout.write(`Wrote ${result.written.length} screen-maps under ${screensDir}\n`);
      for (const file of result.written)
        process.stdout.write(`  + ${path.relative(repoRoot, file)}\n`);
      if (result.skipped.length > 0) {
        process.stdout.write(
          `Skipped ${result.skipped.length} user-edited screen-maps (use a unique --add or rename to add a fresh entry):\n`
        );
        for (const file of result.skipped)
          process.stdout.write(`  ~ ${path.relative(repoRoot, file)}\n`);
      }
    }
  );

program
  .command('edit <id>')
  .description('print a markdown context summary for one screen')
  .option('--root <path>', 'repo root', process.cwd())
  .option('--playwright', 'capture a screenshot via Playwright', false)
  .action(async (id: string, opts: { root: string; playwright: boolean }) => {
    const repoRoot = path.resolve(opts.root);
    const summary = await loadScreenContext(id, {
      repoRoot,
      includePlaywright: opts.playwright,
    });
    process.stdout.write(`${summary}\n`);
  });

program
  .command('update')
  .description('detect drift between code and screen-maps; report issues')
  .option('--root <path>', 'repo root', process.cwd())
  .option('--strict', 'exit non-zero on any drift', false)
  .action(async (opts: { root: string; strict: boolean }) => {
    const repoRoot = path.resolve(opts.root);
    const screensDir = path.join(repoRoot, 'docs/screens');
    const files = await discoverScreenMaps(screensDir);
    const screens = await Promise.all(files.map((f) => parseScreenMap(f)));
    const ctx = await buildValidationContext({ repoRoot });
    const { scanDrift } = await import('./scan-drift.js');
    // Phase 3a: only sitemap/endpoint/orphan drift wired here.
    // UC/component/epic drift requires knownUseCases/knownComponents/knownEpics sets
    // — those will be wired in Phase 3b alongside the bootstrap runs.
    const issues = scanDrift({ screens, context: ctx });
    if (issues.length === 0) {
      process.stdout.write('No drift detected.\n');
      return;
    }
    process.stdout.write(
      `Drift detected (${issues.length} issue${issues.length === 1 ? '' : 's'}):\n`
    );
    for (const issue of issues) {
      process.stdout.write(`  ${formatDrift(issue)}\n`);
    }
    if (opts.strict) process.exit(1);
  });

program
  .command('render')
  .description('generate mermaid diagrams (site graph, endpoint matrix, status dashboard)')
  .option('--root <path>', 'repo root', process.cwd())
  .option('--scope <name>', 'product | all', 'all')
  .option('--out <path>', 'output dir', 'docs/screens/_diagrams')
  .action(async (opts: { root: string; scope: string; out: string }) => {
    const repoRoot = path.resolve(opts.root);
    const screensDir = path.join(repoRoot, 'docs/screens');
    const files = await discoverScreenMaps(screensDir);
    let screens = await Promise.all(files.map((f) => parseScreenMap(f)));
    if (opts.scope !== 'all') {
      screens = screens.filter((s) => s.frontmatter.product === opts.scope);
    }
    const { renderSiteGraph, renderEndpointMatrix, renderStatusDashboard } = await import(
      './render.js'
    );
    const fs = await import('node:fs/promises');
    const outDir = path.resolve(repoRoot, opts.out);
    await fs.mkdir(outDir, { recursive: true });
    const scopeName = opts.scope;
    const writes: Array<[string, string]> = [
      [`${scopeName}-site-graph.mmd`, renderSiteGraph(screens)],
      [`${scopeName}-endpoint-matrix.md`, renderEndpointMatrix(screens)],
      [`${scopeName}-status.mmd`, renderStatusDashboard(screens)],
    ];
    for (const [filename, content] of writes) {
      const filepath = path.join(outDir, filename);
      await fs.writeFile(filepath, `${content}\n`, 'utf8');
      process.stdout.write(`  wrote ${path.relative(repoRoot, filepath)}\n`);
    }
  });

program
  .command('query [expr]')
  .description(
    'query screen-maps by frontmatter filter (e.g. "product:ppt,redesignStatus:in-progress")'
  )
  .option('--root <path>', 'repo root', process.cwd())
  .option('--format <fmt>', 'table | json | md', 'table')
  .action(async (expr: string | undefined, opts: { root: string; format: string }) => {
    const repoRoot = path.resolve(opts.root);
    const screensDir = path.join(repoRoot, 'docs/screens');
    const files = await discoverScreenMaps(screensDir);
    const screens = await Promise.all(files.map((f) => parseScreenMap(f)));
    const { queryScreens, formatQueryResult } = await import('./query.js');
    const filtered = queryScreens(screens, expr ?? '');
    const fmt = (opts.format === 'json' || opts.format === 'md' ? opts.format : 'table') as
      | 'table'
      | 'json'
      | 'md';
    process.stdout.write(`${formatQueryResult(filtered, fmt)}\n`);
    process.stdout.write(
      `\n${filtered.length} screen-map${filtered.length === 1 ? '' : 's'} matched.\n`
    );
  });

program
  .command('review')
  .description('spawn the Visual Review server and open the browser')
  .option('--root <path>', 'repo root', process.cwd())
  .option('--product <name>', 'ppt | reality')
  .option('--preview <mode>', 'local | staging | design', 'local')
  .option('--filter <expr>', 'frontmatter filter, e.g. redesignStatus:in-progress')
  .action(
    async (opts: {
      root: string;
      product?: 'ppt' | 'reality';
      preview: 'local' | 'staging' | 'design';
      filter?: string;
    }) => {
      const repoRoot = path.resolve(opts.root);
      const result = await startReviewServer({
        repoRoot,
        product: opts.product,
        preview: opts.preview,
        filter: opts.filter ? parseFilter(opts.filter) : undefined,
      });
      process.stdout.write(`Review server running at ${result.url}\n`);
      process.stdout.write('Press Ctrl-C to stop.\n');
      await result.finished;
      process.exit(0);
    }
  );

function formatDrift(issue: import('./scan-drift.js').DriftIssue): string {
  switch (issue.kind) {
    case 'unmapped-sitemap':
      return `unmapped-sitemap :: ${issue.sitemapId} (no screen-map references it)`;
    case 'unknown-endpoint':
      return `unknown-endpoint :: ${issue.screenId} :: ${issue.endpointId}`;
    case 'unknown-component':
      return `unknown-component :: ${issue.screenId} :: ${issue.component}`;
    case 'unknown-use-case':
      return `unknown-use-case :: ${issue.screenId} :: ${issue.useCaseId}`;
    case 'unknown-epic':
      return `unknown-epic :: ${issue.screenId} :: ${issue.epicId}`;
    case 'orphan-screen':
      return `orphan-screen :: ${issue.screenId} :: sitemap "${issue.sitemapId}" not found`;
  }
}

// Only run the CLI when this module is the entry point — guards against
// `import { parseFilter } from './cli.js'` (used in unit tests) triggering
// the commander parse loop and exiting the test process.
const isMain = (() => {
  try {
    return process.argv[1] === fileURLToPath(import.meta.url);
  } catch {
    return false;
  }
})();
if (isMain) {
  program.parseAsync().catch((err) => {
    process.stderr.write(`Unexpected error: ${(err as Error).message}\n`);
    process.exit(2);
  });
}
