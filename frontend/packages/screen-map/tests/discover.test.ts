import { mkdir, rm, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { discoverScreenMaps } from '../src/discover.js';

let tmpRoot: string;

beforeEach(async () => {
  tmpRoot = await mkdtemp(path.join(os.tmpdir(), 'screen-map-discover-'));
});

afterEach(async () => {
  await rm(tmpRoot, { recursive: true, force: true });
});

async function mkdtemp(prefix: string): Promise<string> {
  const { mkdtemp } = await import('node:fs/promises');
  return mkdtemp(prefix);
}

describe('discoverScreenMaps', () => {
  it('finds .md files under <root>/<product>/', async () => {
    await mkdir(path.join(tmpRoot, 'ppt'), { recursive: true });
    await mkdir(path.join(tmpRoot, 'reality'), { recursive: true });
    await writeFile(path.join(tmpRoot, 'ppt', 'a.md'), '---\nid: ppt/a\n---\n');
    await writeFile(path.join(tmpRoot, 'ppt', 'b.md'), '---\nid: ppt/b\n---\n');
    await writeFile(path.join(tmpRoot, 'reality', 'c.md'), '---\nid: reality/c\n---\n');

    const found = await discoverScreenMaps(tmpRoot);
    const ids = found.map((f) => path.relative(tmpRoot, f)).sort();
    expect(ids).toEqual(['ppt/a.md', 'ppt/b.md', 'reality/c.md']);
  });

  it('ignores README.md, _template.md, and .gitkeep', async () => {
    await mkdir(path.join(tmpRoot, 'ppt'), { recursive: true });
    await writeFile(path.join(tmpRoot, 'README.md'), '');
    await writeFile(path.join(tmpRoot, '_template.md'), '');
    await writeFile(path.join(tmpRoot, 'ppt', '.gitkeep'), '');
    await writeFile(path.join(tmpRoot, 'ppt', 'a.md'), '---\nid: ppt/a\n---\n');
    const found = await discoverScreenMaps(tmpRoot);
    expect(found).toHaveLength(1);
    expect(found[0].endsWith('ppt/a.md')).toBe(true);
  });

  it('returns [] when the root directory does not exist', async () => {
    const found = await discoverScreenMaps(path.join(tmpRoot, 'missing'));
    expect(found).toEqual([]);
  });

  it.skipIf(typeof process.getuid === 'function' && process.getuid() === 0)(
    'propagates non-ENOENT readdir errors instead of silently skipping',
    async () => {
      // chmod 000 the product dir to provoke EACCES on readdir.
      const dir = path.join(tmpRoot, 'ppt');
      await mkdir(dir, { recursive: true });
      const { chmod } = await import('node:fs/promises');
      await chmod(dir, 0o000);
      try {
        await expect(discoverScreenMaps(tmpRoot)).rejects.toThrow();
      } finally {
        await chmod(dir, 0o755);
      }
    }
  );
});
