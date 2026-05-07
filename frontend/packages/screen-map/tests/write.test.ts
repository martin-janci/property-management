import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import { describe, expect, it } from 'vitest';
import { parseScreenMap, parseScreenMapString } from '../src/parse.js';
import { writeScreenMapString } from '../src/write.js';

const fixturesDir = path.dirname(fileURLToPath(import.meta.url));
const validFixture = path.join(fixturesDir, 'fixtures/building-detail.md');

describe('writeScreenMapString', () => {
  it('preserves the markdown body verbatim across a parse/write round-trip', async () => {
    const original = await readFile(validFixture, 'utf8');
    const parsed = await parseScreenMap(validFixture);
    const written = writeScreenMapString(parsed);
    const reparsed = parseScreenMapString(written, '<inline>');
    expect(reparsed.body).toBe(parsed.body);
  });

  it('reflects mutated frontmatter values', async () => {
    const parsed = await parseScreenMap(validFixture);
    parsed.frontmatter.implementations['mobile']!.redesignStatus = 'applied';
    parsed.frontmatter.lastReview = '2026-05-08';
    const written = writeScreenMapString(parsed);
    expect(written).toMatch(/redesignStatus:\s*['"]?applied['"]?/);
    expect(written).toMatch(/lastReview:\s*['"]?2026-05-08['"]?/);
  });
});
