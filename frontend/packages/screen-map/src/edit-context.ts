import path from 'node:path';
import { discoverScreenMaps } from './discover.js';
import { parseScreenMap } from './parse.js';
import type { ScreenMap } from './types.js';

export interface LoadScreenContextOptions {
  repoRoot: string;
  /** Run Playwright on a known route to capture a screenshot path. */
  includePlaywright: boolean;
  /** Override default screens dir. */
  screensDir?: string;
}

export async function loadScreenContext(
  id: string,
  options: LoadScreenContextOptions
): Promise<string> {
  const screensDir = options.screensDir ?? path.join(options.repoRoot, 'docs/screens');
  const all = await discoverScreenMaps(screensDir);
  const parsed = await Promise.all(all.map((f) => parseScreenMap(f).catch(() => null)));
  const screens = parsed.filter((s): s is ScreenMap => s !== null);
  const target = screens.find((s) => s.frontmatter.id === id);
  if (!target) {
    throw new Error(`screen "${id}" not found under ${screensDir}`);
  }
  const related = (target.frontmatter.relatedScreens ?? []).map((r) => {
    const found = screens.find((s) => s.frontmatter.id === r.id);
    return { ...r, name: found?.frontmatter.name };
  });
  return formatSummary(target, related);
}

function formatSummary(
  screen: ScreenMap,
  related: { id: string; rel: string; name?: string }[]
): string {
  const fm = screen.frontmatter;
  const lines: string[] = [
    `# ${fm.id}`,
    '',
    `**Name:** ${fm.name}`,
    `**Product:** ${fm.product}`,
    '',
    '## Implementations',
    '',
  ];
  for (const [platform, impl] of Object.entries(fm.implementations)) {
    if (!impl) continue;
    lines.push(
      `- **${platform}**: buildStatus: ${impl.buildStatus}, redesignStatus: ${impl.redesignStatus}, apiStatus: ${impl.apiStatus}${impl.route ? `, route: ${impl.route}` : ''}${impl.screen ? `, screen: ${impl.screen}` : ''}${impl.component ? `, component: ${impl.component}` : ''}`
    );
  }
  lines.push('');
  if (fm.endpoints?.length) {
    lines.push('## Endpoints');
    lines.push('');
    for (const ep of fm.endpoints) lines.push(`- ${ep}`);
    lines.push('');
  }
  if (related.length > 0) {
    lines.push('## Related Screens');
    lines.push('');
    for (const r of related) {
      lines.push(`- (${r.rel}) ${r.id}${r.name ? ` — ${r.name}` : ''}`);
    }
    lines.push('');
  }
  if (fm.useCases?.length) {
    lines.push(`**Use Cases:** ${fm.useCases.join(', ')}`);
  }
  if (fm.epics?.length) {
    lines.push(`**Epics:** ${fm.epics.join(', ')}`);
  }
  lines.push('');
  // Recent agent log: pull the last 5 list items from the body's "## Agent Log" section.
  const agentLog = extractAgentLog(screen.body);
  if (agentLog.length > 0) {
    lines.push('## Recent Agent Log');
    lines.push('');
    for (const entry of agentLog.slice(0, 5)) lines.push(entry);
    lines.push('');
  }
  return lines.join('\n');
}

function extractAgentLog(body: string): string[] {
  const idx = body.indexOf('## Agent Log');
  if (idx < 0) return [];
  const after = body.slice(idx);
  return after.split(/\r?\n/).filter((l) => l.startsWith('- '));
}
