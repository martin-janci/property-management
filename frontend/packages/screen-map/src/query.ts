import { parseFilter } from './filter.js';
import type { ScreenMap } from './types.js';

export type QueryFormat = 'table' | 'json' | 'md';

/**
 * Filter screens by a parseFilter expression. Empty expression returns all.
 */
export function queryScreens(screens: ScreenMap[], expr: string): ScreenMap[] {
  const predicate = parseFilter(expr);
  return screens.filter((s) => predicate(s.frontmatter));
}

export function formatQueryResult(screens: ScreenMap[], format: QueryFormat): string {
  if (format === 'json') {
    return JSON.stringify(
      screens.map((s) => s.frontmatter),
      null,
      2
    );
  }
  if (format === 'md') {
    const lines: string[] = [
      '| id | name | product | platforms | lastReview |',
      '|---|---|---|---|---|',
    ];
    for (const s of screens) {
      const platforms = Object.keys(s.frontmatter.implementations).join(', ');
      lines.push(
        `| ${s.frontmatter.id} | ${s.frontmatter.name} | ${s.frontmatter.product} | ${platforms} | ${s.frontmatter.lastReview ?? '-'} |`
      );
    }
    return lines.join('\n');
  }
  // table (default)
  const headers = ['id', 'name', 'product', 'platforms', 'lastReview'];
  const rows = screens.map((s) => [
    s.frontmatter.id,
    s.frontmatter.name,
    s.frontmatter.product,
    Object.keys(s.frontmatter.implementations).join(','),
    s.frontmatter.lastReview ?? '-',
  ]);
  const colWidths = headers.map((h, i) => Math.max(h.length, ...rows.map((r) => r[i].length)));
  const fmtRow = (cells: string[]): string =>
    cells.map((c, i) => c.padEnd(colWidths[i])).join('  ');
  return [
    fmtRow(headers),
    fmtRow(headers.map((_, i) => '-'.repeat(colWidths[i]))),
    ...rows.map(fmtRow),
  ].join('\n');
}
