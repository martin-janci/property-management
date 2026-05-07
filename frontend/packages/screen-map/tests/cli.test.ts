import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { mkdir, mkdtemp, rm, writeFile } from 'node:fs/promises';
import os from 'node:os';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';

const execFileP = promisify(execFile);
const here = path.dirname(fileURLToPath(import.meta.url));
const cliPath = path.join(here, '..', 'src', 'cli.ts');

let tmpRepo: string;

beforeEach(async () => {
  tmpRepo = await mkdtemp(path.join(os.tmpdir(), 'screen-map-cli-'));
  // Minimal fake repo: docs/screens/ppt/<one valid screen using real sitemap ids>.
  await mkdir(path.join(tmpRepo, 'docs/screens/ppt'), { recursive: true });
});

afterEach(async () => {
  await rm(tmpRepo, { recursive: true, force: true });
});

async function run(args: string[]): Promise<{ stdout: string; stderr: string; code: number }> {
  try {
    const { stdout, stderr } = await execFileP('npx', ['tsx', cliPath, ...args], {
      env: { ...process.env, NO_COLOR: '1' },
    });
    return { stdout, stderr, code: 0 };
  } catch (err: unknown) {
    const e = err as { stdout?: string; stderr?: string; code?: number };
    return { stdout: e.stdout ?? '', stderr: e.stderr ?? '', code: e.code ?? 1 };
  }
}

describe('cli', () => {
  it('exits 0 when no screens exist (empty repo)', async () => {
    const result = await run(['validate', '--root', tmpRepo]);
    expect(result.code).toBe(0);
    expect(result.stdout).toMatch(/0 screen-maps/i);
  });

  it('exits 1 in --strict on a parse error', async () => {
    await writeFile(
      path.join(tmpRepo, 'docs/screens/ppt/bad.md'),
      '---\nid: bad\nname: Bad\nproduct: ppt\nimplementations: {}\n---\n',
    );
    const result = await run(['validate', '--root', tmpRepo, '--strict']);
    expect(result.code).toBe(1);
    expect(result.stderr + result.stdout).toMatch(/id must match/);
  });
});
