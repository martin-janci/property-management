import type { ScreenMap } from './types.js';

export type QueryFormat = 'table' | 'json' | 'md';

/**
 * Filter screens by a parseFilter expression. Empty expression returns all.
 * The matching logic mirrors `parseFilter` in cli.ts (kept duplicated here
 * to avoid a circular import; consolidating into a shared helper is a follow-up).
 */
export function queryScreens(screens: ScreenMap[], expr: string): ScreenMap[] {
  if (!expr.trim()) return [...screens];
  const terms = expr.split(',').map((t) => {
    const colonIdx = t.indexOf(':');
    return {
      key: (colonIdx >= 0 ? t.slice(0, colonIdx) : t).trim(),
      value: (colonIdx >= 0 ? t.slice(colonIdx + 1) : '').trim(),
    };
  });
  return screens.filter((s) => {
    return terms.every(({ key, value }) => {
      const path = key.split('.');
      let cursor: unknown = s.frontmatter;
      for (const seg of path) {
        if (cursor && typeof cursor === 'object' && seg in cursor) {
          cursor = (cursor as Record<string, unknown>)[seg];
        } else {
          return false;
        }
      }
      return String(cursor) === value;
    });
  });
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
