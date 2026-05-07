import { readFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { parseScreenMap, parseScreenMapString } from '../src/parse.js';

const fixturesDir = path.dirname(fileURLToPath(import.meta.url));
const validFixture = path.join(fixturesDir, 'fixtures/building-detail.md');
const invalidFixture = path.join(fixturesDir, 'fixtures/invalid-frontmatter.md');

describe('parseScreenMap', () => {
  it('reads frontmatter and body from a file', async () => {
    const screen = await parseScreenMap(validFixture);
    expect(screen.frontmatter.id).toBe('ppt/building-detail');
    expect(screen.frontmatter.product).toBe('ppt');
    expect(screen.frontmatter.implementations['ppt-web']?.buildStatus).toBe('shipped');
    expect(screen.body).toContain('## Functionality Checklist');
    expect(screen.body).toContain('## Agent Log');
    expect(screen.filePath).toBe(validFixture);
  });

  it('throws a descriptive error on invalid frontmatter', async () => {
    await expect(parseScreenMap(invalidFixture)).rejects.toThrow(/id must match/);
  });
});

describe('parseScreenMapString', () => {
  it('parses an in-memory markdown string', async () => {
    const raw = await readFile(validFixture, 'utf8');
    const screen = parseScreenMapString(raw, '<inline>');
    expect(screen.frontmatter.id).toBe('ppt/building-detail');
    expect(screen.filePath).toBe('<inline>');
  });
});
