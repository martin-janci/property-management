import { readdir, stat } from 'node:fs/promises';
import path from 'node:path';

const PRODUCT_DIRS = ['ppt', 'reality'];
const IGNORED = new Set(['README.md', '_template.md']);

export async function discoverScreenMaps(rootDir: string): Promise<string[]> {
  const out: string[] = [];

  for (const product of PRODUCT_DIRS) {
    const dir = path.join(rootDir, product);
    let entries: string[];
    try {
      entries = await readdir(dir);
    } catch (err) {
      if ((err as NodeJS.ErrnoException).code === 'ENOENT') continue;
      throw err;
    }
    for (const entry of entries) {
      if (IGNORED.has(entry)) continue;
      // .gitkeep is filtered by the .md suffix check below; no separate case.
      if (!entry.endsWith('.md')) continue;
      const full = path.join(dir, entry);
      const s = await stat(full);
      if (s.isFile()) out.push(full);
    }
  }
  return out.sort();
}
