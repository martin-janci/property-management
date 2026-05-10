import type { ScreenMap } from './types.js';

export function renderSiteGraph(screens: ScreenMap[]): string {
  const lines: string[] = ['graph TD'];
  // Nodes — one line per screen. Label includes the original id so consumers
  // searching the rendered output can locate screens by their canonical id.
  for (const s of screens) {
    const id = s.frontmatter.id;
    const name = s.frontmatter.name.replace(/"/g, '&quot;');
    lines.push(`  ${nodeId(id)}["${id}<br/>${name}"]`);
  }
  // Edges — dedupe by sorted pair to avoid double-rendering parent/child both ways.
  // Use the original ids in the edge syntax (Mermaid accepts the sanitized
  // node id followed by the label/comment) so that an edge line still
  // references the human-readable ids when grepped.
  const edges = new Set<string>();
  for (const s of screens) {
    for (const r of s.frontmatter.relatedScreens ?? []) {
      const a = nodeId(s.frontmatter.id);
      const b = nodeId(r.id);
      const key = [a, b].sort().join('--');
      if (edges.has(key)) continue;
      edges.add(key);
      lines.push(`  ${a} -- "${s.frontmatter.id} ${r.rel} ${r.id}" --> ${b}`);
    }
  }
  return lines.join('\n');
}

export function renderEndpointMatrix(screens: ScreenMap[]): string {
  const allEndpoints = new Set<string>();
  for (const s of screens) {
    for (const e of s.frontmatter.endpoints ?? []) allEndpoints.add(e);
  }
  const sortedEndpoints = [...allEndpoints].sort();
  const sortedScreens = [...screens].sort((a, b) =>
    a.frontmatter.id.localeCompare(b.frontmatter.id)
  );
  const header = `| Screen | ${sortedEndpoints.join(' | ')} |`;
  const sep = `|---|${sortedEndpoints.map(() => '---').join('|')}|`;
  const rows: string[] = [];
  for (const s of sortedScreens) {
    const eps = new Set(s.frontmatter.endpoints ?? []);
    const cells = sortedEndpoints.map((e) => (eps.has(e) ? '✓' : ''));
    rows.push(`| ${s.frontmatter.id} | ${cells.join(' | ')} |`);
  }
  return [header, sep, ...rows].join('\n');
}

export function renderStatusDashboard(screens: ScreenMap[]): string {
  const platforms = new Set<string>();
  for (const s of screens) {
    for (const p of Object.keys(s.frontmatter.implementations)) platforms.add(p);
  }
  const blocks: string[] = [];
  for (const platform of [...platforms].sort()) {
    for (const axis of ['build', 'redesign', 'api'] as const) {
      const counts = new Map<string, number>();
      for (const s of screens) {
        const impl = (
          s.frontmatter.implementations as Record<
            string,
            { buildStatus: string; redesignStatus: string; apiStatus: string } | undefined
          >
        )[platform];
        if (!impl) continue;
        const value =
          axis === 'build'
            ? impl.buildStatus
            : axis === 'redesign'
              ? impl.redesignStatus
              : impl.apiStatus;
        counts.set(value, (counts.get(value) ?? 0) + 1);
      }
      if (counts.size === 0) continue;
      const slices = [...counts.entries()].map(([k, v]) => `    "${k}" : ${v}`).join('\n');
      blocks.push(`pie title ${platform} ${axis}\n${slices}`);
    }
  }
  return blocks.join('\n\n');
}

function nodeId(id: string): string {
  // Mermaid node IDs cannot contain `/`; replace with `__`.
  return id.replace(/\//g, '__');
}
